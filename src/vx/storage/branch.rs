use crate::context::Context;
use crate::core::branch::Branch;
use crate::storage::BRANCHES_FILE_NAME;
use sled::Db;
use thiserror::Error;
use xxhash_rust::xxh3::xxh3_64;

/// Represents errors that can occur while handling branches.
#[derive(Error, Debug)]
pub enum BranchError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sled::Error),

    #[error("Serialization/Deserialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("Branch not found")]
    NotFound,

    #[error("Branch with name '{0}' already exists")]
    BranchExists(String),

    #[error("Invalid branch name: {0}")]
    InvalidName(String),

    #[error("Invalid parent branch: {0}")]
    InvalidParent(String),

    #[error("{0}")]
    Other(String),
}

/// Opens branch store.
fn open(context: &Context) -> Result<Db, BranchError> {
    let db = sled::open(context.workspace_path.join(BRANCHES_FILE_NAME))?;
    Ok(db)
}

/// Creates a new branch.
pub fn new(
    context: &Context,
    name: String,
    headseq: u64,
    parent: u64,
    parentseq: u64,
) -> Result<Branch, BranchError> {
    let db = open(context)?;

    // Compute branch id as a 64-bit hash of the branch name using xxHash.
    let id = xxh3_64(name.as_bytes());
    let branch = Branch {
        id,
        name: name.clone(),
        headseq,
        parent,
        parentseq,
        ver: 0,
    };
    let key = branch.id.to_be_bytes().to_vec();
    let value = bincode::serialize(&branch)?;

    // Attempt to atomically insert the branch only if no record with the same id exists.
    let result = db.compare_and_swap(key.clone(), None as Option<&[u8]>, Some(value))?;
    match result {
        Ok(()) => {
            db.flush()?;
            Ok(branch)
        }
        Err(e) => {
            // A record with the same id already exists.
            match e.current {
                Some(existing_bytes) => {
                    let existing_branch: Branch = bincode::deserialize(&existing_bytes)?;
                    if existing_branch.name == name {
                        Err(BranchError::BranchExists(name))
                    } else {
                        // TODO: Even if it is super rare, handle hash collisions properly.
                        Err(BranchError::DatabaseError(sled::Error::Unsupported(
                            format!(
                                "Hash collision! Branch with id {} already exists under different name '{}'",
                                branch.id, existing_branch.name
                            ),
                        )))
                    }
                }
                None => Err(BranchError::DatabaseError(sled::Error::Unsupported(
                    format!(
                        "Branch with id {} already exists but existing record is unavailable",
                        branch.id
                    ),
                ))),
            }
        }
    }
}

/// Gets branch by ID.
pub fn get(context: &Context, id: u64) -> Result<Branch, BranchError> {
    let key = id.to_be_bytes();
    let db = open(context)?;
    match db.get(key)? {
        Some(ivec) => {
            let branch: Branch = bincode::deserialize(&ivec)?;
            Ok(branch)
        }
        None => Err(BranchError::NotFound),
    }
}

/// Gets branch by name.
pub fn get_by_name(context: &Context, name: &str) -> Result<Branch, BranchError> {
    let id = xxh3_64(name.as_bytes());
    // TODO: handle hash collisions.
    get(context, id)
}

/// Lists all branches.
pub fn list(context: &Context) -> Result<Vec<Branch>, BranchError> {
    let db = open(context)?;
    let mut branches = Vec::new();
    for item in db.iter() {
        let (_key, value) = item?;
        let branch: Branch = bincode::deserialize(&value)?;
        branches.push(branch);
    }
    Ok(branches)
}

/// Updates the head sequence number of a branch.
pub fn update_headseq(
    context: &Context,
    branch_id: u64,
    new_headseq: u64,
    new_ver: u32,
) -> Result<Branch, BranchError> {
    let db = open(context)?;
    let key = branch_id.to_be_bytes();

    // Create a mutable reference to store any error that happens in the closure
    let mut closure_error: Option<BranchError> = None;
    let mut closure_branch: Option<Branch> = None;

    // update_and_fetch returns binary, so we save the actual error and branch in the closure
    db.update_and_fetch(key, |current| {
        match current {
            Some(current_bytes) => {
                // Try to deserialize the branch
                match bincode::deserialize::<Branch>(current_bytes) {
                    Ok(mut branch) => {
                        branch.headseq = new_headseq; // Try to serialize the updated branch
                        branch.ver = new_ver;
                        match bincode::serialize(&branch) {
                            Ok(serialized) => {
                                closure_branch = Some(branch);
                                Some(serialized)
                            }
                            Err(err) => {
                                // Store the serialization error
                                closure_error = Some(BranchError::SerializationError(err));
                                None
                            }
                        }
                    }
                    Err(err) => {
                        // Store the deserialization error
                        closure_error = Some(BranchError::SerializationError(err));
                        None
                    }
                }
            }
            None => {
                // Branch not found
                closure_error = Some(BranchError::NotFound);
                None
            }
        }
    })?;

    // Check if an error occurred in the closure
    if let Some(err) = closure_error {
        return Err(err);
    }

    db.flush()?;

    // If we got here, closure_branch should be Some(_)
    Ok(closure_branch.unwrap())
}
