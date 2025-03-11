use crate::context::Context;
use crate::core::commit::{Commit, CommitID};
use crate::core::common::Digest;
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
    branch: u64,
    seq: u64,
    treehash: Digest,
    message: String,
) -> Result<Commit, CommitError> {
    let commit_tree = open_tree(context, COMMITS_TREE)?;

    let commit = Commit {
        id: CommitID { branch, seq },
        treehash,
        message,
    };

    // Use branch ID and sequence number as composite key
    let key = compose_key(commit.id);
    let value = bincode::serialize(&commit)?;

    commit_tree.insert(key, value)?;
    commit_tree.flush()?;
    Ok(commit)
}

/// Gets commit info by branch ID and sequence number.
pub fn get(context: &Context, branch: u64, seq: u64) -> Result<Commit, CommitError> {
    let commit_id = CommitID { branch, seq };
    let key = compose_key(commit_id);
    let commit_tree = open_tree(context, COMMITS_TREE)?;

    match commit_tree.get(key)? {
        Some(ivec) => {
            let commit: Commit = bincode::deserialize(&ivec)?;
            Ok(commit)
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
pub fn list(context: &Context, branch: u64) -> Result<Vec<Commit>, CommitError> {
    let commit_tree = open_tree(context, COMMITS_TREE)?;
    let mut commits = Vec::new();

    // TODO: smarter and faster retrieval
    for result in commit_tree.iter() {
        let (key, value) = result?;
        let (key_branch, _) = decompose_key(&key);

        if key_branch == branch {
            let commit: Commit = bincode::deserialize(&value)?;
            commits.push(commit);
        }
    }

    Ok(commits)
}

/// Helper function to decompose composite key into branch ID and sequence number
fn decompose_key(key: &[u8]) -> (u64, u64) {
    let branch = u64::from_be_bytes(key[..8].try_into().unwrap());
    let seq = u64::from_be_bytes(key[8..].try_into().unwrap());
    (branch, seq)
}
