#!/bin/bash

# Fix mismatched delimiter in protocol.rs

set -e

echo "Fixing mismatched delimiter in protocol.rs..."

# 1. First, let's check the current state around line 313-331
echo "Current state around lines 313-331:"
sed -n '310,335p' src/network/protocol.rs

# 2. Fix the mismatched delimiter by reconstructing the calculate_merkle_root method
echo "Reconstructing calculate_merkle_root method..."
sed -i '313,331d' src/network/protocol.rs

# Add the corrected method
sed -i '313a\
    pub fn calculate_merkle_root(&mut self) -> Result<[u8; 64], ProtocolError> {\
        let tx_hashes: Vec<[u8; 64]> = self.transactions.iter()\
            .map(|tx| {\
                let mut hash = [0u8; 64];\
                let tx_data = bincode::serialize(tx)\
                    .map_err(|e| ProtocolError::SerializationError(e.to_string()))?;\
                if tx_data.len() >= 64 {\
                    hash.copy_from_slice(&tx_data[..64]);\
                }\
                Ok(hash)\
            })\
            .collect::<Result<Vec<_>, ProtocolError>>()?;\
\
        let merkle_tree = MerkleTree::new(&tx_hashes)\
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))?;\
        Ok(merkle_tree.root())\
    }' src/network/protocol.rs

echo "Delimiter fix applied! Please run 'cargo build' to check for remaining issues."