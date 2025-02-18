use crate::core::common::Digest;

/// Represents a single commit.
#[derive(Debug, Clone)]
pub struct Commit {
    // Identifier of a branch.
    pub branch: u32,
    /// The sequential number of the commit.
    pub seq: u64,
    /// The hash of the file tree associated with the commit.
    pub treehash: Digest,
    /// The commit message.
    pub message: String,
}

impl Commit {
    /// Creates a new Commit instance.
    pub fn new(branch: u32, seq: u64, treehash: [u8; 16], message: String) -> Self {
        Commit {
            branch,
            seq,
            treehash,
            message,
        }
    }
    /// Returns a formatted string of the commit information.
    pub fn summary(&self) -> String {
        format!("Commit {}\n{}", self.seq, self.message)
    }
}
