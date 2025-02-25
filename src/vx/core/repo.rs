use crate::context::Context;
use crate::core::branch::Branch;
use crate::storage::repo::{self, RepoError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        Branch::new(&context, String::from("main"), 0, 0, 0)
            .map_err(|e| RepoError::Other(format!("Failed to create main branch: {}", e)))?;

        Ok((repo, context))
    }
}
