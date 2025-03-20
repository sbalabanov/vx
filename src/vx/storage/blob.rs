use crate::context::Context;
use crate::core::blob::Blob;
use crate::core::digest::{Digest, DigestExt};
use crate::storage::BLOBS_FOLDER_NAME;
use sled::Db;
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

    #[error("Database error: {0}")]
    DatabaseError(#[from] sled::Error),

    #[error("Serialization/Deserialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("Other error: {0}")]
    Other(String),
}

const BLOB_DB_FILE_NAME: &str = "blob.db";

/// Opens the blob database and returns a connection.
pub fn open(context: &Context) -> Result<Db, BlobError> {
    let db = sled::open(context.workspace_path.join(BLOB_DB_FILE_NAME))?;
    Ok(db)
}

/// Gets the path to the blob storage directory.
fn get_blob_dir(context: &Context) -> PathBuf {
    context.workspace_path.join(BLOBS_FOLDER_NAME)
}

/// Gets the path to a specific blob file based on its content hash.
fn get_blob_path(context: &Context, contenthash: Digest) -> PathBuf {
    let hash_str = contenthash.to_hex_string();

    // Use the first 2 characters as a subdirectory to avoid too many files in one directory
    let subdir = &hash_str[..2];
    get_blob_dir(context).join(subdir).join(&hash_str[2..])
}

/// Copies a file to the blob store and returns a Blob object.
pub fn from_file(context: &Context, db: &Db, file_path: &Path) -> Result<Blob, BlobError> {
    // Compute the hash of the file
    let (contenthash, size) = Digest::compute_hash(file_path)?;

    // Check if the blob already exists in the database.
    // Unlike file system, database is atomic so if the record is in the database,
    // the actual blob storage is confirmed to have the blob.
    let key = contenthash.to_be_bytes();
    if db.contains_key(&key)? {
        // The blob is already in the store, no need to copy it.
        return Ok(Blob { contenthash, size });
    }

    // Determine the destination path in the blob store
    let blob_path = get_blob_path(context, contenthash);

    // TODO: handle a potential data race when two threads try to copy to the same destination file.
    // May be the path to a file in blob store has to be randomly generated and saved as a reference
    // in the database.
    if let Err(e) = fs::copy(file_path, &blob_path) {
        // If the error is not due to missing directory, return early
        if e.kind() != std::io::ErrorKind::NotFound {
            return Err(BlobError::IoError(e));
        }

        // Create the directory structure if it doesn't exist
        if let Some(parent) = blob_path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Retry copying the file after creating the directory
        fs::copy(file_path, &blob_path)?;
    }

    // Store the blob metadata in the database
    let blob = Blob { contenthash, size };
    let value = bincode::serialize(&blob)?;
    db.insert(key, value)?;
    // The caller is responsible for flushing when needed

    Ok(blob)
}

/// Copies a blob from the blob store to the specified file path.
pub fn to_file(
    context: &Context,
    db: &Db,
    contenthash: Digest,
    dest_path: &Path,
) -> Result<(), BlobError> {
    // Check if the blob exists in the database
    let key = contenthash.to_be_bytes();
    if !db.contains_key(&key)? {
        // TODO: is this check really needed?
        // We should not have concurrent writes and reads at the same time.
        return Err(BlobError::BlobNotFound(contenthash.to_hex_string()));
    }

    let blob_path = get_blob_path(context, contenthash);

    // Try copying directly to the destination file.
    // The caller should guarantee that only one thread is copying to the same destination file.
    // TODO: handle permissions / attributes.
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
        fs::copy(&blob_path, dest_path)?;
    }

    Ok(())
}

/// Retrieves blob metadata from the database.
pub fn get_blob_metadata(db: &Db, contenthash: Digest) -> Result<Blob, BlobError> {
    let key = contenthash.to_be_bytes();

    match db.get(key)? {
        Some(ivec) => {
            let blob: Blob = bincode::deserialize(&ivec)?;
            Ok(blob)
        }
        None => Err(BlobError::BlobNotFound(contenthash.to_hex_string())),
    }
}
