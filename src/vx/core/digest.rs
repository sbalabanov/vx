use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use xxhash_rust::xxh3::Xxh3;

pub type Digest = u128;

/// Trait for converting a digest to a hexadecimal string representation and computing a hash.
pub trait DigestExt {
    const NONE: Digest = 0;

    /// Converts the digest to a hexadecimal string representation.
    fn to_hex_string(&self) -> String;

    /// Computes the hash of a file and returns it as a Digest and the size of the file.
    fn compute_hash(file_path: &Path) -> Result<(Digest, u64), std::io::Error>;
}

impl DigestExt for Digest {
    fn to_hex_string(&self) -> String {
        // Convert the u128 digest to a hexadecimal string representation
        format!("{:032x}", self)
    }

    fn compute_hash(file_path: &Path) -> Result<(Digest, u64), std::io::Error> {
        const BUFFER_SIZE: usize = 8192; // 8 KB

        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut buffer = [0u8; BUFFER_SIZE];
        let mut hasher = Xxh3::new();
        let mut total_size = 0;

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break; // EOF reached
            }
            hasher.update(&buffer[..bytes_read]); // Feed chunks into the hasher
            total_size += bytes_read as u64;
        }

        Ok((hasher.digest128(), total_size)) // Finalize and return the hash and size
    }
}
