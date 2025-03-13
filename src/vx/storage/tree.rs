use crate::context::Context;
use crate::core::common::Digest;
use crate::core::tree::Tree as VxTree;
use sled::Tree;
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

    #[error("Folder not found")]
    FolderNotFound,

    #[error("File not found")]
    FileNotFound,

    #[error("{0}")]
    Other(String),
}

const TREE_NAME: &str = "tree";
const TREE_FILE_NAME: &str = "tree.db";

/// Opens the database and returns a specific tree.
fn open_tree(context: &Context, name: &str) -> Result<Tree, TreeError> {
    let db = sled::open(context.workspace_path.join(TREE_FILE_NAME))?;
    let tree = db.open_tree(name)?;
    Ok(tree)
}

/// Saves a tree to the database.
pub fn save(context: &Context, tree: &VxTree) -> Result<(), TreeError> {
    let sled_tree = open_tree(context, TREE_NAME)?;
    let key = tree.hash.to_be_bytes();
    let value = bincode::serialize(tree)?;

    sled_tree.insert(key, value)?;
    sled_tree.flush()?;
    Ok(())
}

/// Retrieves a folder from the database by its hash.
pub fn get(context: &Context, hash: &Digest) -> Result<VxTree, TreeError> {
    let sled_tree = open_tree(context, TREE_NAME)?;
    let key = hash.to_be_bytes();

    if let Some(ivec) = sled_tree.get(key)? {
        let tree: VxTree = bincode::deserialize(&ivec)?;
        Ok(tree)
    } else {
        Err(TreeError::FolderNotFound)
    }
}
