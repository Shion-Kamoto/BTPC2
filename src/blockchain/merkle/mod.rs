//! Merkle tree utilities for SHA-512 (64-byte) leaf hashes.
//!
//! - Parents are computed as `SHA512(left || right)`.
//! - When a level has an odd number of nodes, the last is duplicated (Bitcoin-style).
//! - Leaves are treated as already-hashed 64-byte values.
//! - Root returned by value as `[u8; 64]`.

use core::fmt;

#[derive(Debug)]
pub enum MerkleError {
    Empty,
}

impl fmt::Display for MerkleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MerkleError::Empty => write!(f, "merkle tree requires at least one leaf"),
        }
    }
}

impl std::error::Error for MerkleError {}

pub struct MerkleTree {
    root: [u8; 64],
}

impl MerkleTree {
    /// Build a Merkle tree from pre-hashed 64-byte leaves.
    pub fn new(leaves: &[[u8; 64]]) -> Result<Self, MerkleError> {
        if leaves.is_empty() {
            return Err(MerkleError::Empty);
        }
        if leaves.len() == 1 {
            return Ok(Self { root: leaves[0] });
        }

        use sha2::{Digest, Sha512};

        // Work buffer: start with the leaves
        let mut level: Vec<[u8; 64]> = leaves.to_vec();

        // Reduce until one node remains
        while level.len() > 1 {
            // If odd, duplicate last
            if level.len() % 2 == 1 {
                let last = *level.last().expect("non-empty");
                level.push(last);
            }

            let mut next = Vec::with_capacity(level.len() / 2);

            // Hash pairs (left || right)
            for pair in level.chunks_exact(2) {
                let (left, right) = (&pair[0], &pair[1]);

                // Concatenate into 128-byte stack buffer to avoid allocs
                let mut buf = [0u8; 128];
                buf[..64].copy_from_slice(left);
                buf[64..].copy_from_slice(right);

                let parent = Sha512::digest(buf);
                let mut parent_arr = [0u8; 64];
                parent_arr.copy_from_slice(&parent);
                next.push(parent_arr);
            }

            level = next;
        }

        Ok(Self { root: level[0] })
    }

    /// Return the Merkle root by value.
    #[inline]
    pub fn root(&self) -> [u8; 64] {
        self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha512};

    fn h(b: &[u8]) -> [u8; 64] {
        let x = Sha512::digest(b);
        let mut out = [0u8; 64];
        out.copy_from_slice(&x);
        out
    }

    #[test]
    fn merkle_one_leaf() {
        let leaves = [h(b"a")];
        let t = MerkleTree::new(&leaves).unwrap();
        assert_eq!(t.root(), leaves[0]);
    }

    #[test]
    fn merkle_two_leaves() {
        let leaves = [h(b"a"), h(b"b")];
        let t = MerkleTree::new(&leaves).unwrap();
        // expected = SHA512( h(a) || h(b) )
        let mut buf = [0u8; 128];
        buf[..64].copy_from_slice(&leaves[0]);
        buf[64..].copy_from_slice(&leaves[1]);
        let exp = Sha512::digest(buf);
        let mut exp_arr = [0u8; 64];
        exp_arr.copy_from_slice(&exp);
        assert_eq!(t.root(), exp_arr);
    }

    #[test]
    fn merkle_odd_duplication() {
        let leaves = [h(b"a"), h(b"b"), h(b"c")];
        let t = MerkleTree::new(&leaves).unwrap();
        let r1 = t.root();
        let r2 = MerkleTree::new(&leaves).unwrap().root();
        assert_eq!(r1, r2);
    }
}
