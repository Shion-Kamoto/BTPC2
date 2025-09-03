use serde::{Deserialize, Serialize};

/// Difficulty adjustment parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyParams {
    pub target_block_time: u64, // in seconds
    pub adjustment_interval: u64, // number of blocks between adjustments
    pub min_difficulty: u64,
    pub max_difficulty: u64,
    pub difficulty_precision: u32,
}

impl Default for DifficultyParams {
    fn default() -> Self {
        DifficultyParams {
            target_block_time: 600, // 10 minutes
            adjustment_interval: 2016, // Same as Bitcoin (2 weeks at 10-minute blocks)
            min_difficulty: 1,
            max_difficulty: u64::MAX,
            difficulty_precision: 16,
        }
    }
}

/// Difficulty calculation and adjustment
#[derive(Debug, Clone)]
pub struct DifficultyManager {
    params: DifficultyParams,
    current_difficulty: u64,
    last_adjustment_height: u64,
}

impl DifficultyManager {
    /// Creates a new difficulty manager with initial difficulty
    pub fn new(params: DifficultyParams, initial_difficulty: u64) -> Self {
        DifficultyManager {
            params,
            current_difficulty: initial_difficulty,
            last_adjustment_height: 0,
        }
    }

    /// Calculates the initial difficulty based on network hashrate estimate
    pub fn calculate_initial_difficulty(target_block_time: u64, network_hashrate: f64) -> u64 {
        // Difficulty = (network hashrate * target block time) / (2^32)
        let difficulty = (network_hashrate * target_block_time as f64) / (u32::MAX as f64);
        difficulty.max(1.0) as u64
    }

    /// Adjusts difficulty based on actual block times
    pub fn adjust_difficulty(
        &mut self,
        current_height: u64,
        previous_block_times: &[u64],
    ) -> Result<u64, String> {
        if current_height < self.last_adjustment_height + self.params.adjustment_interval {
            return Ok(self.current_difficulty);
        }

        if previous_block_times.len() < self.params.adjustment_interval as usize {
            return Err("Insufficient block time data for adjustment".to_string());
        }

        let actual_time: u64 = previous_block_times.iter().sum();
        let expected_time = self.params.target_block_time * self.params.adjustment_interval;

        let adjustment_factor = if actual_time == 0 {
            1.0
        } else {
            expected_time as f64 / actual_time as f64
        };

        let new_difficulty = (self.current_difficulty as f64 * adjustment_factor) as u64;

        // Apply bounds
        self.current_difficulty = new_difficulty
            .max(self.params.min_difficulty)
            .min(self.params.max_difficulty);

        self.last_adjustment_height = current_height;

        Ok(self.current_difficulty)
    }

    /// Returns the current difficulty
    pub fn get_difficulty(&self) -> u64 {
        self.current_difficulty
    }

    /// Calculates the target value from difficulty
    pub fn difficulty_to_target(difficulty: u64) -> [u8; 32] {
        if difficulty == 0 {
            return [0xFF; 32];
        }

        let target = (u64::MAX as f64 / difficulty as f64) as u64;
        let mut target_bytes = [0u8; 32];
        let target_be = target.to_be_bytes();

        // Copy to the end of the array (big-endian representation)
        target_bytes[32 - target_be.len()..].copy_from_slice(&target_be);
        target_bytes
    }

    /// Calculates difficulty from target value
    pub fn target_to_difficulty(target: &[u8; 32]) -> u64 {
        let target_value = u64::from_be_bytes([
            target[24], target[25], target[26], target[27],
            target[28], target[29], target[30], target[31],
        ]);

        if target_value == 0 {
            return u64::MAX;
        }

        (u64::MAX as f64 / target_value as f64) as u64
    }

    /// Calculates the network hashrate estimate
    pub fn estimate_network_hashrate(&self, actual_block_time: u64) -> f64 {
        if actual_block_time == 0 {
            return 0.0;
        }

        (self.current_difficulty as f64 * (u32::MAX as f64)) / actual_block_time as f64
    }

