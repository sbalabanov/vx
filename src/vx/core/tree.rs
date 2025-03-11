use crate::context::Context;
use crate::core::blob::Blob;
use crate::core::common::{Digest, DigestExt};
use crate::storage::tree::{self, TreeError};
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::Xxh3;

/// Represents a folder content in a file tree.
/// TODO: add required attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    /// Hash of the folder content, i.e subfolders' and files' hashes.
    pub hash: Digest,
    /// Subfolders in this folder, sorted alphabetically by name.
    pub folders: Vec<File>,
    /// Files in this folder, sorted alphabetically by name.
    pub files: Vec<File>,
}

/// Represents a file in the file tree.
/// TODO: add required attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    /// Name of the file or folder.
    pub name: String,
    /// Blob containing the file's data or folder's content.
    pub blob: Blob,
}

impl Folder {
    /// Creates a new empty folder with default values.
    pub fn default() -> Self {
        Self {
            hash: Digest::NONE,
            folders: Vec::new(),
            files: Vec::new(),
        }
    }

    /// Creates a new folder with the specified contents.
    pub fn new(context: &Context, folders: Vec<File>, files: Vec<File>) -> Result<Self, TreeError> {
        // Calculate hash based on contents
        let mut hasher = Xxh3::new();

        // Add folder names and hashes to the hash calculation
        for folder in &folders {
            hasher.update(folder.name.as_bytes());
            hasher.update(&folder.blob.contenthash.to_be_bytes());
        }

        // Add file names and hashes to the hash calculation
        for file in &files {
            hasher.update(file.name.as_bytes());
            hasher.update(&file.blob.contenthash.to_be_bytes());
        }

        let folder = Self {
            hash: hasher.digest128(),
            folders,
            files,
        };

        tree::save_folder(context, &folder)?;

        Ok(folder)
    }

    /// Retrieves a folder from the database by its hash.
    pub fn get(context: &Context, hash: &Digest) -> Result<Self, TreeError> {
        tree::get_folder(context, hash)
    }
}
