use crate::context::Context;
use crate::core::common::Digest;
use crate::core::tree::Tree;
use crate::storage::commit::{self as commitstore, CommitError};
use serde::{Deserialize, Serialize};

/// Identifier of a commit.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CommitID {
    /// Identifier of a branch.
    pub branch: u64,
    /// The sequential number of the commit in a branch.
    pub seq: u64,
}

/// Represents a single commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    // Identifier of a branch.
    pub id: CommitID,
    /// The hash of the file tree root associated with the commit.
    pub treehash: Digest,
    /// The commit message.
    /// TODO: make it a blob?
    pub message: String,
}

impl Commit {
    /// Creates a new Commit instance with the provided values, saves it to the commit store,
    /// and updates the current branch and sequence number.
    pub(crate) fn new(
        context: &Context,
        id: CommitID,
        treehash: Digest,
        message: String,
    ) -> Result<Self, CommitError> {
        let commit = commitstore::new(context, id.branch, id.seq, treehash, message)?;
        commitstore::save_current(context, id)?;
        Ok(commit)
    }

    /// Creates a new commit.
    pub fn make(context: &Context, message: String) -> Result<Self, CommitError> {
        let commit_id = commitstore::get_current(context)?;

        let treehash = Tree::create(context)
            .map_err(|e| CommitError::Other(format!("Tree error: {:?}", e)))?;

        let commit = Commit::get(context, commit_id)?;

        // Check if the current commit's tree hash matches the new tree hash
        // If they're the same, there are no changes to commit
        if commit.treehash == treehash {
            return Err(CommitError::NoChanges);
        }

        Self::new(
            context,
            CommitID {
                branch: commit_id.branch,
                seq: commit_id.seq + 1,
            },
            treehash,
            message,
        )
    }

    /// Returns a formatted string of the commit information.
    pub fn summary(&self) -> String {
        format!("Commit {}\n{}", self.id.seq, self.message)
    }

    /// Lists all commits for the current branch.
    pub fn list(context: &Context) -> Result<Vec<Self>, CommitError> {
        let commit_id = commitstore::get_current(context)?;
        if commit_id.branch == 0 {
            return Err(CommitError::NoBranchSelected);
        }
        commitstore::list(context, commit_id.branch)
    }

    /// Retrieves a specific commit by id.
    pub fn get(context: &Context, id: CommitID) -> Result<Self, CommitError> {
        commitstore::get(context, id)
    }

    /// Retrieves a specific commit by id.
    pub fn get_from_current_branch(context: &Context, seq: u64) -> Result<Self, CommitError> {
        let mut commit_id = commitstore::get_current(context)?;
        commit_id.seq = seq;
        commitstore::get(context, commit_id)
    }

    /// Retrieves the current commit.
    pub fn get_current(context: &Context) -> Result<Self, CommitError> {
        let commit_id = commitstore::get_current(context)?;
        commitstore::get(context, commit_id)
    }
}
