use crate::context::Context;
use crate::core::digest::Digest;
use crate::core::tree::Tree as VxTree;
use sled::Db;
use thiserror::Error;

/// Represents errors that can occur while handling tree operations.
#[derive(Error, Debug)]
pub enum TreeError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sled::Error),

    #[error("Serialization/Deserialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("Filesystem error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Tree not found")]
    TreeNotFound,

    #[error("{0}")]
    Other(String),
}

const TREE_FILE_NAME: &str = "tree.db";

/// Opens the database and returns a specific tree.
pub fn open(context: &Context) -> Result<Db, TreeError> {
    let db = sled::open(context.workspace_path.join(TREE_FILE_NAME))?;
    Ok(db)
}

/// Saves a tree to the database.
pub fn save(db: &Db, tree: &VxTree) -> Result<(), TreeError> {
    let key = tree.hash.to_be_bytes();
    let value = bincode::serialize(tree)?;

    db.insert(key, value)?;
    // it is up to the caller to flush when needed
    Ok(())
}

/// Retrieves a folder from the database by its hash.
pub fn get(db: &Db, hash: Digest) -> Result<VxTree, TreeError> {
    let key = hash.to_be_bytes();

    if let Some(ivec) = db.get(key)? {
        let tree: VxTree = bincode::deserialize(&ivec)?;
        Ok(tree)
    } else {
        Err(TreeError::TreeNotFound)
    }
}
