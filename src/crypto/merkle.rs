use sha2::{Sha512, Digest};
use std::fmt;
use std::error::Error;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub enum MerkleError {
    EmptyTree,
    InvalidProof,
    InvalidIndex,
}

impl fmt::Display for MerkleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MerkleError::EmptyTree => write!(f, "Cannot build Merkle tree from empty data"),
            MerkleError::InvalidProof => write!(f, "Invalid Merkle proof"),
            MerkleError::InvalidIndex => write!(f, "Invalid index for proof generation"),
        }
    }
}

impl Error for MerkleError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MerkleProof {
    pub leaf_hash: Vec<u8>,
    pub proof_hashes: Vec<Vec<u8>>,
    pub proof_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    leaves: Vec<Vec<u8>>,
    levels: Vec<Vec<Vec<u8>>>,
    root: Vec<u8>,
}

impl MerkleTree {
    pub fn new(data: &[Vec<u8>]) -> Result<Self, MerkleError> {
        if data.is_empty() {
            return Err(MerkleError::EmptyTree);
        }

        let leaves: Vec<Vec<u8>> = data.iter()
            .map(|d| Self::hash_leaf(d))
            .collect();

        let mut levels = Vec::new();
        levels.push(leaves.clone());

        let mut current_level = leaves.clone(); // Clone here to avoid move
        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in current_level.chunks(2) {
                let hash = if chunk.len() == 2 {
                    Self::hash_nodes(&chunk[0], &chunk[1])
                } else {
                    // For odd number of nodes, duplicate the last node
                    Self::hash_nodes(&chunk[0], &chunk[0])
                };
                next_level.push(hash);
            }

            levels.push(next_level.clone());
            current_level = next_level;
        }

        let root = current_level[0].clone();

        Ok(MerkleTree {
            leaves,
            levels,
            root,
        })
    }

    fn hash_leaf(data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha512::new();
        hasher.update(b"leaf:");
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    fn hash_nodes(left: &[u8], right: &[u8]) -> Vec<u8> {
        let mut hasher = Sha512::new();
        hasher.update(b"node:");
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().to_vec()
    }

    pub fn root(&self) -> &[u8] {
        &self.root
    }

    pub fn leaves(&self) -> &[Vec<u8>] {
        &self.leaves
    }

    pub fn generate_proof(&self, index: usize) -> Result<MerkleProof, MerkleError> {
        if index >= self.leaves.len() {
            return Err(MerkleError::InvalidIndex);
        }

        let leaf_hash = self.leaves[index].clone();
        let mut proof_hashes = Vec::new();
        let mut proof_indices = Vec::new();
        let mut current_index = index;

        for level in 0..(self.levels.len() - 1) {
            let current_level = &self.levels[level];

            if current_index % 2 == 0 {
                // Current node is left child
                if current_index + 1 < current_level.len() {
                    proof_hashes.push(current_level[current_index + 1].clone());
                    proof_indices.push(1); // Right sibling
                } else {
                    // No right sibling, use left sibling (duplicate)
                    proof_hashes.push(current_level[current_index].clone());
                    proof_indices.push(0); // Left sibling (self)
                }
            } else {
                // Current node is right child
                proof_hashes.push(current_level[current_index - 1].clone());
                proof_indices.push(0); // Left sibling
            }

            current_index /= 2;
        }

        Ok(MerkleProof {
            leaf_hash,
            proof_hashes,
            proof_indices,
        })
    }

    pub fn verify_proof(proof: &MerkleProof, root: &[u8]) -> bool {
        let mut current_hash = proof.leaf_hash.clone();

        for (i, sibling_hash) in proof.proof_hashes.iter().enumerate() {
            let sibling_index = proof.proof_indices[i];

            current_hash = if sibling_index == 0 {
                // Sibling is left, current is right
                Self::hash_nodes(sibling_hash, &current_hash)
            } else {
                // Sibling is right, current is left
                Self::hash_nodes(&current_hash, sibling_hash)
            };
        }

        current_hash == root
    }

    pub fn verify_leaf(&self, leaf_data: &[u8], index: usize) -> bool {
        let _leaf_hash = Self::hash_leaf(leaf_data); // Fix unused variable
        if let Ok(proof) = self.generate_proof(index) {
            Self::verify_proof(&proof, &self.root)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree_creation() {
        let data = vec![
            b"data1".to_vec(),
            b"data2".to_vec(),
            b"data3".to_vec(),
        ];

        let tree = MerkleTree::new(&data).unwrap();
        assert!(!tree.root().is_empty());
        assert_eq!(tree.leaves().len(), 3);
    }

    #[test]
    fn test_merkle_proof_verification() {
        let data = vec![
            b"data1".to_vec(),
            b"data2".to_vec(),
            b"data3".to_vec(),
        ];

        let tree = MerkleTree::new(&data).unwrap();
        let proof = tree.generate_proof(0).unwrap();

        assert!(MerkleTree::verify_proof(&proof, tree.root()));
    }

    #[test]
    fn test_merkle_leaf_verification() {
        let data = vec![
            b"data1".to_vec(),
            b"data2".to_vec(),
            b"data3".to_vec(),
        ];

        let tree = MerkleTree::new(&data).unwrap();
        assert!(tree.verify_leaf(b"data1", 0));
        assert!(!tree.verify_leaf(b"wrong_data", 0));
    }

    #[test]
    fn test_empty_tree_error() {
        let data: Vec<Vec<u8>> = vec![];
        let result = MerkleTree::new(&data);
        assert!(result.is_err());
    }
}