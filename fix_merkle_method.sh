#!/bin/bash

# Fix the corrupted calculate_merkle_root method in protocol.rs

set -e

echo "Fixing corrupted calculate_merkle_root method..."

# 1. Find and remove the corrupted lines
echo "Removing corrupted lines..."
sed -i '/let mut root = \[0u8; 64\];/d' src/network/protocol.rs
sed -i '/root.copy_from_slice(merkle_tree.root());/d' src/network/protocol.rs
sed -i '/self.header.merkle_root = root;/d' src/network/protocol.rs
sed -i '/Ok(root)/d' src/network/protocol.rs

# 2. Fix the calculate_merkle_root method
echo "Fixing calculate_merkle_root method..."
# Find the line number where the method starts
start_line=$(grep -n "pub fn calculate_merkle_root" src/network/protocol.rs | cut -d: -f1)
if [ -n "$start_line" ]; then
    # Find the line where the method should end (look for the next closing brace at the same indentation)
    end_line=$(sed -n "${start_line},\$p" src/network/protocol.rs | grep -n -m 1 "^    }" | cut -d: -f1)
    end_line=$((start_line + end_line - 1))

    if [ -n "$end_line" ]; then
        # Replace the method with a correct implementation
        sed -i "${start_line},${end_line}d" src/network/protocol.rs
        sed -i "${start_line}a\
    pub fn calculate_merkle_root(&mut self) -> Result<[u8; 64], ProtocolError> {\n\
        let tx_hashes: Vec<[u8; 64]> = self.transactions.iter()\n\
            .map(|tx| {\n\
                tx.hash()\n\
                    .map_err(|e| ProtocolError::SerializationError(e.to_string()))\n\
            })\n\
            .collect::<Result<Vec<_>, ProtocolError>>()?;\n\
\n\
        let merkle_tree = MerkleTree::new(&tx_hashes)\n\
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))?;\n\
        \n\
        let root = merkle_tree.root();\n\
        self.header.merkle_root = root;\n\
        Ok(root)\n\
    }" src/network/protocol.rs
    fi
fi

echo "calculate_merkle_root method fixed!"