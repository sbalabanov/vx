use crate::context::Context;
use crate::core::commit::{Commit, CommitID};
use crate::storage::branch::{self as branchstore, BranchError};
use serde::{Deserialize, Serialize};

/// Represents a branch in the version control system.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Branch {
    /// Unique identifier for the branch.
    pub id: u64,
    /// Name of the branch, must be unique in the repository.
    pub name: String,
    /// Sequence number of the head commit in this branch.
    pub headseq: u64,
    /// Version of the branch, each change increases version.
    pub ver: u32,
    /// Identifier of the parent branch from which this branch was created.
    pub parent: u64,
    /// Sequence number of the parent's commit at the time of branch creation.
    pub parentseq: u64,
}

const FOUNDATIONAL_ID: u64 = 0;

impl Branch {
    /// Creates a new Branch instance off the current commit.
    pub fn new(context: &Context, name: String) -> Result<Self, BranchError> {
        validate_branch_name(&name)?;

        let commit = Commit::get_current(context)
            .map_err(|e| BranchError::Other(format!("Failed to get current commit ID: {}", e)))?;
        let parent_branch = branchstore::get(context, commit.id.branch)?;

        if !parent_branch.is_foundational() {
            // Fundamentally we can allow branches to be based on one another, but it will complicate
            // rebasing algorithms, so for the time being we only allow new branches to be based off main,
            // which creates a very simple tree structure.
            return Err(BranchError::InvalidParent(
                "Parent branch must be foundational (i.e. main)".to_string(),
            ));
        }

        // Create the branch
        let branch = branchstore::new(
            context,
            name,
            CommitID::SEQ_ZERO,
            commit.id.branch,
            commit.id.seq,
        )?;

        // create a centinel commit for the new branch by copying the current commit.
        // TODO: potential race condition here, we have a branch but no commit yet. By design every branch
        // must have at least one commit.
        let branch_commit =
            Commit::create_zero_commit(context, branch.id, commit.treehash, commit.message)
                .map_err(|e| {
                    BranchError::Other(format!("Failed to create centinel commit: {}", e))
                })?;

        Commit::save_current(context, branch_commit.id)
            .map_err(|e| BranchError::Other(format!("Failed to set current branch: {}", e)))?;

        Ok(branch)
    }

    /// Retrieves the current branch based on the current commit.
    pub fn get_current(context: &Context) -> Result<Self, BranchError> {
        // Get the current commit to find out which branch we're on
        let current_commit = Commit::get_current(context)
            .map_err(|e| BranchError::Other(format!("Failed to get current commit: {}", e)))?;

        // Retrieve the branch using the branch ID from the current commit
        Self::get(context, current_commit.id.branch)
    }

    /// Checks if this branch is the foundational branch (not based on any other branch).
    pub fn is_foundational(&self) -> bool {
        self.parent == FOUNDATIONAL_ID
    }

    /// Retrieves a branch from the database by name.
    pub fn get_by_name(context: &Context, name: &str) -> Result<Branch, BranchError> {
        branchstore::get_by_name(context, name)
    }

    /// Lists all branches from the database.
    pub fn list(context: &Context) -> Result<Vec<Branch>, BranchError> {
        branchstore::list(context)
    }

    /// Retrieves a branch from the database by ID.
    pub fn get(context: &Context, id: u64) -> Result<Branch, BranchError> {
        branchstore::get(context, id)
    }

    /// Creates a foundational branch, i.e. the one that is not based on any other branch.
    /// Only saves the branch to datastore and should be called as a part of a bigger workflow.
    pub(crate) fn create_foundational_branch(
        context: &Context,
        name: String,
    ) -> Result<Branch, BranchError> {
        validate_branch_name(&name)?;

        // Create the foundational branch with ID 0, no parent
        // Create the branch
        let branch = branchstore::new(
            context,
            name,
            CommitID::SEQ_ZERO, // Initial head sequence is zero
            FOUNDATIONAL_ID,    // Parent is itself (foundational ID)
            CommitID::SEQ_ZERO, // Parent sequence is 0 since there's no real parent
        )?;

        Ok(branch)
    }

    pub(crate) fn advance_head(
        context: &Context,
        branch_id: u64,
        new_headseq: u64,
        new_ver: u32,
    ) -> Result<Branch, BranchError> {
        branchstore::update_headseq(context, branch_id, new_headseq, new_ver)
    }
}

/// Validates if a branch name is valid.
///
/// Branch names can only contain lowercase letters, numbers, and the characters: . / -
fn validate_branch_name(name: &str) -> Result<(), BranchError> {
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '/' || c == '-')
    {
        return Err(BranchError::InvalidName(
            "Branch names can only contain lowercase letters, numbers, and the following characters: . / -"
                .to_string(),
        ));
    }
    Ok(())
}
