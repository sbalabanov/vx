use crate::context::Context;
use crate::core::commit::{Commit, CommitID};
use crate::core::digest::Digest;
use crate::storage::COMMITS_FILE_NAME;
use sled::Tree;
use std::io;
use thiserror::Error;

/// Represents errors that can occur while handling commits.
#[derive(Error, Debug)]
pub enum CommitError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sled::Error),

    #[error("Serialization/Deserialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("Filesystem error: {0}")]
    IoError(#[from] io::Error),

    #[error("Commit not found")]
    NotFound,

    #[error("No branch selected")]
    NoBranchSelected,

    #[error("No changes to commit")]
    NoChanges,

    #[error("{0}")]
    Other(String),
}

const CURRENT_COMMIT_KEY: &[u8] = b"current";

const COMMITS_TREE: &str = "commits";
const METADATA: &str = "sequences";

/// Opens the database and returns a specific tree.
fn open_tree(context: &Context, name: &str) -> Result<Tree, CommitError> {
    let db = sled::open(context.workspace_path.join(COMMITS_FILE_NAME))?;
    let tree = db.open_tree(name)?;
    Ok(tree)
}

/// Creates a new commit.
pub fn new(
    context: &Context,
    commit_id: CommitID,
    ver: u32,
    treehash: Digest,
    message: String,
) -> Result<Commit, CommitError> {
    let commit_tree = open_tree(context, COMMITS_TREE)?;

    let commit = Commit {
        id: commit_id,
        treehash,
        message,
        ver,
    };

    // Use branch ID and sequence number as composite key
    let key = compose_key(commit.id);

    // Create a mutable reference to store any error that happens in the closure
    let mut closure_error: Option<CommitError> = None;

    // We use branch and sequence number to identify a commit, however we also need to track
    // old commits to support undo and eventual consistent states when branch is rebuilt. Thus we store a list
    // of all commits with the same id for each branch, sorted by commit version. Sled does not have
    // a built-in way to store arrays, so we have to serialize/deserialize the entire array on each
    // update. It is possible in the future to either use RocksDB or implement a separate table to store
    // historical commit records.
    commit_tree.update_and_fetch(key, |existing| {
        match existing {
            Some(existing_bytes) => {
                // Try to deserialize existing commits array
                match bincode::deserialize::<Vec<Commit>>(existing_bytes) {
                    Ok(mut commits) => {
                        // Sort by version in descending order
                        // The array is already sorted by version in descending order
                        // Check if the new commit should be at the beginning (most common case)
                        // If we got here, there should be at least one commit in the array.
                        if commit.ver > commits[0].ver {
                            commits.insert(0, commit.clone());
                        } else {
                            // Find the correct position using binary search
                            match commits.binary_search_by(|c| commit.ver.cmp(&c.ver)) {
                                Ok(pos) => {
                                    // found commit with the same version, most likely a previous
                                    // failing attempt to rebuild a branch; safe to overwrite
                                    commits[pos] = commit.clone();
                                }
                                Err(pos) => {
                                    commits.insert(pos, commit.clone());
                                }
                            }
                        }

                        // Serialize the updated array
                        match bincode::serialize(&commits) {
                            Ok(serialized) => Some(serialized),
                            Err(err) => {
                                closure_error = Some(CommitError::SerializationError(err));
                                None
                            }
                        }
                    }
                    Err(err) => {
                        closure_error = Some(CommitError::SerializationError(err));
                        None
                    }
                }
            }
            None => {
                // No existing commits for this key, create a new array with just this commit
                let commits = vec![commit.clone()];
                match bincode::serialize(&commits) {
                    Ok(serialized) => Some(serialized),
                    Err(err) => {
                        closure_error = Some(CommitError::SerializationError(err));
                        None
                    }
                }
            }
        }
    })?;

    // Check if an error occurred in the closure
    if let Some(err) = closure_error {
        return Err(err);
    }

    commit_tree.flush()?;
    Ok(commit)
}

/// Gets commit info by commit ID, with version no greater than specified.
pub fn get(context: &Context, commit_id: CommitID, ver: u32) -> Result<Commit, CommitError> {
    let key = compose_key(commit_id);
    let commit_tree = open_tree(context, COMMITS_TREE)?;

    match commit_tree.get(key)? {
        Some(ivec) => {
            let commits: Vec<Commit> = bincode::deserialize(&ivec)?;

            // Since commits are already sorted by descending version,
            // find the first commit with version <= ver
            commits
                .into_iter()
                .find(|c| c.ver <= ver)
                .ok_or(CommitError::NotFound)
        }
        None => Err(CommitError::NotFound),
    }
}

/// Gets the current commit's branch ID and sequence number.
pub fn get_current(context: &Context) -> Result<CommitID, CommitError> {
    let seq_tree = open_tree(context, METADATA)?;

    match seq_tree.get(CURRENT_COMMIT_KEY)? {
        Some(ivec) => {
            let (branch, seq): (u64, u64) = bincode::deserialize(&ivec)?;
            Ok(CommitID { branch, seq })
        }
        None => Ok(CommitID { branch: 0, seq: 0 }), // Return (0,0) if no commits exist
    }
}

/// Saves the current commit's branch ID and sequence number.
pub fn save_current(context: &Context, commit_id: CommitID) -> Result<(), CommitError> {
    let seq_tree = open_tree(context, METADATA)?;
    let value = bincode::serialize(&(commit_id.branch, commit_id.seq))?;
    seq_tree.insert(CURRENT_COMMIT_KEY, value)?;
    seq_tree.flush()?;
    Ok(())
}

/// Helper function to create composite key from branch ID and sequence number
fn compose_key(commit_id: CommitID) -> [u8; 16] {
    let mut key = [0u8; 16];
    key[..8].copy_from_slice(&commit_id.branch.to_be_bytes());
    key[8..].copy_from_slice(&commit_id.seq.to_be_bytes());
    key
}

/// Lists all commits for a given branch.
pub fn list(
    context: &Context,
    branch_id: u64,
    branch_ver: u32,
    branch_headseq: u64,
) -> Result<Vec<Commit>, CommitError> {
    let commit_tree = open_tree(context, COMMITS_TREE)?;
    let mut commits = Vec::with_capacity(16);

    // Start from the head commit and work backwards
    let mut current_seq = branch_headseq;

    loop {
        // TODO: this is technically parallelizable but we'll likely change the return type to be
        // iterator in the future anyways.

        let key = compose_key(CommitID {
            branch: branch_id,
            seq: current_seq,
        });

        if let Some(ivec) = commit_tree.get(key)? {
            let commit_versions: Vec<Commit> = bincode::deserialize(&ivec)?;

            // Find the first commit with version <= branch_ver
            if let Some(commit) = commit_versions.into_iter().find(|c| c.ver <= branch_ver) {
                commits.push(commit);
            }
        }

        if current_seq == 0 {
            break;
        }
        current_seq -= 1;
    }

    Ok(commits)
}
