#!/bin/bash

# Fix the corrupted code from previous sed commands

set -e

echo "Fixing corrupted code..."

# 1. Fix protocol.rs line 249
echo "Fixing protocol.rs line 249..."
sed -i '249s/Ok(hash_transaction(Ok(hash_transaction(&serialized))serialized?))/Ok(hash_transaction(\&serialized?))/' src/network/protocol.rs

# 2. Fix protocol.rs line 254
echo "Fixing protocol.rs line 254..."
sed -i '254s/self.signature.verify(self.signature.verify(&message)message).map(|_| true)/self.signature.verify(\&message).map(|_| true)/' src/network/protocol.rs

# 3. Fix protocol.rs lines 321-324 (merkle tree code)
echo "Fixing protocol.rs merkle tree code..."
sed -i '321,324d' src/network/protocol.rs
sed -i '321a\        let tx_hashes: Vec<[u8; 64]> = tx_hashes.iter().map(|tx| {\n            let mut hash = [0u8; 64];\n            if tx.len() >= 64 {\n                hash.copy_from_slice(&tx[..64]);\n            }\n            hash\n        }).collect();\n        let merkle_tree = MerkleTree::new(&tx_hashes);' src/network/protocol.rs

# 4. Fix utxo_set.rs clear method corruption
echo "Fixing utxo_set.rs clear method..."
# First, check the current state and fix the corrupted lines
sed -i '99s/fn clear(fn clear(&self) -> Result<(), UTXOError> {mut self) -> Result<(), UTXOError> {/fn clear(&mut self) -> Result<(), UTXOError> {/' src/database/utxo_set.rs
sed -i '189s/fn clear(fn clear(&self) -> Result<(), UTXOError> {mut self) -> Result<(), UTXOError> {/fn clear(&mut self) -> Result<(), UTXOError> {/' src/database/utxo_set.rs

# 5. Check if the clear method is still corrupted and fix it properly
if grep -q "fn clear(fn clear" src/database/utxo_set.rs; then
    echo "Fixing deeply corrupted clear methods..."
    # Backup the file
    cp src/database/utxo_set.rs src/database/utxo_set.rs.backup

    # Recreate the file with proper clear methods
    head -n 98 src/database/utxo_set.rs.backup > src/database/utxo_set.rs
    echo "    fn clear(&mut self) -> Result<(), UTXOError>;" >> src/database/utxo_set.rs
    sed -n '100,188p' src/database/utxo_set.rs.backup >> src/database/utxo_set.rs
    echo "    fn clear(&mut self) -> Result<(), UTXOError> {" >> src/database/utxo_set.rs
    echo "        self.outputs.clear();" >> src/database/utxo_set.rs
    echo "        Ok(())" >> src/database/utxo_set.rs
    echo "    }" >> src/database/utxo_set.rs
    sed -n '190,$p' src/database/utxo_set.rs.backup >> src/database/utxo_set.rs
fi

echo "Corruption fixes applied! Please run 'cargo build' to check for remaining issues."