/// Represents a binary large object (Blob).
pub struct Blob {
    /// The size of the blob in bytes.
    pub size: u64,
    /// The content hash of the blob.
    pub contenthash: [u8; 16],
}

impl Blob {
    /// Creates a new `Blob` with the given content hash and size.
    ///
    /// # Arguments
    ///
    /// * `contenthash` - A 16-byte array representing the content hash.
    /// * `size` - A 64-bit unsigned integer representing the size of the file on disk.
    ///
    /// # Example
    ///
    /// ```
    /// let hash = [0u8; 16];
    /// let blob = Blob::new(hash, 1024);
    /// ```
    pub fn new(contenthash: [u8; 16], size: u64) -> Self {
        Blob { contenthash, size }
    }
}
