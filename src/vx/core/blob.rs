use crate::context::Context;
use crate::core::common::{Digest, DigestExt};
use crate::storage::blob::BlobError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents a binary large object (Blob).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blob {
    pub contenthash: Digest,
    pub size: u64,
}

impl Blob {
    /// Creates a new `Blob`.
    pub fn new(contenthash: Digest, size: u64) -> Self {
        Blob { contenthash, size }
    }

    /// Creates a `Blob` object from a file by computing the hash of the file.
    pub fn from_file(file_path: &Path) -> Result<Self, std::io::Error> {
        let (hash, size) = Digest::compute_hash(&file_path)?;
        let blob = Self::new(hash, size);
        Ok(blob)
    }

    /// Creates a `Blob` from a file by calling the appropriate function from storage.
    pub fn from_file_and_store(context: &Context, file_path: &Path) -> Result<Self, BlobError> {
        crate::storage::blob::from_file(context, file_path)
    }

    /// Copies a `Blob` to a file by calling the appropriate function from storage.
    pub fn to_file_and_store(&self, context: &Context, dest_path: &Path) -> Result<(), BlobError> {
        crate::storage::blob::to_file(context, self.contenthash, dest_path)
    }
}
