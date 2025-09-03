#!/bin/bash

# Comprehensive fix script for Quantum Resistant Blockchain

set -e

echo "Fixing serde attribute syntax and missing implementations..."

# 1. Fix serde attribute syntax - the sed command was wrong
echo "Fixing serde attribute syntax..."
find src -name "*.rs" -exec sed -i 's/#\[serde(with = "serde_bytes")\] //g' {} \;
find src -name "*.rs" -exec sed -i 's/pub \([a-zA-Z_][a-zA-Z0-9_]*\): \[u8; 64\],/pub \1: [u8; 64],/g' {} \;

# 2. Remove the manual Serialize/Deserialize implementations (orphan rule violations)
echo "Removing orphan rule implementations..."
sed -i '/impl Serialize for \[u8; 64\] {/,/^}$/d' src/network/protocol.rs
sed -i '/impl.*Deserialize.*for \[u8; 64\] {/,/^}$/d' src/network/protocol.rs

# 3. Fix the actual serde_bytes usage with proper attribute syntax
echo "Adding proper serde_bytes attributes..."
sed -i 's/pub checksum: \[u8; 64\],/    #[serde(with = "serde_bytes")]\n    pub checksum: [u8; 64],/g' src/network/protocol.rs
sed -i 's/pub hash: \[u8; 64\],/    #[serde(with = "serde_bytes")]\n    pub hash: [u8; 64],/g' src/network/protocol.rs
sed -i 's/pub prev_block_hash: \[u8; 64\],/    #[serde(with = "serde_bytes")]\n    pub prev_block_hash: [u8; 64],/g' src/network/protocol.rs
sed -i 's/pub merkle_root: \[u8; 64\],/    #[serde(with = "serde_bytes")]\n    pub merkle_root: [u8; 64],/g' src/network/protocol.rs
sed -i 's/pub hash_stop: \[u8; 64\],/    #[serde(with = "serde_bytes")]\n    pub hash_stop: [u8; 64],/g' src/network/protocol.rs
sed -i 's/pub stop_hash: \[u8; 64\],/    #[serde(with = "serde_bytes")]\n    pub stop_hash: [u8; 64],/g' src/network/sync.rs

