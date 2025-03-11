use crate::context::Context;
use crate::core::common::Digest;
use crate::core::tree::Folder;
use sled::Tree;
use thiserror::Error;

/// Represents errors that can occur while handling tree operations.
#[derive(Error, Debug)]
pub enum TreeError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sled::Error),

    #[error("Serialization/Deserialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("Folder not found")]
    FolderNotFound,

    #[error("File not found")]
    FileNotFound,
}

const FOLDERS_TREE: &str = "folders";
const TREE_FILE_NAME: &str = "tree.db";

/// Opens the database and returns a specific tree.
fn open_tree(context: &Context, name: &str) -> Result<Tree, TreeError> {
    let db = sled::open(context.workspace_path.join(TREE_FILE_NAME))?;
    let tree = db.open_tree(name)?;
    Ok(tree)
}

/// Saves a folder to the database.
pub fn save_folder(context: &Context, folder: &Folder) -> Result<(), TreeError> {
    let folder_tree = open_tree(context, FOLDERS_TREE)?;
    let key = folder.hash.to_be_bytes();
    let value = bincode::serialize(folder)?;

    folder_tree.insert(key, value)?;
    folder_tree.flush()?;
    Ok(())
}

/// Retrieves a folder from the database by its hash.
pub fn get_folder(context: &Context, hash: &Digest) -> Result<Folder, TreeError> {
    let folder_tree = open_tree(context, FOLDERS_TREE)?;
    let key = hash.to_be_bytes();

    if let Some(ivec) = folder_tree.get(key)? {
        let folder: Folder = bincode::deserialize(&ivec)?;
        Ok(folder)
    } else {
        Err(TreeError::FolderNotFound)
    }
}
