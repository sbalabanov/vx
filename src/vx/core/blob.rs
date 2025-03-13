use crate::context::Context;
use crate::core::common::Digest;
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
    /// Creates a `Blob` from a file, compute digest and size, and store it in the database.
    pub fn from_file(context: &Context, file_path: &Path) -> Result<Self, BlobError> {
        crate::storage::blob::from_file(context, file_path)
    }

    /// Copies a `Blob` to a file by calling the appropriate function from storage.
    pub fn to_file(&self, context: &Context, dest_path: &Path) -> Result<(), BlobError> {
        crate::storage::blob::to_file(context, self.contenthash, dest_path)
    }
}