    /// Returns the expected time to mine a block at current difficulty
    pub fn expected_block_time(&self, miner_hashrate: f64) -> f64 {
        if miner_hashrate == 0.0 {
            return f64::INFINITY;
        }

        (self.current_difficulty as f64 * (u32::MAX as f64)) / miner_hashrate
    }

    /// Checks if a solution meets the target difficulty
    pub fn meets_difficulty(hash: &[u8; 32], target: &[u8; 32]) -> bool {
        hash <= target
    }

    /// Returns the difficulty adjustment parameters
    pub fn get_params(&self) -> &DifficultyParams {
        &self.params
    }
}

/// Compact difficulty representation (similar to Bitcoin's nBits)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CompactDifficulty(u32);

impl CompactDifficulty {
    /// Converts from compact representation to full difficulty
    pub fn to_difficulty(&self) -> u64 {
        let exponent = (self.0 >> 24) as u8;
        let mantissa = self.0 & 0x00FFFFFF;

        if exponent <= 3 {
            (mantissa >> (8 * (3 - exponent))) as u64
        } else {
            (mantissa as u64) << (8 * (exponent - 3))
        }
    }

    /// Converts from full difficulty to compact representation
    pub fn from_difficulty(difficulty: u64) -> Self {
        if difficulty == 0 {
            return CompactDifficulty(0);
        }

        let mut size = (difficulty.ilog2() / 8 + 1) as u8;
        let mut compact = if size <= 3 {
            (difficulty << (8 * (3 - size))) as u32
        } else {
            (difficulty >> (8 * (size - 3))) as u32
        };

        // The 0x00800000 bit denotes the sign, so if it is already set, divide the mantissa by 256
        // and increase the exponent
        if compact & 0x00800000 != 0 {
            compact >>= 8;
            size += 1;
        }

        CompactDifficulty(compact | (size as u32) << 24)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_adjustment() {
        let params = DifficultyParams::default();
        let mut manager = DifficultyManager::new(params, 1000);

        // Simulate faster blocks (should increase difficulty)
        let fast_times = vec![500; 2016]; // 500s blocks instead of 600s
        let result = manager.adjust_difficulty(2016, &fast_times);
        assert!(result.is_ok());
        assert!(manager.get_difficulty() > 1000);

        // Simulate slower blocks (should decrease difficulty)
        let slow_times = vec![1200; 2016]; // 1200s blocks
        let result = manager.adjust_difficulty(4032, &slow_times);
        assert!(result.is_ok());
        assert!(manager.get_difficulty() < 2000);
    }

    #[test]
    fn test_difficulty_target_conversion() {
        let difficulty = 1000;
        let target = DifficultyManager::difficulty_to_target(difficulty);
        let converted_diff = DifficultyManager::target_to_difficulty(&target);

        // Allow for some floating point precision loss
        assert!((converted_diff as i64 - difficulty as i64).abs() <= 1);
    }

    #[test]
    fn test_compact_difficulty() {
        let test_values = [1, 1000, 10000, 100000, 1000000, u64::MAX];

        for &value in &test_values {
            let compact = CompactDifficulty::from_difficulty(value);
            let recovered = compact.to_difficulty();

            // Compact representation may lose some precision for very large values
            if value < 1 << 24 {
                assert_eq!(recovered, value);
            }
        }
    }

    #[test]
    fn test_meets_difficulty() {
        let target = DifficultyManager::difficulty_to_target(1000);
        let low_hash = [0x00; 32]; // Meets difficulty
        let high_hash = [0xFF; 32]; // Doesn't meet difficulty

        assert!(DifficultyManager::meets_difficulty(&low_hash, &target));
        assert!(!DifficultyManager::meets_difficulty(&high_hash, &target));
    }
}