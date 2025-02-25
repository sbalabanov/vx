use crate::context::Context;
use crate::core::repo::Repo;
use crate::global::DATA_FOLDER;
use crate::storage::REPO_FILE_NAME;
use sled::Error as SledError;
use std::collections::HashMap;
use std::fs;
use thiserror::Error;

/// Represents errors that can occur while handling repositories.
#[derive(Error, Debug)]
pub enum RepoError {
    #[error("Repository not found")]
    NotFound,

    #[error("Repository with name '{0}' already exists")]
    RepoExists(String),

    #[error("Invalid repository name: {0}")]
    InvalidName(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization/Deserialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("Database error: {0}")]
    DatabaseError(#[from] SledError),

    #[error("{0}")]
    Other(String),
}

/// Creates a new repository.
pub fn new(name: String, metadata: HashMap<String, String>) -> Result<(Repo, Context), RepoError> {
    let current_dir = std::env::current_dir()?;
    let repo_path = current_dir.join(&name);

    // Try to create repository directory atomically
    match fs::create_dir(&repo_path) {
        Ok(_) => (),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                return Err(RepoError::RepoExists(name));
            }
            return Err(RepoError::IoError(e));
        }
    }

    // Create .vx workspace directory
    let workspace_path = repo_path.join(DATA_FOLDER);
    fs::create_dir_all(&workspace_path)?;

    // Open repo database and create metadata tree
    let db = sled::open(workspace_path.join(REPO_FILE_NAME))?;
    let metadata_tree = db.open_tree("metadata")?;

    // Save each metadata key-value pair separately
    for (key, value) in metadata.iter() {
        let full_key = format!("{}:{}", name, key);
        metadata_tree.insert(full_key.as_bytes(), value.as_bytes())?;
    }
    metadata_tree.flush()?;

    let context = Context::new(workspace_path, repo_path);

    Ok((Repo { name, metadata }, context))
}
