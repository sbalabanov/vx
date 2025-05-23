use crate::context::Context;
use crate::core::branch::Branch;
use crate::core::commit::{Commit, CurrentCommitSpec};
use crate::core::tree::Tree;
use crate::storage::repo::{self as repostore, RepoError};
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
        let (repo, context) = repostore::new(name, metadata)?;

        // Create a new empty tree for a centinel commit.
        let tree = Tree::create_empty(&context)
            .map_err(|e| RepoError::Other(format!("Failed to create empty tree: {}", e)))?;

        // Create initial "main" branch using workspace path
        let branch = Branch::create_foundational_branch(&context, String::from("main"))
            .map_err(|e| RepoError::Other(format!("Failed to create main branch: {}", e)))?;

        // TODO: potential inconsistent state here, we have a branch but no commit yet. By design every branch
        // must have at least one commit. For now we will solve it by advising the user to trash the repo and start over.

        // Create a centinel commit with empty tree.
        let commit = Commit::create_zero_commit(
            &context,
            branch.id,
            tree.hash,
            String::from("Initial commit"),
        )
        .map_err(|e| RepoError::Other(format!("Failed to create initial commit: {}", e)))?;

        let current = CurrentCommitSpec {
            commit_id: commit.id,
            ver: branch.ver,
            rebuild_seq: CurrentCommitSpec::NO_REBUILD,
            rebuild_ver: CurrentCommitSpec::NO_REBUILD,
        };

        // Set this as the current branch
        current
            .save(&context)
            .map_err(|e| RepoError::Other(format!("Failed to set current branch: {}", e)))?;

        Ok((repo, context))
    }
}