# 4. Fix database imports and missing functions
echo "Fixing database imports..."
cat > src/database/utxo_set.rs << 'EOF'
use serde::{Deserialize, Serialize};
use serde_bytes;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct OutPoint {
    #[serde(with = "serde_bytes")]
    pub tx_hash: [u8; 64],
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxOutput {
    pub value: u64,
    pub script_pubkey: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXORecord {
    pub outpoint: OutPoint,
    pub output: TxOutput,
    pub block_height: u64,
    pub is_coinbase: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXOStats {
    pub total_outputs: u64,
    pub total_value: u64,
    pub unspent_outputs: u64,
    pub unspent_value: u64,
}

#[derive(Debug)]
pub enum UTXOError {
    SerializationError(String),
    NotFound,
    AlreadySpent,
    InvalidInput,
}

pub trait UTXOStorage: fmt::Debug + Send + Sync {
    fn add_output(&mut self, outpoint: OutPoint, output: TxOutput, block_height: u64, is_coinbase: bool) -> Result<(), UTXOError>;
    fn spend_output(&mut self, outpoint: &OutPoint, spending_tx_hash: [u8; 64]) -> Result<(), UTXOError>;
    fn get_output(&self, outpoint: &OutPoint) -> Result<Option<(TxOutput, u64, bool)>, UTXOError>;
    fn get_unspent_outputs(&self) -> Result<Vec<UTXORecord>, UTXOError>;
    fn get_stats(&self) -> Result<UTXOStats, UTXOError>;
    fn clear(&mut self) -> Result<(), UTXOError>;
}

#[derive(Debug, Clone)]
pub struct UTXOSet {
    storage: std::sync::Arc<std::sync::RwLock<Box<dyn UTXOStorage + Send + Sync>>>,
}

impl UTXOSet {
    pub fn new(storage: Box<dyn UTXOStorage + Send + Sync>) -> Self {
        UTXOSet {
            storage: std::sync::Arc::new(std::sync::RwLock::new(storage)),
        }
    }

    pub fn add_outputs(&self, tx_hash: [u8; 64], outputs: Vec<(u32, TxOutput)>, block_height: u64, is_coinbase: bool) -> Result<(), UTXOError> {
        let mut storage = self.storage.write().unwrap();
        for (index, output) in outputs {
            let outpoint = OutPoint { tx_hash, index };
            storage.add_output(outpoint, output, block_height, is_coinbase)?;
        }
        Ok(())
    }

    pub fn spend_outputs(&self, inputs: &[OutPoint], spending_tx_hash: [u8; 64]) -> Result<(), UTXOError> {
        let mut storage = self.storage.write().unwrap();
        for input in inputs {
            storage.spend_output(input, spending_tx_hash)?;
        }
        Ok(())
    }

    pub fn get_unspent_outputs(&self) -> Result<Vec<UTXORecord>, UTXOError> {
        self.storage.read().unwrap().get_unspent_outputs()
    }

    pub fn get_stats(&self) -> Result<UTXOStats, UTXOError> {
        self.storage.read().unwrap().get_stats()
    }

    pub fn clear(&self) -> Result<(), UTXOError> {
        self.storage.write().unwrap().clear()
    }
}

pub fn hash_transaction(tx_data: &[u8]) -> [u8; 64] {
    use sha2::{Sha512, Digest};
    let mut hasher = Sha512::new();
    hasher.update(tx_data);
    let result = hasher.finalize();
    let mut hash = [0u8; 64];
    hash.copy_from_slice(&result);
    hash
}

pub fn create_outpoint(tx_hash: [u8; 64], index: u32) -> OutPoint {
    OutPoint { tx_hash, index }
}
EOF

# 5. Fix missing config field in ConsensusManager
echo "Fixing ConsensusManager config field..."
sed -i '/pub struct ConsensusManager {/a\    config: ConsensusConfig,' src/consensus/mod.rs

# 6. Fix reward amount type mismatch
echo "Fixing reward amount type mismatch..."
sed -i 's/pub amount: u64,/pub amount: f64,/g' src/blockchain/reward.rs
sed -i 's/100.0/100/g' src/blockchain/block.rs

# 7. Fix crypto module imports
echo "Fixing crypto module imports..."
cat > src/crypto/mod.rs << 'EOF'
pub mod signatures;
pub mod utxo_set;
pub mod protocol;
pub mod sync;

// Re-export commonly used items
pub use signatures::{PrivateKey, PublicKey, KeyPair, SignatureData, SignatureError, sha512_hash, sha512_hash_string};
pub use utxo_set::{UTXOSet, UTXOStorage, UTXORecord, TxOutput, OutPoint, UTXOError, UTXOStats, hash_transaction, create_outpoint};
pub use protocol::{ProtocolError, PeerInfo, NetworkMessage, MessageBuilder, PROTOCOL_VERSION};
pub use sync::{SyncError, SyncState, SyncStatus, SyncManager};
EOF

# 8. Fix network protocol imports
echo "Fixing network protocol imports..."
sed -i 's/use crate::database::{hash_transaction, OutPoint};/use crate::crypto::utxo_set::{hash_transaction, OutPoint};/g' src/network/protocol.rs

# 9. Fix sync module imports
echo "Fixing sync module imports..."
sed -i 's/database::hash_transaction/crypto::utxo_set::hash_transaction/g' src/network/sync.rs
sed -i 's/use crate::database::{UTXOSet, DatabaseManager};/use crate::crypto::utxo_set::UTXOSet;\nuse crate::database::DatabaseManager;/g' src/network/sync.rs

# 10. Fix database module
echo "Fixing database module..."
cat > src/database/mod.rs << 'EOF'
pub mod utxo_set;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::crypto::utxo_set::{UTXOSet, UTXOStorage, UTXORecord, TxOutput, OutPoint, UTXOError, UTXOStats, hash_transaction, create_outpoint};

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub data_dir: PathBuf,
    pub max_cache_size: usize,
}

#[derive(Debug, Clone)]
pub struct DatabaseManager {
    pub utxo_set: UTXOSet,
    pub config: DatabaseConfig,
}

impl DatabaseManager {
    pub fn new(storage: Box<dyn UTXOStorage + Send + Sync>, config: DatabaseConfig) -> Self {
        DatabaseManager {
            utxo_set: UTXOSet::new(storage),
            config,
        }
    }

    pub fn get_utxo_stats(&self) -> Result<UTXOStats, UTXOError> {
        self.utxo_set.get_stats()
    }

    pub fn clear_all(&self) -> Result<(), UTXOError> {
        self.utxo_set.clear()
    }

    pub fn serialize_with_checksum<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, UTXOError> {
        use bincode;
        let serialized = bincode::serialize(value)
            .map_err(|e| UTXOError::SerializationError(e.to_string()))?;

        let checksum = hash_transaction(&serialized);
        let mut result = Vec::with_capacity(serialized.len() + 64);
        result.extend_from_slice(&serialized);
        result.extend_from_slice(&checksum);
        Ok(result)
    }

    pub fn deserialize_with_checksum<T: serde::de::DeserializeOwned>(data: &[u8]) -> Result<T, UTXOError> {
        use bincode;
        if data.len() < 64 {
            return Err(UTXOError::SerializationError("Data too short for checksum".into()));
        }

        let (payload, checksum) = data.split_at(data.len() - 64);
        let expected_checksum = hash_transaction(payload);

        if checksum != expected_checksum {
            return Err(UTXOError::SerializationError("Checksum verification failed".into()));
        }

        bincode::deserialize(payload)
            .map_err(|e| UTXOError::SerializationError(e.to_string()))
    }
}

#[derive(Debug)]
pub struct Database;

impl Database {
    pub fn new() -> Self {
        Database
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseTransaction {
    pub inputs: Vec<OutPoint>,
    pub outputs: Vec<TxOutput>,
    pub tx_hash: [u8; 64],
}

impl DatabaseTransaction {
    pub fn execute(self, db_manager: &DatabaseManager) -> Result<(), UTXOError> {
        if !self.inputs.is_empty() {
            db_manager.utxo_set.spend_outputs(&self.inputs, self.tx_hash)?;
        }
        if !self.outputs.is_empty() {
            let outputs_with_index = self.outputs.into_iter().enumerate()
                .map(|(index, output)| (index as u32, output))
                .collect();
            db_manager.utxo_set.add_outputs(self.tx_hash, outputs_with_index, 0, false)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSnapshot {
    pub unspent_outputs: Vec<UTXORecord>,
    pub stats: UTXOStats,
}

impl DatabaseSnapshot {
    pub fn create(db_manager: &DatabaseManager) -> Result<DatabaseSnapshot, UTXOError> {
        let unspent_outputs = db_manager.utxo_set.get_unspent_outputs()?;
        let stats = db_manager.utxo_set.get_stats()?;
        Ok(DatabaseSnapshot { unspent_outputs, stats })
    }
}
EOF

# 11. Fix MemoryUTXOStorage implementation
echo "Adding MemoryUTXOStorage implementation..."
cat >> src/database/utxo_set.rs << 'EOF'

#[derive(Debug)]
pub struct MemoryUTXOStorage {
    outputs: std::collections::HashMap<OutPoint, (TxOutput, u64, bool, Option<[u8; 64]>)>,
}

impl MemoryUTXOStorage {
    pub fn new() -> Self {
        MemoryUTXOStorage {
            outputs: std::collections::HashMap::new(),
        }
    }
}

impl UTXOStorage for MemoryUTXOStorage {
    fn add_output(&mut self, outpoint: OutPoint, output: TxOutput, block_height: u64, is_coinbase: bool) -> Result<(), UTXOError> {
        self.outputs.insert(outpoint, (output, block_height, is_coinbase, None));
        Ok(())
    }

    fn spend_output(&mut self, outpoint: &OutPoint, spending_tx_hash: [u8; 64]) -> Result<(), UTXOError> {
        if let Some(entry) = self.outputs.get_mut(outpoint) {
            if entry.3.is_some() {
                return Err(UTXOError::AlreadySpent);
            }
            entry.3 = Some(spending_tx_hash);
            Ok(())
        } else {
            Err(UTXOError::NotFound)
        }
    }

    fn get_output(&self, outpoint: &OutPoint) -> Result<Option<(TxOutput, u64, bool)>, UTXOError> {
        Ok(self.outputs.get(outpoint).map(|(output, height, is_cb, spent)| {
            (output.clone(), *height, *is_cb)
        }))
    }

    fn get_unspent_outputs(&self) -> Result<Vec<UTXORecord>, UTXOError> {
        let mut results = Vec::new();
        for (outpoint, (output, height, is_coinbase, spent)) in &self.outputs {
            if spent.is_none() {
                results.push(UTXORecord {
                    outpoint: outpoint.clone(),
                    output: output.clone(),
                    block_height: *height,
                    is_coinbase: *is_coinbase,
                });
            }
        }
        Ok(results)
    }

    fn get_stats(&self) -> Result<UTXOStats, UTXOError> {
        let mut stats = UTXOStats {
            total_outputs: self.outputs.len() as u64,
            total_value: 0,
            unspent_outputs: 0,
            unspent_value: 0,
        };

        for (output, height, is_coinbase, spent) in self.outputs.values() {
            stats.total_value += output.value;
            if spent.is_none() {
                stats.unspent_outputs += 1;
                stats.unspent_value += output.value;
            }
        }

        Ok(stats)
    }

    fn clear(&mut self) -> Result<(), UTXOError> {
        self.outputs.clear();
        Ok(())
    }
}
EOF

# 12. Fix the config.rs network usage
echo "Fixing config.rs network usage..."
sed -i 's/network.default_port()/config.network.default_port()/g' src/config.rs

# 13. Add missing new method to UTXOSet
echo "Adding new method to UTXOSet..."
sed -i '/pub struct UTXOSet {/a\    storage: std::sync::Arc<std::sync::RwLock<Box<dyn UTXOStorage + Send + Sync>>>,' src/database/utxo_set.rs
sed -i '/pub struct UTXOSet {/a\impl UTXOSet {\n    pub fn new(storage: Box<dyn UTXOStorage + Send + Sync>) -> Self {\n        UTXOSet {\n            storage: std::sync::Arc::new(std::sync::RwLock::new(storage)),\n        }\n    }\n}' src/database/utxo_set.rs

# 14. Fix the consensus manager initialization
echo "Fixing consensus manager initialization..."
sed -i 's/ConsensusManager {/ConsensusManager {\n            config: config.clone(),/g' src/consensus/mod.rs

# 15. Remove unused imports
echo "Removing unused imports..."
sed -i '/use serde_bytes;/d' src/blockchain/block.rs
sed -i '/use serde_bytes;/d' src/blockchain/reward.rs
sed -i '/use serde_bytes;/d' src/consensus/difficulty.rs
sed -i '/use serde_bytes;/d' src/consensus/pow.rs

echo "All fixes applied! Please run 'cargo build' to check for remaining issues."