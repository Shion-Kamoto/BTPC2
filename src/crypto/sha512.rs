// file: src/crypto/sha512.rs
use sha2::{Sha512, Digest};
use rayon::prelude::*;

#[derive(Clone)]
pub struct DoubleSha512;

impl DoubleSha512 {
    /// Optimized double SHA512 for mining
    pub fn hash(data: &[u8]) -> [u8; 64] {
        let first = Sha512::digest(data);
        let second = Sha512::digest(&first);
        second.into()
    }

    /// Parallel batch hashing for improved performance
    pub fn hash_batch(data_chunks: &[&[u8]]) -> Vec<[u8; 64]> {
        data_chunks.par_iter().map(|chunk| Self::hash(chunk)).collect()
    }

    /// ASIC-resistant variant with Blake3 mixing
    pub fn hash_asic_resistant(data: &[u8]) -> [u8; 64] {
        let first = Sha512::digest(data);
        let blake_mixed = Blake3Hasher::new().update(&first).finalize();
        let second = Sha512::digest(blake_mixed.as_bytes());
        second.into()
    }
}