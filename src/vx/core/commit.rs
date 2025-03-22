use crate::context::Context;
use crate::core::branch::Branch;
use crate::core::digest::Digest;
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

impl CommitID {
    pub(crate) const SEQ_ZERO: u64 = 0;

    /// Resolves a string in format "branch_name:seq" into a CommitID
    ///   - If spec is an integer, it's treated as a sequence number on the current branch
    ///   - Otherwise, it's treated as a branch name with the head sequence
    pub fn resolve(context: &Context, spec: &str) -> Result<Self, CommitError> {
        match spec.find(':') {
            Some(pos) => {
                // Format is "branch_name:seq"
                let branch_name = &spec[0..pos];
                let seq_str = &spec[pos + 1..];

                let seq = seq_str.parse::<u64>().map_err(|_| {
                    CommitError::Other(format!("Invalid sequence number: {}", seq_str))
                })?;

                // Always look up branch by name
                let branch = Branch::get_by_name(&context, branch_name)
                    .map_err(|e| CommitError::Other(format!("Branch error: {:?}", e)))?;
                Ok(CommitID {
                    branch: branch.id,
                    seq,
                })
            }
            None => {
                match spec.parse::<u64>() {
                    Ok(seq) => {
                        // No separator and spec is an integer - use as sequence on current branch
                        let current_commit_id = commitstore::get_current(context)?;
                        Ok(CommitID {
                            branch: current_commit_id.branch,
                            seq,
                        })
                    }
                    Err(_) => {
                        // No separator and spec is not an integer - treat as branch name
                        let branch = Branch::get_by_name(&context, spec)
                            .map_err(|e| CommitError::Other(format!("Branch error: {:?}", e)))?;
                        Ok(CommitID {
                            branch: branch.id,
                            seq: branch.headseq,
                        })
                    }
                }
            }
        }
    }
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
    // TODO: add author and other metadata
}

impl Commit {
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

        let commit = Commit::new(
            context,
            CommitID {
                branch: commit_id.branch,
                seq: commit_id.seq + 1,
            },
            treehash,
            message,
        )?;

        // TODO: potential race condition between new commit and branch update
        // Current commit may be recorded before the branch really updates, so in case of a failure
        // the current commit's seq will be ahead of the branch's headseq.

        Branch::advance_head(context, commit.id.branch, commit.id.seq)
            .map_err(|e| CommitError::Other(format!("Failed to advance branch head: {}", e)))?;

        // TODO: handle rebase if commit is in the middle of a branch

        Ok(commit)
    }

    /// Amends the current commit with a new tree and optionally a new message.
    /// If no message is provided, the existing message is preserved.
    pub fn amend(context: &Context, message: Option<String>) -> Result<Self, CommitError> {
        // Get the current commit
        let current_commit = Commit::get_current(context)?;

        // Generate a new tree hash from the current working directory
        let treehash = Tree::create(context)
            .map_err(|e| CommitError::Other(format!("Tree error: {:?}", e)))?;

        // If no changes to the tree and the message remains the same, return NoChanges error
        if current_commit.treehash == treehash
            && (message.is_none() || message.as_ref() == Some(&current_commit.message))
        {
            return Err(CommitError::NoChanges);
        }

        // Use the new message if provided, otherwise keep the existing one
        let commit_message = message.unwrap_or_else(|| current_commit.message.clone());

        // Create a new commit with the same ID as the current one
        let commit = commitstore::new(
            context,
            current_commit.id.branch,
            current_commit.id.seq,
            treehash,
            commit_message,
        )?;

        // No need to update current commit pointer since we're using the same ID

        // TODO: handle rebase if commit is in the middle of a branch

        Ok(commit)
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

    /// Retrieves the current commit ID.
    pub fn get_current_id(context: &Context) -> Result<CommitID, CommitError> {
        commitstore::get_current(context)
    }

    /// Retrieves the current commit.
    pub fn get_current(context: &Context) -> Result<Self, CommitError> {
        let commit_id = commitstore::get_current(context)?;
        // TODO: process branch_id:0 current commits (i.e. new branch without commits).
        // Either add a centinel commit or resolve to the branch's headseq.
        commitstore::get(context, commit_id)
    }

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

    /// Saves the current commit.
    pub(crate) fn save_current(context: &Context, id: CommitID) -> Result<(), CommitError> {
        commitstore::save_current(context, id)
    }
}
