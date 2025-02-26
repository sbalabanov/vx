use crate::context::Context;
use crate::core::blob::Blob;
use crate::core::common::{Digest, DigestExt};
use crate::storage::BLOBS_FOLDER_NAME;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Represents errors that can occur while handling blobs.
#[derive(Error, Debug)]
pub enum BlobError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Blob not found: {0}")]
    BlobNotFound(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Gets the path to the blob storage directory.
fn get_blob_dir(context: &Context) -> PathBuf {
    context.workspace_path.join(BLOBS_FOLDER_NAME)
}

/// Gets the path to a specific blob file based on its content hash.
fn get_blob_path(context: &Context, contenthash: &Digest) -> PathBuf {
    let hash_str = contenthash.to_hex_string();

    // Use the first 2 characters as a subdirectory to avoid too many files in one directory
    let subdir = &hash_str[..2];
    get_blob_dir(context).join(subdir).join(&hash_str[2..])
}

/// Copies a file to the blob store and returns a Blob object.
pub fn from_file(context: &Context, file_path: &Path) -> Result<Blob, BlobError> {
    // Compute the hash of the file
    let (contenthash, size) = Digest::compute_hash(file_path)?;

    // Determine the destination path in the blob store
    let blob_path = get_blob_path(context, &contenthash);

    // Only copy the file if it doesn't already exist in the blob store
    // TODO: Make this atomic and more efficient for both small and large files
    if let Err(e) = fs::copy(file_path, &blob_path) {
        if e.kind() == std::io::ErrorKind::NotFound {
            // Create the directory structure if it doesn't exist
            if let Some(parent) = blob_path.parent() {
                fs::create_dir_all(parent)?;
            }
            // Retry copying the file after creating the directory
            fs::copy(file_path, &blob_path)?;
        } else if e.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(BlobError::IoError(e));
        }
    }

    Ok(Blob::new(contenthash, size))
}

/// Copies a blob from the blob store to the specified file path.
pub fn to_file(context: &Context, contenthash: &Digest, dest_path: &Path) -> Result<(), BlobError> {
    let blob_path = get_blob_path(context, contenthash);

    // Try copying directly to the destination file
    // TODO: Make this atomic and more efficient for both small and large files
    if let Err(e) = fs::copy(&blob_path, dest_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            return Err(BlobError::IoError(e));
        }

        // Check if the error is due to the parent directory not existing
        if let Some(parent) = dest_path.parent() {
            // create_dir_all is concurrently safe
            fs::create_dir_all(parent)?;
        }
        // Retry copying after creating the directory
        if let Err(e) = fs::copy(&blob_path, dest_path) {
            // TODO: even though we've created the directory, something might delete it in the interim,
            // in which case we would return wrong error type BlobNotFound
            return if e.kind() == std::io::ErrorKind::NotFound {
                Err(BlobError::BlobNotFound(contenthash.to_hex_string()))
            } else {
                Err(BlobError::IoError(e))
            };
        }
    }
    Ok(())
}
