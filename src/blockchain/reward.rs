use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

// ======================================================================
// Linear-decay economic model constants
// ======================================================================

/// 10-minute blocks (6 * 24 * 365)
const BLOCKS_PER_YEAR: u64 = 52_560;

/// 1 BTP = 100,000,000 base units
const COIN: u64 = 100_000_000;

/// 32.375 BTP initial block reward
const INITIAL_REWARD: f64 = 32.375;

/// 0.5 BTP tail emission reward
const FINAL_REWARD: f64 = 0.5;

/// 24 years decay period
const DECAY_PERIOD_YEARS: u64 = 24;

/// Total blocks in the decay period
const DECAY_PERIOD_BLOCKS: u64 = BLOCKS_PER_YEAR * DECAY_PERIOD_YEARS;

// ======================================================================
// Reward data model
// ======================================================================

/// Represents a reward in the blockchain system with the specified linear decay model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reward {
    pub recipient: String,
    /// Amount in base units (credits)
    pub amount: u64,
    pub timestamp: u64,
    pub reason: String,
    pub transaction_hash: Option<String>,
    pub block_height: u64,
}

impl Reward {
    /// Creates a new reward based on the block reward at the given height.
    pub fn new(recipient: String, block_height: u64, reason: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let reward_amount = calculate_block_reward(block_height as f64);

        Reward {
            recipient,
            amount: reward_amount,
            timestamp,
            reason,
            transaction_hash: None,
            block_height,
        }
    }

    /// Creates a new reward with a transaction hash.
    pub fn with_transaction_hash(
        recipient: String,
        block_height: u64,
        reason: String,
        transaction_hash: String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let reward_amount = calculate_block_reward(block_height as f64);

        Reward {
            recipient,
            amount: reward_amount,
            timestamp,
            reason,
            transaction_hash: Some(transaction_hash),
            block_height,
        }
    }

    /// Returns the reward amount in BTP (not base units).
    pub fn amount_in_btp(&self) -> f64 {
        self.amount as f64 / COIN as f64
    }

    /// Validates the reward.
    pub fn is_valid(&self) -> bool {
        if self.amount == 0 {
            return false;
        }

        if self.recipient.is_empty() {
            return false;
        }

        if self.block_height == 0 && self.amount != calculate_block_reward(0.0) {
            return false;
        }

        true
    }

    /// Returns the currency symbol.
    pub fn get_currency_symbol(&self) -> &str {
        "BTP"
    }

    /// Checks if the reward has a transaction hash.
    pub fn has_transaction_hash(&self) -> bool {
        self.transaction_hash.is_some()
    }

    /// Sets the transaction hash.
    pub fn set_transaction_hash(&mut self, hash: String) {
        self.transaction_hash = Some(hash);
    }

    /// Returns the age of the reward in seconds.
    pub fn get_age(&self) -> u64 {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        current_time - self.timestamp
    }

    /// Returns true if this is a tail emission reward (after decay period).
    pub fn is_tail_emission(&self) -> bool {
        self.block_height >= DECAY_PERIOD_BLOCKS
    }
}

/// Provide human-readable formatting; enables the standard `ToString` via `Display`.
impl fmt::Display for Reward {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Reward: {:.3} BTP to {} at block {} - {}",
            self.amount_in_btp(),
            self.recipient,
            self.block_height,
            self.reason
        )
    }
}

// ======================================================================
//
// Linear-decay reward calculation
//
// ======================================================================

/// Linear-decay block reward: 32.375 → 0.5 BTP over 24 years, then tail 0.5 BTP.
pub fn calculate_block_reward(block_height: f64) -> u64 {
    if block_height >= DECAY_PERIOD_BLOCKS as f64 {
        // Tail emission: constant 0.5 BTP after decay period
        return (FINAL_REWARD * COIN as f64) as u64;
    }

    // reward = initial - (initial - final) * (block_height / total_decay_blocks)
    let progress = block_height / DECAY_PERIOD_BLOCKS as f64;
    let reward_btp = INITIAL_REWARD - (INITIAL_REWARD - FINAL_REWARD) * progress;

    // Convert to base units and ensure we don't go below final reward due to floating point issues
    let reward_base_units = (reward_btp * COIN as f64) as u64;
    let min_reward = (FINAL_REWARD * COIN as f64) as u64;

    reward_base_units.max(min_reward)
}

/// Returns the total supply at a given block height.
pub fn calculate_total_supply(block_height: u64) -> u64 {
    if block_height == 0 {
        return calculate_block_reward(0.0);
    }

    let mut total_supply = 0u64;

    // Sum rewards from block 0 to block_height (inclusive)
    for height in 0..=block_height {
        total_supply += calculate_block_reward(height as f64);
    }

    total_supply
}

/// Returns the estimated annual inflation rate at a given block height.
pub fn calculate_inflation_rate(block_height: u64, current_supply: u64) -> f64 {
    if current_supply == 0 {
        return 0.0;
    }

    let annual_reward = calculate_block_reward(block_height as f64) * BLOCKS_PER_YEAR;
    (annual_reward as f64 / current_supply as f64) * 100.0
}

// ======================================================================
// Reward helpers
// ======================================================================

/// Reward types for the blockchain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RewardType {
    BlockReward,
    TransactionFee,
    StakingReward,
    GovernanceReward,
    Custom(String),
}

