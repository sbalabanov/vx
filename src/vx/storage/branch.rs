use crate::context::Context;
use crate::core::branch::Branch;
use crate::storage::DATABASE_FILE_NAME;
use sled::Tree;
use thiserror::Error;
use xxhash_rust::xxh3::xxh3_64; // Use xxHash (xxh3_64) for computing branch ID

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
}

/// Structure to hold the branch tree.
struct BranchStore {
    branch_tree: Tree,
}

/// Opens branch store.
fn open(context: &Context) -> Result<BranchStore, BranchError> {
    let db = sled::open(context.workspace_path.join(DATABASE_FILE_NAME))?;
    let branch_tree = db.open_tree("branches")?;
    Ok(BranchStore { branch_tree })
}

/// Creates a new branch.
pub fn new(
    context: &Context,
    name: String,
    headseq: u64,
    parent: u64,
    parentseq: u64,
) -> Result<Branch, BranchError> {
    let store = open(context)?;

    // Compute branch id as a 64-bit hash of the branch name using xxHash.
    let id = xxh3_64(name.as_bytes());
    let branch = Branch {
        id,
        name: name.clone(),
        headseq,
        parent,
        parentseq,
    };
    let key = branch.id.to_be_bytes().to_vec();
    let value = bincode::serialize(&branch)?;

    // Attempt to atomically insert the branch only if no record with the same id exists.
    let result =
        store
            .branch_tree
            .compare_and_swap(key.clone(), None as Option<&[u8]>, Some(value))?;
    match result {
        Ok(()) => {
            store.branch_tree.flush()?;
            Ok(branch)
        }
        Err(e) => {
            // A record with the same id already exists.
            if let Some(existing_bytes) = e.current {
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
            } else {
                Err(BranchError::DatabaseError(sled::Error::Unsupported(
                    format!(
                        "Branch with id {} already exists but existing record is unavailable",
                        branch.id
                    ),
                )))
            }
        }
    }
}

/// Gets branch by ID.
pub fn get(context: &Context, id: u64) -> Result<Branch, BranchError> {
    let key = id.to_be_bytes();
    let store = open(context)?;
    match store.branch_tree.get(key)? {
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
    let store = open(context)?;
    let mut branches = Vec::new();
    for item in store.branch_tree.iter() {
        let (_key, value) = item?;
        let branch: Branch = bincode::deserialize(&value)?;
        branches.push(branch);
    }
    Ok(branches)
}

/// Deletes branch by name.
pub fn delete(context: &Context, name: &str) -> Result<(), BranchError> {
    let id = xxh3_64(name.as_bytes());
    // TODO: handle hash collisions.
    let key = id.to_be_bytes();
    let store = open(context)?;
    match store.branch_tree.remove(key)? {
        Some(_ivec) => {
            store.branch_tree.flush()?;
            Ok(())
        }
        None => Err(BranchError::NotFound),
    }
}
