use crate::context::Context;
use crate::core::common::Digest;
use crate::storage::commit::{self as commitstore, CommitError};
use serde::{Deserialize, Serialize};

/// Represents a single commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    // Identifier of a branch.
    pub branch: u64,
    /// The sequential number of the commit.
    pub seq: u64,
    /// The hash of the file tree associated with the commit.
    pub treehash: Digest,
    /// The commit message.
    /// TODO: make it a blob?
    pub message: String,
}

impl Commit {
    /// Creates a new commit.
    pub fn new(context: &Context, message: String) -> Result<Self, CommitError> {
        let (branch, seq) = commitstore::get_current(context)?;
        if branch == 0 {
            return Err(CommitError::NoBranchSelected);
        }
        let next_seq = seq + 1;
        let treehash: Digest = 0; // TODO: Replace with actual tree hash

        let commit = commitstore::new(context, branch, next_seq, treehash, message)?;
        commitstore::save_current(context, branch, next_seq)?;

        Ok(commit)
    }

    /// Returns a formatted string of the commit information.
    pub fn summary(&self) -> String {
        format!("Commit {}\n{}", self.seq, self.message)
    }

    /// Lists all commits for the current branch.
    pub fn list(context: &Context) -> Result<Vec<Self>, CommitError> {
        let (branch, _) = commitstore::get_current(context)?;
        if branch == 0 {
            return Err(CommitError::NoBranchSelected);
        }
        commitstore::list(context, branch)
    }

    /// Retrieves a specific commit by its sequence number.
    pub fn get(context: &Context, seq: u64) -> Result<Self, CommitError> {
        let (branch, _) = commitstore::get_current(context)?;
        if branch == 0 {
            return Err(CommitError::NoBranchSelected);
        }
        commitstore::get(context, branch, seq)
    }
}