impl RewardType {
    /// Returns the default reason for the reward type.
    pub fn default_reason(&self) -> String {
        match self {
            RewardType::BlockReward => "Block mining reward".to_string(),
            RewardType::TransactionFee => "Transaction fee collection".to_string(),
            RewardType::StakingReward => "Staking reward".to_string(),
            RewardType::GovernanceReward => "Governance participation reward".to_string(),
            RewardType::Custom(reason) => reason.clone(),
        }
    }

    /// Creates a reward of this type.
    pub fn create_reward(&self, recipient: String, block_height: u64) -> Reward {
        Reward::new(recipient, block_height, self.default_reason())
    }
}

/// Parameters for reward calculation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RewardParameters {
    pub block_height: u64,
    pub transaction_fees: u64,
    pub total_staked: u64,
}

/// Reward calculator for the linear decay model.
pub struct LinearDecayRewardCalculator;

impl LinearDecayRewardCalculator {
    /// Calculates total reward for a block (block reward + fees).
    pub fn calculate_total_reward(params: &RewardParameters) -> u64 {
        let block_reward = calculate_block_reward(params.block_height as f64);
        block_reward + params.transaction_fees
    }

    /// Returns the remaining decay period in blocks.
    pub fn remaining_decay_blocks(block_height: u64) -> u64 {
        DECAY_PERIOD_BLOCKS.saturating_sub(block_height)
    }

    /// Returns the progress of decay as a percentage (0.0 to 100.0).
    pub fn decay_progress(block_height: u64) -> f64 {
        if block_height >= DECAY_PERIOD_BLOCKS {
            100.0
        } else {
            (block_height as f64 / DECAY_PERIOD_BLOCKS as f64) * 100.0
        }
    }
}

// ======================================================================
// Tests
// ======================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_block_reward() {
        // Test genesis block reward
        assert_eq!(
            calculate_block_reward(0.0),
            (INITIAL_REWARD * COIN as f64) as u64
        );

        // Test middle of decay period
        let middle_block = (DECAY_PERIOD_BLOCKS / 2) as f64;
        let expected_middle =
            (INITIAL_REWARD - (INITIAL_REWARD - FINAL_REWARD) * 0.5) * COIN as f64;
        assert_eq!(calculate_block_reward(middle_block), expected_middle as u64);

        // Test final decay block
        let final_block = (DECAY_PERIOD_BLOCKS - 1) as f64;
        let expected_final = (INITIAL_REWARD
            - (INITIAL_REWARD - FINAL_REWARD)
                * ((DECAY_PERIOD_BLOCKS - 1) as f64 / DECAY_PERIOD_BLOCKS as f64))
            * COIN as f64;
        assert_eq!(calculate_block_reward(final_block), expected_final as u64);

        // Test tail emission
        assert_eq!(
            calculate_block_reward(DECAY_PERIOD_BLOCKS as f64),
            (FINAL_REWARD * COIN as f64) as u64
        );
        assert_eq!(
            calculate_block_reward((DECAY_PERIOD_BLOCKS + 1000) as f64),
            (FINAL_REWARD * COIN as f64) as u64
        );
    }

    #[test]
    fn test_reward_creation() {
        let reward = Reward::new("miner123".to_string(), 1000, "Block mining".to_string());

        assert_eq!(reward.recipient, "miner123");
        assert_eq!(reward.block_height, 1000);
        assert_eq!(reward.amount, calculate_block_reward(1000.0));
        assert!(reward.is_valid());

        // Display / ToString path
        let s = reward.to_string();
        assert!(s.contains("Reward:"));
        assert!(s.contains("miner123"));
    }

    #[test]
    fn test_total_supply_calculation() {
        // Test early supply
        let early_supply = calculate_total_supply(10);
        assert!(early_supply > 0);

        // Test that supply increases
        let later_supply = calculate_total_supply(20);
        assert!(later_supply > early_supply);
    }

    #[test]
    fn test_tail_emission_detection() {
        let pre_tail_reward = Reward::new(
            "test".to_string(),
            DECAY_PERIOD_BLOCKS - 1,
            "test".to_string(),
        );
        let tail_reward = Reward::new("test".to_string(), DECAY_PERIOD_BLOCKS, "test".to_string());

        assert!(!pre_tail_reward.is_tail_emission());
        assert!(tail_reward.is_tail_emission());
    }

    #[test]
    fn test_decay_progress() {
        assert_eq!(LinearDecayRewardCalculator::decay_progress(0), 0.0);
        assert_eq!(
            LinearDecayRewardCalculator::decay_progress(DECAY_PERIOD_BLOCKS / 2),
            50.0
        );
        assert_eq!(
            LinearDecayRewardCalculator::decay_progress(DECAY_PERIOD_BLOCKS),
            100.0
        );
        assert_eq!(
            LinearDecayRewardCalculator::decay_progress(DECAY_PERIOD_BLOCKS + 1000),
            100.0
        );
    }

    #[test]
    fn test_reward_amount_in_btp() {
        let reward = Reward::new("test".to_string(), 0, "test".to_string());
        assert_eq!(reward.amount_in_btp(), INITIAL_REWARD);

        let tail_reward = Reward::new("test".to_string(), DECAY_PERIOD_BLOCKS, "test".to_string());
        assert_eq!(tail_reward.amount_in_btp(), FINAL_REWARD);
    }
}
