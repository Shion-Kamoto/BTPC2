//! UTXO storage abstraction + in-memory implementation.
//! Switched raw `[u8; 64]` fields to the `Hash` newtype for clean serde,
//! added `Default` where Clippy suggested, and small type aliases.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::fmt;

use crate::network::protocol::Hash;

/// A reference to a previous transaction output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct OutPoint {
    /// 64-byte SHA-512 tx hash (binary, not hex) — now uses `Hash` newtype.
    pub tx_hash: Hash,
    pub index: u32,
}

/// Canonical transaction output type used by the UTXO set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TxOutput {
    /// Value in base units (satoshis/credits).
    pub value: u64,
    /// Locking script (opaque to the UTXO set).
    pub script_pubkey: Vec<u8>,
}

/// An unspent output record exposed by `get_unspent_outputs`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UTXORecord {
    pub outpoint: OutPoint,
    pub output: TxOutput,
    pub block_height: u64,
    pub is_coinbase: bool,
}

/// Accumulated statistics over the UTXO set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UTXOStats {
    pub total_outputs: u64,
    pub total_value: u64,
    pub unspent_outputs: u64,
    pub unspent_value: u64,
}

/// Errors produced by UTXO operations.
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
            UTXOError::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
            UTXOError::NotFound => write!(f, "UTXO not found"),
            UTXOError::AlreadySpent => write!(f, "UTXO already spent"),
            UTXOError::InvalidInput => write!(f, "Invalid input"),
        }
    }
}

impl std::error::Error for UTXOError {}

/// Storage API for UTXO backends (in-memory, RocksDB, etc.).
pub trait UTXOStorage: fmt::Debug + Send + Sync {
    fn add_output(
        &mut self,
        outpoint: OutPoint,
        output: TxOutput,
        block_height: u64,
        is_coinbase: bool,
    ) -> Result<(), UTXOError>;

    /// Mark an output as spent. Implementations may simply remove it, or keep tombstones.
    fn spend_output(
        &mut self,
        outpoint: &OutPoint,
        _spending_tx_hash: Hash,
    ) -> Result<(), UTXOError>;

    fn get_output(&self, outpoint: &OutPoint) -> Result<Option<(TxOutput, u64, bool)>, UTXOError>;

    fn get_unspent_outputs(&self) -> Result<Vec<UTXORecord>, UTXOError>;

    fn get_stats(&self) -> Result<UTXOStats, UTXOError>;

    fn clear(&mut self) -> Result<(), UTXOError>;
}

/// Utility: SHA-512 hash of arbitrary bytes, returned as a 64-byte array.
pub fn hash_transaction(data: &[u8]) -> [u8; 64] {
    let mut hasher = Sha512::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let mut out = [0u8; 64];
    out.copy_from_slice(&digest);
    out
}

/// Utility constructor for OutPoint.
pub fn create_outpoint(tx_hash: Hash, index: u32) -> OutPoint {
    OutPoint { tx_hash, index }
}

/// Internal entry representation:
/// (TxOutput, block_height, is_coinbase)
pub type UTXOEntry = (TxOutput, u64, bool);

/// A simple in-memory UTXO storage, useful for tests and development.
#[derive(Debug, Default)]
pub struct MemoryUTXOStorage {
    pub outputs: HashMap<OutPoint, UTXOEntry>,
}

impl MemoryUTXOStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl UTXOStorage for MemoryUTXOStorage {
    fn add_output(
        &mut self,
        outpoint: OutPoint,
        output: TxOutput,
        block_height: u64,
        is_coinbase: bool,
    ) -> Result<(), UTXOError> {
        // Overwrites are not expected; guard to catch logic errors.
        if self.outputs.contains_key(&outpoint) {
            return Err(UTXOError::InvalidInput);
        }
        self.outputs
            .insert(outpoint, (output, block_height, is_coinbase));
        Ok(())
    }

    fn spend_output(
        &mut self,
        outpoint: &OutPoint,
        _spending_tx_hash: Hash,
    ) -> Result<(), UTXOError> {
        match self.outputs.remove(outpoint) {
            Some(_) => Ok(()),
            None => Err(UTXOError::NotFound),
        }
    }

