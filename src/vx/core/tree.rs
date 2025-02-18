use crate::core::blob::Blob;
use crate::core::common::Digest;

/// Represents a folder in a file tree.
pub struct Folder {
    /// Hash of the folder, including content of its children and files.
    pub hash: Digest,
    /// Name of the folder.
    pub name: String,
    /// Children of the folder.
    pub children: Vec<Digest>,
    /// Parent of the folder.
    pub parent: Option<Digest>,
    /// Files in this folder.
    pub files: Vec<File>,
}

/// Represents a file in the file tree.
pub struct File {
    /// Hash of the file, including its name and content.
    pub hash: Digest,
    /// Name of the file.
    pub name: String,
    /// Blob containing the file's data.
    pub blob: Blob,
}
