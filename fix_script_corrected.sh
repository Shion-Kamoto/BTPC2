#!/bin/bash

# Corrected fix script for Quantum Resistant Blockchain

set -e

echo "Fixing syntax errors and import paths..."

# 1. Fix the consensus mod.rs syntax errors
echo "Fixing consensus mod.rs syntax..."
cat > src/consensus/mod.rs << 'EOF'
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
EOF

# 2. Fix the UTXOSet struct syntax
echo "Fixing UTXOSet struct syntax..."
sed -i '53,57d' src/database/utxo_set.rs
sed -i '52a\\nimpl UTXOSet {' src/database/utxo_set.rs
sed -i '53a\    pub fn new(storage: Box<dyn UTXOStorage + Send + Sync>) -> Self {' src/database/utxo_set.rs
sed -i '54a\        UTXOSet {' src/database/utxo_set.rs
sed -i '55a\            storage: std::sync::Arc::new(std::sync::RwLock::new(storage)),' src/database/utxo_set.rs
sed -i '56a\        }' src/database/utxo_set.rs
sed -i '57a\    }' src/database/utxo_set.rs
sed -i '58a\}' src/database/utxo_set.rs

# 3. Fix import paths in network protocol
echo "Fixing network protocol imports..."
sed -i 's/use crate::crypto::utxo_set::{hash_transaction, OutPoint};/use crate::database::utxo_set::{hash_transaction, OutPoint};/g' src/network/protocol.rs

# 4. Fix MerkleTree import
sed -i 's/use crate::crypto::MerkleTree;/use crate::blockchain::merkle::MerkleTree;/g' src/network/protocol.rs

# 5. Fix database module imports
echo "Fixing database module imports..."
sed -i 's/use crate::crypto::utxo_set/use crate::database::utxo_set/g' src/database/mod.rs

# 6. Fix ed25519_dalek imports
echo "Fixing ed25519_dalek imports..."
cat > src/crypto/signatures.rs << 'EOF'
use ed25519_dalek::{Signer, Verifier, Signature, SigningKey, VerifyingKey, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH, SIGNATURE_LENGTH};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureData {
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub public_key: Vec<u8>,
}

impl SignatureData {
    pub fn new(signature: Vec<u8>, public_key: Vec<u8>) -> Self {
        SignatureData { signature, public_key }
    }
}

pub type PrivateKey = SigningKey;
pub type PublicKey = VerifyingKey;
pub type KeyPair = (SigningKey, VerifyingKey);
pub type SignatureError = ed25519_dalek::SignatureError;

pub fn sha512_hash(data: &[u8]) -> [u8; 64] {
    use sha2::{Sha512, Digest};
    let mut hasher = Sha512::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 64];
    hash.copy_from_slice(&result);
    hash
}

pub fn sha512_hash_string(data: &[u8]) -> String {
    hex::encode(sha512_hash(data))
}
EOF

# 7. Fix crypto module imports
echo "Fixing crypto module imports..."
cat > src/crypto/mod.rs << 'EOF'
pub mod signatures;
pub mod utxo_set;
pub mod protocol;
pub mod sync;

// Re-export commonly used items
pub use signatures::{PrivateKey, PublicKey, KeyPair, SignatureData, SignatureError, sha512_hash, sha512_hash_string};
// Note: utxo_set items are re-exported from database module
// Note: protocol items are re-exported from network module
// Note: sync items are re-exported from network module
EOF

# 8. Fix sync module imports
echo "Fixing sync module imports..."
sed -i 's/crypto::utxo_set::hash_transaction/database::utxo_set::hash_transaction/g' src/network/sync.rs
sed -i 's/use crate::crypto::utxo_set::UTXOSet;//g' src/network/sync.rs

# 9. Fix network protocol crypto imports
echo "Fixing network protocol crypto imports..."
sed -i 's/use crate::crypto::{PublicKey, SignatureData, sha512_hash, sha512_hash_string};/use crate::crypto::signatures::{PublicKey, SignatureData, sha512_hash, sha512_hash_string};/g' src/network/protocol.rs

# 10. Fix network sync crypto imports
echo "Fixing network sync crypto imports..."
sed -i 's/use crate::crypto::{sha512_hash, PublicKey};/use crate::crypto::signatures::{sha512_hash, PublicKey};/g' src/network/sync.rs

# 11. Fix network mod crypto imports
echo "Fixing network mod crypto imports..."
sed -i 's/use crate::crypto::KeyPair;/use crate::crypto::signatures::KeyPair;/g' src/network/mod.rs

# 12. Fix the serde attribute in sync.rs
echo "Fixing serde attribute in sync.rs..."
sed -i '73s/^.*$/    pub stop_hash: [u8; 64],/' src/network/sync.rs

# 13. Fix config.rs network usage
echo "Fixing config.rs network usage..."
sed -i 's/match config.network {/match config.network {/g' src/config.rs

# 14. Remove unused imports
echo "Removing unused imports..."
sed -i '/use sha2::{Sha512, Digest};/d' src/network/sync.rs
sed -i '/use Instant;/d' src/network/sync.rs
sed -i '/use std::net::SocketAddr;/d' src/network/sync.rs
sed -i '/use SystemTime;/d' src/consensus/difficulty.rs
sed -i '/use UNIX_EPOCH;/d' src/consensus/difficulty.rs
sed -i '/use Signer;/d' src/crypto/signatures.rs
sed -i '/use Verifier;/d' src/crypto/signatures.rs
sed -i '/use Signature;/d' src/crypto/signatures.rs
sed -i '/use PUBLIC_KEY_LENGTH;/d' src/crypto/signatures.rs
sed -i '/use SECRET_KEY_LENGTH;/d' src/crypto/signatures.rs
sed -i '/use SIGNATURE_LENGTH;/d' src/crypto/signatures.rs

# 15. Create basic protocol and sync modules
echo "Creating basic protocol module..."
cat > src/crypto/protocol.rs << 'EOF'
// Basic protocol implementation
#![allow(unused)]

pub struct ProtocolError;
pub struct PeerInfo;
pub struct NetworkMessage;
pub struct MessageBuilder;
pub const PROTOCOL_VERSION: u32 = 1;
EOF

echo "Creating basic sync module..."
cat > src/crypto/sync.rs << 'EOF'
// Basic sync implementation
#![allow(unused)]

pub struct SyncError;
pub struct SyncState;
pub struct SyncStatus;
pub struct SyncManager;
EOF

# 16. Create basic utxo_set module for crypto (minimal)
echo "Creating basic crypto utxo_set module..."
cat > src/crypto/utxo_set.rs << 'EOF'
// Minimal utxo_set for crypto module (re-exports from database)
#![allow(unused)]

pub use crate::database::utxo_set::{
    UTXOSet, UTXOStorage, UTXORecord, TxOutput, OutPoint,
    UTXOError, UTXOStats, hash_transaction, create_outpoint
};
EOF

echo "All fixes applied! Please run 'cargo build' to check for remaining issues."