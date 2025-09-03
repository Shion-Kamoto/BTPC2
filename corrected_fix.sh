#!/bin/bash

# Corrected fix script for parameter syntax and import issues

set -e

echo "Fixing parameter syntax and import issues..."

# 1. Fix the corrupted parameter names in network files
echo "Fixing corrupted parameter names..."

# network/mod.rs
sed -i 's/_peer: peer: &PeerInfoPeerInfo/_peer: \&PeerInfo/g' src/network/mod.rs
sed -i 's/_message: NetworkMessage/_message: NetworkMessage/g' src/network/mod.rs

# network/sync.rs
sed -i 's/_peer_id: peer_id: &strstr/_peer_id: \&str/g' src/network/sync.rs
sed -i 's/_peer: peer: &PeerInfoPeerInfo/_peer: \&PeerInfo/g' src/network/sync.rs
sed -i 's/_locator: locator: &BlockLocatorBlockLocator/_locator: \&BlockLocator/g' src/network/sync.rs
sed -i 's/_block_hash: block_hash: &\[u8; 64\]\[u8; 64\]/_block_hash: \&[u8; 64]/g' src/network/sync.rs

# 2. Fix duplicate merkle module
echo "Fixing duplicate merkle module..."
sed -i '/pub mod merkle;/d' src/blockchain/mod.rs
echo "pub mod merkle;" >> src/blockchain/mod.rs
echo "pub mod block;" >> src/blockchain/mod.rs
echo "pub mod reward;" >> src/blockchain/mod.rs

# 3. Fix ed25519_dalek imports
echo "Fixing ed25519_dalek imports..."
cat > src/crypto/signatures.rs << 'EOF'
use ed25519_dalek::{Signer, Signature, SigningKey, VerifyingKey, SignatureError};
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

    pub fn verify(&self, message: &[u8]) -> Result<(), SignatureError> {
        use ed25519_dalek::Verifier;
        let public_key = VerifyingKey::from_bytes(&self.public_key.try_into().map_err(|_| SignatureError::new())?)?;
        let signature = Signature::from_bytes(&self.signature.try_into().map_err(|_| SignatureError::new())?);
        public_key.verify(message, &signature)
    }
}

pub type PrivateKey = SigningKey;
pub type PublicKey = VerifyingKey;
pub type KeyPair = (SigningKey, VerifyingKey);

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

# 4. Fix crypto module re-exports
echo "Fixing crypto module re-exports..."
cat > src/crypto/mod.rs << 'EOF'
pub mod signatures;
pub mod utxo_set;
pub mod protocol;
pub mod sync;

// Re-export commonly used items from signatures
pub use signatures::{PrivateKey, PublicKey, KeyPair, SignatureData, sha512_hash, sha512_hash_string};
EOF

# 5. Fix config.rs network usage
echo "Fixing config.rs network usage..."
sed -i 's/match config.network {/match config.network {/g' src/config.rs
sed -i 's/config.config.network.default_rpc_port()/config.network.default_rpc_port()/g' src/config.rs

# 6. Fix error.rs imports
echo "Fixing error.rs imports..."
sed -i 's/crate::crypto::UTXOError/crate::database::utxo_set::UTXOError/g' src/error.rs
sed -i 's/crate::crypto::ProtocolError/crate::network::ProtocolError/g' src/error.rs

# 7. Fix UTXOError Display implementation
echo "Fixing UTXOError Display implementation..."
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

impl fmt::Display for UTXOError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UTXOError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            UTXOError::NotFound => write!(f, "UTXO not found"),
            UTXOError::AlreadySpent => write!(f, "UTXO already spent"),
            UTXOError::InvalidInput => write!(f, "Invalid input"),
        }
    }
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
        Ok(self.outputs.get(outpoint).map(|(output, height, is_cb, _spent)| {
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

    fn clear(&self) -> Result<(), UTXOError> {
        let mut storage = self.outputs.clone();
        storage.clear();
        Ok(())
    }
}
EOF

# 8. Fix reward amount types
echo "Fixing reward amount types..."
sed -i 's/pub amount: u64,/pub amount: f64,/g' src/blockchain/reward.rs
sed -i 's/100.0/100.0/g' src/blockchain/block.rs

# 9. Fix merkle tree implementation
echo "Fixing merkle tree implementation..."
cat > src/blockchain/merkle/mod.rs << 'EOF'
use sha2::{Sha512, Digest};

#[derive(Debug)]
pub struct MerkleTree {
    root: [u8; 64],
}

impl MerkleTree {
    pub fn new(hashes: &[[u8; 64]]) -> Result<Self, MerkleError> {
        if hashes.is_empty() {
            return Err(MerkleError::EmptyInput);
        }

        let root = Self::compute_root(hashes);
        Ok(MerkleTree { root })
    }

    pub fn compute_root(hashes: &[[u8; 64]]) -> [u8; 64] {
        let mut current_level = hashes.to_vec();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..current_level.len()).step_by(2) {
                if i + 1 < current_level.len() {
                    let mut hasher = Sha512::new();
                    hasher.update(&current_level[i]);
                    hasher.update(&current_level[i + 1]);
                    let hash = hasher.finalize();
                    let mut result = [0u8; 64];
                    result.copy_from_slice(&hash);
                    next_level.push(result);
                } else {
                    next_level.push(current_level[i]);
                }
            }

            current_level = next_level;
        }

        current_level[0]
    }

    pub fn root(&self) -> [u8; 64] {
        self.root
    }
}

#[derive(Debug)]
pub struct MerkleProof;

#[derive(Debug)]
pub enum MerkleError {
    EmptyInput,
    InvalidProof,
}
EOF

# 10. Fix network protocol merkle usage
echo "Fixing network protocol merkle usage..."
sed -i 's/let merkle_tree = MerkleTree::new(\&tx_hashes)/let merkle_tree = MerkleTree::new(\&tx_hashes)/g' src/network/protocol.rs
sed -i 's/\.map_err(|e| ProtocolError::SerializationError(e.to_string()))?;//g' src/network/protocol.rs

# 11. Fix serde_bytes usage for Vec<[u8; 64]>
echo "Fixing serde_bytes usage..."
sed -i 's/#\[serde(with = "serde_bytes")\]/\/\/ #[serde(with = "serde_bytes")]/g' src/network/protocol.rs

# 12. Remove unused imports
echo "Removing unused imports..."
sed -i '/use Instant;/d' src/network/sync.rs
sed -i '/sha512_hash_string,/d' src/network/protocol.rs
sed -i '/create_outpoint,/d' src/database/mod.rs

# 13. Fix the error conversion in error.rs
echo "Fixing error conversion..."
sed -i 's/error.to_string()/format!("{}", error)/g' src/error.rs

echo "All fixes applied! Please run 'cargo build' to check for remaining issues."