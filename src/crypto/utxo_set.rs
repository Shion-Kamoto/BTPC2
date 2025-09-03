// Minimal utxo_set for crypto module (re-exports from database)
#![allow(unused)]

pub use crate::database::utxo_set::{
    UTXOSet, UTXOStorage, UTXORecord, TxOutput, OutPoint,
    UTXOError, UTXOStats, hash_transaction, create_outpoint
};
