//! Database layer: thin wrapper around a pluggable UTXO storage.
//!
//! Notes:
//! - This module intentionally does **not** define a `Database` type anymore.
//!   Use `DatabaseManager` as the main abstraction.
//! - `DatabaseConfig` here is a lightweight config for the database module
//!   (distinct from `crate::config::DatabaseConfig`).
//! - UTXO storage is injected via `Box<dyn UTXOStorage + Send + Sync>` so you
//!   can use `MemoryUTXOStorage` (in-memory) or a persistent backend later.

pub mod utxo_set;

use serde::de::DeserializeOwned;
use sha2::{Digest, Sha512};
use std::path::PathBuf;

pub use utxo_set::{
    create_outpoint, hash_transaction, MemoryUTXOStorage, OutPoint, TxOutput, UTXOError,
    UTXORecord, UTXOSet, UTXOStats, UTXOStorage,
};

/// Simple config local to the database module.
///
/// This is **not** the same type as `crate::config::DatabaseConfig`.
/// Keep this minimal and specific to storage behavior needed here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseConfig {
    /// Where to keep on-disk data (if the storage backend uses disk).
    pub data_dir: PathBuf,
    /// Max cache size (in bytes) for the storage backend, if relevant.
    pub max_cache_size: usize,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data/chainstate"),
            max_cache_size: 512 * 1024 * 1024, // 512 MiB
        }
    }
}

/// High-level manager that owns a concrete UTXO storage.
///
/// You can pass `Box::new(MemoryUTXOStorage::new())` for in-memory usage, or
/// swap in a persistent backend in the future.
#[derive(Debug)]
pub struct DatabaseManager {
    storage: Box<dyn UTXOStorage + Send + Sync>,
    config: DatabaseConfig,
}

impl DatabaseManager {
    /// Construct a new database manager with the given storage implementation.
    pub fn new(storage: Box<dyn UTXOStorage + Send + Sync>, config: DatabaseConfig) -> Self {
        Self { storage, config }
    }

    /// Borrow the underlying storage as a trait object.
    pub fn storage(&self) -> &dyn UTXOStorage {
        &*self.storage
    }

    /// Mutably borrow the underlying storage as a trait object.
    pub fn storage_mut(&mut self) -> &mut dyn UTXOStorage {
        &mut *self.storage
    }

    /// Access the local database config.
    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// Convenience: clear the storage (useful for tests).
    pub fn clear(&mut self) -> Result<(), UTXOError> {
        self.storage.clear()
    }

    /// Deserialize with a checksum check wrapper, mapping errors into `UTXOError`.
    ///
    /// This function currently performs a plain bincode deserialize and wraps errors.
    /// If you later append a checksum to your serialized blobs, you can verify it here.
    pub fn deserialize_with_checksum<T: DeserializeOwned>(data: &[u8]) -> Result<T, UTXOError> {
        bincode::deserialize::<T>(data).map_err(|e| UTXOError::SerializationError(e.to_string()))
    }

    /// Serialize with a SHA-512 checksum appended (helper if you want checksummed blobs).
    /// Format: `<u32:len><bytes...><[u8;64]:sha512(bytes)>`
    pub fn serialize_with_checksum<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, UTXOError> {
        let bytes =
            bincode::serialize(value).map_err(|e| UTXOError::SerializationError(e.to_string()))?;
        let mut out = Vec::with_capacity(4 + bytes.len() + 64);
        let len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&bytes);
        let digest = Sha512::digest(&bytes);
        out.extend_from_slice(&digest);
        Ok(out)
    }

    /// Verify a checksummed blob written by `serialize_with_checksum` and return the raw bytes.
    pub fn verify_and_strip_checksum(data: &[u8]) -> Result<Vec<u8>, UTXOError> {
        if data.len() < 4 + 64 {
            return Err(UTXOError::SerializationError("blob too small".into()));
        }
        let mut len_le = [0u8; 4];
        len_le.copy_from_slice(&data[..4]);
        let len = u32::from_le_bytes(len_le) as usize;

        if data.len() != 4 + len + 64 {
            return Err(UTXOError::SerializationError("length mismatch".into()));
        }
        let payload = &data[4..4 + len];
        let checksum = &data[4 + len..];

        let digest = Sha512::digest(payload);
        if &digest[..] != checksum {
            return Err(UTXOError::SerializationError("checksum mismatch".into()));
        }
        Ok(payload.to_vec())
    }
}
