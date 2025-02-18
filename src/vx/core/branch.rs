use crate::context::Context;
use crate::storage::branch::{delete, get, get_by_name, list, new, BranchError};
use serde::{Deserialize, Serialize};

/// Represents a branch in the version control system.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Branch {
    /// Unique identifier for the branch.
    pub id: u64,
    /// Name of the branch, must be unique in the repository.
    pub name: String,
    /// Sequence number of the head commit in the branch.
    pub headseq: u64,
    /// Identifier of the parent branch from which this branch was created.
    pub parent: u64,
    /// Sequence number of the parent's commit at the time of branch creation.
    pub parentseq: u64,
}

impl Branch {
    /// Creates a new Branch instance.
    pub fn new(
        context: &Context,
        name: String,
        headseq: u64,
        parent: u64,
        parentseq: u64,
    ) -> Result<Self, BranchError> {
        new(context, name, headseq, parent, parentseq)
    }

    /// Retrieves a branch from the database.
    pub fn get(context: &Context, id: u64) -> Result<Branch, BranchError> {
        get(context, id)
    }

    /// Retrieves a branch from the database by name.
    pub fn get_by_name(context: &Context, name: &str) -> Result<Branch, BranchError> {
        get_by_name(context, name)
    }

    /// Lists all branches from the database.
    pub fn list(context: &Context) -> Result<Vec<Branch>, BranchError> {
        list(context)
    }

    /// Deletes a branch from the database.
    pub fn delete(context: &Context, name: &str) -> Result<(), BranchError> {
        delete(context, name)
    }
}
