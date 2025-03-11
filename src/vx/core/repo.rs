use crate::context::Context;
use crate::core::branch::Branch;
use crate::storage::repo::{self, RepoError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::commit::{Commit, CommitID};
use super::tree::Folder;

/// Represents a repository in the version control system.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Repo {
    /// Name of the repository, must be unique.
    pub name: String,
    /// Arbitrary metadata stored as key-value pairs.
    pub metadata: HashMap<String, String>,
}

impl Repo {
    /// Creates a new Repo instance.
    pub fn new(
        name: String,
        metadata: HashMap<String, String>,
    ) -> Result<(Self, Context), RepoError> {
        // Validate repo name - only allow lowercase alphanumeric and : . / _ characters
        if !name.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '/' || c == '-'
        }) {
            return Err(RepoError::InvalidName(
                "Repository names can only contain lowercase letters, numbers, and the following characters: . / -"
                    .to_string(),
            ));
        }
        let (repo, context) = repo::new(name, metadata)?;

        // Create initial "main" branch using workspace path
        let branch = Branch::new(&context, String::from("main"), 0, 0, 0)
            .map_err(|e| RepoError::Other(format!("Failed to create main branch: {}", e)))?;

        // Create a new centinel commit with empty tree
        let tree = Folder::new(&context, vec![], vec![])
            .map_err(|e| RepoError::Other(format!("Failed to create empty tree: {}", e)))?;
        Commit::new(
            &context,
            CommitID {
                branch: branch.id,
                seq: 0, // zero as a sentinel, first user commit will start from 1
            },
            tree.hash,
            String::from("Initial commit"),
        )
        .map_err(|e| RepoError::Other(format!("Failed to create initial commit: {}", e)))?;

        Ok((repo, context))
    }
}
