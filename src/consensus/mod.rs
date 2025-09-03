pub mod difficulty;
pub mod pow;

// Re-export for easier access
pub use difficulty::{DifficultyManager, DifficultyParams, CompactDifficulty};
pub use pow::{PowSolution, PowMiner, PowValidator, PowParams, PowAlgorithm};

/// Consensus configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConsensusConfig {
    pub difficulty_params: DifficultyParams,
    pub pow_params: PowParams,
    pub max_block_size: u64,
    pub max_transaction_size: u64,
    pub coinbase_maturity: u64, // Number of blocks before coinbase can be spent
    pub max_future_block_time: u64, // Maximum allowed future block time in seconds
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        ConsensusConfig {
            difficulty_params: DifficultyParams::default(),
            pow_params: PowParams {
                algorithm: PowAlgorithm::Sha256d,
                version: 1,
                nonce_range: (0, u64::MAX),
            },
            max_block_size: 1_000_000, // 1MB
            max_transaction_size: 100_000, // 100KB
            coinbase_maturity: 100, // 100 blocks
            max_future_block_time: 7200, // 2 hours
        }
    }
}

/// Consensus manager that coordinates difficulty and PoW
#[derive(Debug)]
pub struct ConsensusManager {
    config: ConsensusConfig,
    difficulty_manager: DifficultyManager,
    current_height: u64,
    block_times: Vec<u64>,
}

impl ConsensusManager {
    /// Creates a new consensus manager
    pub fn new(config: ConsensusConfig, initial_difficulty: u64) -> Self {
        ConsensusManager {
            config: config.clone(),
            difficulty_manager: DifficultyManager::new(
                config.difficulty_params.clone(),
                initial_difficulty,
            ),
            current_height: 0,
            block_times: Vec::new(),
        }
    }

    /// Processes a new block
    pub fn process_block(&mut self, block_time: u64, block_height: u64) -> Result<u64, String> {
        if block_height != self.current_height + 1 {
            return Err("Block height must be consecutive".to_string());
        }

        self.block_times.push(block_time);
        self.current_height = block_height;

        // Adjust difficulty at the appropriate interval
        if self.current_height % self.config.difficulty_params.adjustment_interval == 0 {
            let new_difficulty = self.difficulty_manager.adjust_difficulty(
                self.current_height,
                &self.block_times,
            )?;

            // Keep only recent block times for next adjustment
            if self.block_times.len() > self.config.difficulty_params.adjustment_interval as usize {
                self.block_times = self.block_times
                    [self.block_times.len() - self.config.difficulty_params.adjustment_interval as usize..]
                    .to_vec();
            }

            Ok(new_difficulty)
        } else {
            Ok(self.difficulty_manager.get_difficulty())
        }
    }

    /// Validates a block's PoW solution
    pub fn validate_block_pow(
        &self,
        solution: &PowSolution,
        block_header: &[u8],
    ) -> Result<(), String> {
        let target = DifficultyManager::difficulty_to_target(solution.difficulty);

        if !PowValidator::validate(solution, block_header, &target) {
            return Err("Invalid PoW solution".to_string());
        }

        if !PowValidator::validate_timestamp(solution, self.config.max_future_block_time) {
            return Err("Solution timestamp too far in future".to_string());
        }

        Ok(())
    }

    /// Returns the current target for mining
    pub fn get_current_target(&self) -> [u8; 32] {
        DifficultyManager::difficulty_to_target(self.difficulty_manager.get_difficulty())
    }

    /// Returns the current difficulty
    pub fn get_current_difficulty(&self) -> u64 {
        self.difficulty_manager.get_difficulty()
    }

    /// Returns the network hashrate estimate
    pub fn estimate_network_hashrate(&self) -> f64 {
        if self.block_times.is_empty() {
            return 0.0;
        }

        let avg_block_time = self.block_times.iter().sum::<u64>() as f64 / self.block_times.len() as f64;
        self.difficulty_manager.estimate_network_hashrate(avg_block_time as u64)
    }

    /// Returns the consensus configuration
    pub fn get_config(&self) -> &ConsensusConfig {
        &self.config
    }

    /// Returns the current block height
    pub fn get_current_height(&self) -> u64 {
        self.current_height
    }

    /// Checks if a transaction is mature (coinbase check)
    pub fn is_transaction_mature(&self, transaction_height: u64) -> bool {
        self.current_height >= transaction_height + self.config.coinbase_maturity
    }

    /// Validates block size constraints
    pub fn validate_block_size(&self, block_size: u64) -> Result<(), String> {
        if block_size > self.config.max_block_size {
            return Err(format!("Block size {} exceeds maximum {}", block_size, self.config.max_block_size));
        }
        Ok(())
    }

    /// Validates transaction size constraints
    pub fn validate_transaction_size(&self, transaction_size: u64) -> Result<(), String> {
        if transaction_size > self.config.max_transaction_size {
            return Err(format!("Transaction size {} exceeds maximum {}", transaction_size, self.config.max_transaction_size));
        }
        Ok(())
    }
}

/// Consensus errors
#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Invalid proof of work: {0}")]
    InvalidProofOfWork(String),

    #[error("Block validation failed: {0}")]
    BlockValidation(String),

    #[error("Transaction validation failed: {0}")]
    TransactionValidation(String),

    #[error("Difficulty adjustment error: {0}")]
    DifficultyAdjustment(String),
}

#[derive(Debug)]
pub struct Miner {
    // TODO: Implement miner structure
}

impl Miner {
    pub fn new() -> Self {
        Miner {}
    }
}
