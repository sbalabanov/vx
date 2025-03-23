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
    // Identifier of a commit.
    pub id: CommitID,
    // Version of the branch the commit belongs to, each change increases version.
    pub ver: u32,
    /// The hash of the file tree root associated with the commit.
    pub treehash: Digest,
    /// The commit message.
    /// TODO: make it a blob?
    pub message: String,
    // TODO: add author and other metadata
}

impl Commit {
    /// Creates a new commit.
    pub fn new(context: &Context, message: String) -> Result<Self, CommitError> {
        let treehash = Tree::create(context)
            .map_err(|e| CommitError::Other(format!("Tree error: {:?}", e)))?;

        let commit = Commit::get_current(context)?;

        // Check if the current commit's tree hash matches the new tree hash
        // If they're the same, there are no changes to commit
        if commit.treehash == treehash {
            return Err(CommitError::NoChanges);
        }

        let branch = Branch::get(context, commit.id.branch)
            .map_err(|e| CommitError::Other(format!("Branch error: {:?}", e)))?;

        let new_ver = branch.ver + 1;

        let new_commit_id = CommitID {
            branch: commit.id.branch,
            seq: commit.id.seq + 1,
        };

        // Create a new commit with the same ID as the current one, but a different version.
        let new_commit = commitstore::new(context, new_commit_id, new_ver, treehash, message)?;

        Commit::save_current(context, new_commit_id)?;

        // TODO: rebuild branch if commit is in the middle

        // TODO: potential race condition between new commit and branch update
        // Current commit may be recorded before the branch really updates, so in case of a failure
        // the current commit's seq will be ahead of the branch's headseq.

        Branch::advance_head(context, new_commit.id.branch, new_commit.id.seq, new_ver)
            .map_err(|e| CommitError::Other(format!("Failed to advance branch head: {}", e)))?;

        Ok(new_commit)
    }

    /// Amends the current commit with a new tree and optionally a new message.
    /// If no message is provided, the existing message is preserved.
    pub fn amend(context: &Context, message: Option<String>) -> Result<Self, CommitError> {
        // Get the current commit
        let current_commit = Commit::get_current(context)?;

        // Check if this is a centinel commit (seq is zero)
        if current_commit.id.seq == CommitID::SEQ_ZERO {
            return Err(CommitError::Other(
                "Cannot amend centinel commit".to_string(),
            ));
        }

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

        let branch = Branch::get(context, current_commit.id.branch)
            .map_err(|e| CommitError::Other(format!("Branch error: {:?}", e)))?;

        let new_ver = branch.ver + 1;

        // Create a new commit with the same ID as the current one, but a different version.
        let commit = commitstore::new(
            context,
            current_commit.id,
            new_ver,
            treehash,
            commit_message,
        )?;

        // No need to update current commit pointer since we're using the same ID

        // TODO: rebuild branch if commit is in the middle

        Branch::advance_head(context, commit.id.branch, commit.id.seq, new_ver)
            .map_err(|e| CommitError::Other(format!("Failed to advance branch head: {}", e)))?;

        Ok(commit)
    }

    /// Lists all commits for the current branch.
    /// TODO: change it to iterator or paged vector to avoid loading all commits into memory for long
    /// branches.
    pub fn list(context: &Context) -> Result<Vec<Self>, CommitError> {
        let commit_id = commitstore::get_current(context)?;
        let branch = Branch::get(context, commit_id.branch)
            .map_err(|e| CommitError::Other(format!("Branch error: {:?}", e)))?;
        commitstore::list(context, branch.id, branch.ver, branch.headseq)
    }

    /// Retrieves a specific commit by id.
    pub fn get(context: &Context, id: CommitID) -> Result<Self, CommitError> {
        let branch = Branch::get(context, id.branch)
            .map_err(|e| CommitError::Other(format!("Branch error: {:?}", e)))?;

        commitstore::get(context, id, branch.ver)
    }

    /// Retrieves a specific commit by id.
    pub fn get_from_current_branch(context: &Context, seq: u64) -> Result<Self, CommitError> {
        let mut commit_id = commitstore::get_current(context)?;
        let branch = Branch::get(context, commit_id.branch)
            .map_err(|e| CommitError::Other(format!("Branch error: {:?}", e)))?;
        commit_id.seq = seq;
        commitstore::get(context, commit_id, branch.ver)
    }

    /// Retrieves the current commit.
    pub fn get_current(context: &Context) -> Result<Self, CommitError> {
        let commit_id = commitstore::get_current(context)?;
        let branch = Branch::get(context, commit_id.branch)
            .map_err(|e| CommitError::Other(format!("Branch error: {:?}", e)))?;
        // TODO: process branch_id:0 current commits (i.e. new branch without commits).
        // Either add a centinel commit or resolve to the branch's headseq.
        commitstore::get(context, commit_id, branch.ver)
    }

    /// Creates a new Commit instance which should start a branch and save it to the store.
    /// Typically used as a centinel when new branch is created.
    pub(crate) fn create_zero_commit(
        context: &Context,
        branch_id: u64,
        treehash: Digest,
        message: String,
    ) -> Result<Self, CommitError> {
        let commit = commitstore::new(
            context,
            CommitID {
                branch: branch_id,
                seq: CommitID::SEQ_ZERO,
            },
            0,
            treehash,
            message,
        )?;
        Ok(commit)
    }

    /// Saves the current commit.
    pub(crate) fn save_current(context: &Context, id: CommitID) -> Result<(), CommitError> {
        commitstore::save_current(context, id)
    }
}
