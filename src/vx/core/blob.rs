use crate::context::Context;
use crate::core::digest::Digest;
use crate::storage::blob::{self as blobstore, BlobError};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::Path;

/// Represents a binary large object (Blob).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blob {
    /// Hash of the blob's content, used as a unique identifier
    pub contenthash: Digest,
    /// Size of the blob in bytes
    pub size: u64,
}

impl Blob {
    /// Opens the blob database.
    pub(crate) fn open(context: &Context) -> Result<Db, BlobError> {
        blobstore::open(context)
    }

    /// Creates a `Blob` from a file, compute digest and size, and store it in the database.
    pub(crate) fn from_file(
        context: &Context,
        db: &Db,
        file_path: &Path,
    ) -> Result<Self, BlobError> {
        blobstore::from_file(context, db, file_path)
    }

    /// Copies a `Blob` to a file by calling the appropriate function from storage.
    pub(crate) fn to_file(
        context: &Context,
        db: &Db,
        contenthash: Digest,
        dest_path: &Path,
    ) -> Result<(), BlobError> {
        blobstore::to_file(context, db, contenthash, dest_path)
    }
}
