// file: src/blockchain/chain.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use rocksdb::{DB, Options};
use sha2::{Sha512, Digest};
use blake3::Hasher as Blake3Hasher;
use pqcrypto::sign::dilithium5; // Quantum-resistant signatures

use crate::{
    crypto::{DoubleSha512, MerkleTree},
    database::{Database, UTXOManager},
    consensus::{ProofOfWork, DifficultyManager},
    network::Broadcaster,
    models::{Block, Transaction, BlockHeader},
};

#[derive(Clone)]
pub struct QuantumResistantBlockchain {
    db: Arc<DB>,
    utxo_manager: UTXOManager,
    difficulty_manager: DifficultyManager,
    pow: ProofOfWork,
    broadcaster: Broadcaster,
    // ... other components
}

impl QuantumResistantBlockchain {
    pub async fn new(network: NetworkType) -> Result<Self, BlockchainError> {
        // Initialize database with optimized settings
        let db = Database::new(network).await?;
        let utxo_manager = UTXOManager::new(db.clone());
        let difficulty_manager = DifficultyManager::new(db.clone());
        let pow = ProofOfWork::new(Sha512Algorithm::new());
        let broadcaster = Broadcaster::new(network).await?;

        Ok(Self {
            db,
            utxo_manager,
            difficulty_manager,
            pow,
            broadcaster,
        })
    }

    /// Add and validate a new block with full production checks
    pub async fn add_block(&mut self, block: Block) -> Result<BlockAdded, BlockchainError> {
        // 1. Basic structural validation
        self.validate_block_structure(&block)?;

        // 2. Proof-of-Work validation (SHA512-based)
        let target = self.difficulty_manager.current_target()?;
        if !self.pow.validate_proof(&block.header, target) {
            return Err(BlockchainError::InvalidProof);
        }

        // 3. Transaction validation (quantum-resistant signatures)
        self.validate_transactions(&block.transactions).await?;

        // 4. UTXO validation
        let utxo_snapshot = self.utxo_manager.snapshot();
        let (total_fees, updated_utxos) = self.validate_utxo_changes(&block.transactions, utxo_snapshot).await?;

        // 5. Coinbase validation
        self.validate_coinbase(&block.transactions[0], total_fees, block.height)?;

        // 6. Persist to database
        self.persist_block(block, updated_utxos).await?;

        // 7. Broadcast to network
        self.broadcaster.broadcast_block(&block).await?;

        Ok(BlockAdded::ExtendedChain)
    }

    /// Quantum-resistant transaction validation
    async fn validate_transactions(&self, transactions: &[Transaction]) -> Result<(), BlockchainError> {
        for tx in transactions.iter().skip(1) { // Skip coinbase
            for input in &tx.inputs {
                // Verify Dilithium5 signatures (quantum-resistant)
                let pub_key = self.get_public_key_for_input(input).await?;
                let message = tx.signature_message();

                if let Err(e) = dilithium5::verify_detached(&input.signature, &message, &pub_key) {
                    return Err(BlockchainError::InvalidSignature(e));
                }
            }
        }
        Ok(())
    }

    /// Enhanced reward calculation with halving events
    pub fn calculate_block_reward(height: u64) -> u64 {
        const BLOCKS_PER_ERA: u64 = 210_000; // Bitcoin-like halving every 210k blocks
        const INITIAL_REWARD: u64 = 32_375_000_000; // 32.375 BTP in credits
        const FINAL_ERA: u64 = 24 * 52_560 / 210_000; // 24 years in eras

        let era = height / BLOCKS_PER_ERA;

        if era < FINAL_ERA {
            // Linear decay within each era
            let blocks_in_era = height % BLOCKS_PER_ERA;
            let era_reward = INITIAL_REWARD * (FINAL_ERA - era) / FINAL_ERA;
            let decay_per_block = era_reward / BLOCKS_PER_ERA;

            era_reward - (decay_per_block * blocks_in_era)
        } else {
            // Tail emission: 0.5 BTP
            50_000_000 // 0.5 BTP in credits
        }
    }
}