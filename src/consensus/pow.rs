use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use crate::consensus::difficulty::{DifficultyManager, CompactDifficulty};
use std::time::UNIX_EPOCH;
use std::time::SystemTime;

/// Proof of Work algorithm parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowParams {
    pub algorithm: PowAlgorithm,
    pub version: u32,
    pub nonce_range: (u64, u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PowAlgorithm {
    Sha256d, // Double SHA256 (Bitcoin-style)
    RandomX, // RandomX (Monero-style)
    Ethash,  // Ethash (Ethereum-style)
    Custom(String),
}

/// Proof of Work solution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowSolution {
    pub nonce: u64,
    pub hash: [u8; 32],
    pub difficulty: u64,
    pub timestamp: u64,
    pub extra_nonce: Option<u64>,
}

impl PowSolution {
    /// Creates a new PoW solution
    pub fn new(nonce: u64, hash: [u8; 32], difficulty: u64, extra_nonce: Option<u64>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        PowSolution {
            nonce,
            hash,
            difficulty,
            timestamp,
            extra_nonce,
        }
    }

    /// Validates the PoW solution
    pub fn is_valid(&self, block_header: &[u8], target: &[u8; 32]) -> bool {
        // Verify the hash meets the target difficulty
        if !DifficultyManager::meets_difficulty(&self.hash, target) {
            return false;
        }

        // Verify the hash is actually derived from the block header
        let computed_hash = Self::compute_hash(block_header, self.nonce, self.extra_nonce);
        computed_hash == self.hash
    }

    /// Computes the hash for given block header and nonce
    pub fn compute_hash(block_header: &[u8], nonce: u64, extra_nonce: Option<u64>) -> [u8; 32] {
        let mut hasher = Sha256::new();

        // Include block header
        hasher.update(block_header);

        // Include nonce
        hasher.update(nonce.to_be_bytes());

        // Include extra nonce if present
        if let Some(extra) = extra_nonce {
            hasher.update(extra.to_be_bytes());
        }

        // First hash
        let first_hash = hasher.finalize();

        // Second hash (double SHA256)
        let mut hasher = Sha256::new();
        hasher.update(first_hash);
        let result = hasher.finalize();

        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&result);
        hash_bytes
    }

    /// Returns the solution's hash rate estimate
    pub fn estimate_hashrate(&self, block_time: u64) -> f64 {
        if block_time == 0 {
            return 0.0;
        }

        // hashrate = (difficulty * 2^32) / block_time
        (self.difficulty as f64 * (u32::MAX as f64)) / block_time as f64
    }

    /// Converts to compact representation
    pub fn to_compact(&self) -> CompactDifficulty {
        CompactDifficulty::from_difficulty(self.difficulty)
    }

    /// Returns the solution score (higher is better)
    pub fn score(&self) -> f64 {
        // Lower difficulty solutions have higher scores
        1.0 / self.difficulty as f64
    }
}

/// Proof of Work miner
pub struct PowMiner {
    params: PowParams,
    current_nonce: u64,
    extra_nonce: u64,
    hashes_computed: u64,
    start_time: u64,
}

impl PowMiner {
    /// Creates a new PoW miner
    pub fn new(params: PowParams) -> Self {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        PowMiner {
            params,
            current_nonce: 0,
            extra_nonce: 0,
            hashes_computed: 0,
            start_time,
        }
    }

    /// Mines for a solution
    pub fn mine(&mut self, block_header: &[u8], target: &[u8; 32]) -> Option<PowSolution> {
        let (start_nonce, end_nonce) = self.params.nonce_range;

        for nonce in start_nonce..=end_nonce {
            self.current_nonce = nonce;
            self.hashes_computed += 1;

            let hash = PowSolution::compute_hash(block_header, nonce, Some(self.extra_nonce));

            if DifficultyManager::meets_difficulty(&hash, target) {
                let solution = PowSolution::new(
                    nonce,
                    hash,
                    DifficultyManager::target_to_difficulty(target),
                    Some(self.extra_nonce),
                );
                return Some(solution);
            }

            // Increment extra_nonce when nonce range is exhausted
            if nonce == end_nonce {
                self.extra_nonce += 1;
                self.current_nonce = start_nonce;
            }
        }

        None
    }

    /// Returns the current hash rate
    pub fn current_hashrate(&self) -> f64 {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let elapsed = current_time - self.start_time;
        if elapsed == 0 {
            return 0.0;
        }

        self.hashes_computed as f64 / elapsed as f64
    }

    /// Returns the estimated time to find a solution
    pub fn estimated_time_to_solution(&self, difficulty: u64) -> f64 {
        let hashrate = self.current_hashrate();
        if hashrate == 0.0 {
            return f64::INFINITY;
        }

        (difficulty as f64 * (u32::MAX as f64)) / hashrate
    }

    /// Resets the miner statistics
    pub fn reset(&mut self) {
        self.hashes_computed = 0;
        self.start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
    }
}

/// Proof of Work validator
pub struct PowValidator;

impl PowValidator {
    /// Validates a PoW solution against a block
    pub fn validate(solution: &PowSolution, block_header: &[u8], target: &[u8; 32]) -> bool {
        solution.is_valid(block_header, target)
    }

    /// Validates multiple solutions and returns the best one
    pub fn choose_best_solution(solutions: &[PowSolution]) -> Option<&PowSolution> {
        solutions.iter().max_by(|a, b| {
            a.score().partial_cmp(&b.score()).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Verifies the solution's timestamp is reasonable
    pub fn validate_timestamp(solution: &PowSolution, max_age: u64) -> bool {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        current_time - solution.timestamp <= max_age
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pow_computation() {
        let block_header = b"test block header";
        let nonce = 12345;

        let hash = PowSolution::compute_hash(block_header, nonce, None);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_solution_validation() {
        let block_header = b"test block header";
        let target = DifficultyManager::difficulty_to_target(1000); // Easy target

        let mut miner = PowMiner::new(PowParams {
            algorithm: PowAlgorithm::Sha256d,
            version: 1,
            nonce_range: (0, 100000),
        });

        let solution = miner.mine(block_header, &target);
        assert!(solution.is_some());

        let solution = solution.unwrap();
        assert!(PowValidator::validate(&solution, block_header, &target));
    }

    #[test]
    fn test_best_solution_selection() {
        let solutions = vec![
            PowSolution::new(1, [0x01; 32], 1000, None), // Lower difficulty
            PowSolution::new(2, [0x00; 32], 2000, None), // Higher difficulty (better)
        ];

        let best = PowValidator::choose_best_solution(&solutions);
        assert!(best.is_some());
        assert_eq!(best.unwrap().difficulty, 2000);
    }
}