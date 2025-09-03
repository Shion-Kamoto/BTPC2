use std::fmt;

#[derive(Debug)]
pub enum MerkleError {
    EmptyInput,
    InvalidInput,
    // ... other variants
}

impl fmt::Display for MerkleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MerkleError::EmptyInput => write!(f, "Empty input provided to Merkle tree"),
            MerkleError::InvalidInput => write!(f, "Invalid input provided to Merkle tree"),
            // ... other variants
        }
    }
}

impl std::error::Error for MerkleError {}