    fn get_output(&self, outpoint: &OutPoint) -> Result<Option<(TxOutput, u64, bool)>, UTXOError> {
        Ok(self
            .outputs
            .get(outpoint)
            .map(|(o, h, c)| (o.clone(), *h, *c)))
    }

    fn get_unspent_outputs(&self) -> Result<Vec<UTXORecord>, UTXOError> {
        let mut v = Vec::with_capacity(self.outputs.len());
        for (op, (out, height, coinbase)) in &self.outputs {
            v.push(UTXORecord {
                outpoint: op.clone(),
                output: out.clone(),
                block_height: *height,
                is_coinbase: *coinbase,
            });
        }
        Ok(v)
    }

    fn get_stats(&self) -> Result<UTXOStats, UTXOError> {
        let mut stats = UTXOStats::default();
        stats.total_outputs = self.outputs.len() as u64;
        stats.unspent_outputs = stats.total_outputs;
        for (out, _, _) in self.outputs.values() {
            stats.total_value = stats.total_value.saturating_add(out.value);
            stats.unspent_value = stats.unspent_value.saturating_add(out.value);
        }
        Ok(stats)
    }

    fn clear(&mut self) -> Result<(), UTXOError> {
        self.outputs.clear();
        Ok(())
    }
}

/// A façade over a `UTXOStorage` backend.
#[derive(Debug)]
pub struct UTXOSet {
    storage: Box<dyn UTXOStorage + Send + Sync>,
}

impl UTXOSet {
    pub fn new(storage: Box<dyn UTXOStorage + Send + Sync>) -> Self {
        Self { storage }
    }

    pub fn add(
        &mut self,
        outpoint: OutPoint,
        output: TxOutput,
        block_height: u64,
        is_coinbase: bool,
    ) -> Result<(), UTXOError> {
        self.storage
            .add_output(outpoint, output, block_height, is_coinbase)
    }

    pub fn spend(&mut self, outpoint: &OutPoint, spending_tx_hash: Hash) -> Result<(), UTXOError> {
        self.storage.spend_output(outpoint, spending_tx_hash)
    }

    pub fn get(&self, outpoint: &OutPoint) -> Result<Option<(TxOutput, u64, bool)>, UTXOError> {
        self.storage.get_output(outpoint)
    }

    pub fn unspent(&self) -> Result<Vec<UTXORecord>, UTXOError> {
        self.storage.get_unspent_outputs()
    }

    pub fn stats(&self) -> Result<UTXOStats, UTXOError> {
        self.storage.get_stats()
    }

    pub fn clear(&mut self) -> Result<(), UTXOError> {
        self.storage.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_utxo_roundtrip() {
        let mut store = MemoryUTXOStorage::new();

        let tx_hash = Hash(hash_transaction(b"tx-1"));
        let op = create_outpoint(tx_hash, 0);
        let out = TxOutput {
            value: 42,
            script_pubkey: vec![0x51], // OP_TRUE for testing
        };

        store.add_output(op.clone(), out.clone(), 1, false).unwrap();

        let got = store.get_output(&op).unwrap().expect("present");
        assert_eq!(got.0.value, 42);
        assert_eq!(got.1, 1);
        assert!(!got.2);

        let stats = store.get_stats().unwrap();
        assert_eq!(stats.total_outputs, 1);
        assert_eq!(stats.unspent_outputs, 1);
        assert_eq!(stats.total_value, 42);
        assert_eq!(stats.unspent_value, 42);

        store
            .spend_output(&op, Hash(hash_transaction(b"spend")))
            .unwrap();
        assert!(store.get_output(&op).unwrap().is_none());
    }

    #[test]
    fn utxoset_facade() {
        let mut set = UTXOSet::new(Box::new(MemoryUTXOStorage::new()));
        let tx_hash = Hash(hash_transaction(b"tx-2"));
        let op = create_outpoint(tx_hash, 1);
        let out = TxOutput {
            value: 100,
            script_pubkey: vec![],
        };
        set.add(op.clone(), out.clone(), 10, true).unwrap();
        let rec = set.get(&op).unwrap().unwrap();
        assert_eq!(rec.0.value, 100);
        set.spend(&op, Hash(hash_transaction(b"spend-2"))).unwrap();
        assert!(set.get(&op).unwrap().is_none());
    }
}
