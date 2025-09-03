#!/bin/bash

# Focused fix script for the remaining issues

set -e

echo "Fixing remaining compilation issues..."

# 1. Fix missing semicolons in network/protocol.rs
echo "Fixing missing semicolons..."
sed -i '246s/$/;/' src/network/protocol.rs
sed -i '320s/$/;/' src/network/protocol.rs

# 2. Fix duplicate module declarations
echo "Fixing duplicate module declarations..."
sed -i '/pub mod block;/d' src/blockchain/mod.rs
sed -i '/pub mod reward;/d' src/blockchain/mod.rs
sed -i '/pub mod merkle;/d' src/blockchain/mod.rs
echo "pub mod block;" >> src/blockchain/mod.rs
echo "pub mod reward;" >> src/blockchain/mod.rs
echo "pub mod merkle;" >> src/blockchain/mod.rs

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

        // Convert Vec<u8> to fixed-size arrays
        let public_key_bytes: [u8; 32] = self.public_key.clone().try_into()
            .map_err(|_| SignatureError::new())?;
        let signature_bytes: [u8; 64] = self.signature.clone().try_into()
            .map_err(|_| SignatureError::new())?;

        let public_key = VerifyingKey::from_bytes(&public_key_bytes)?;
        let signature = Signature::from_bytes(&signature_bytes);

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

# 4. Fix config.rs network usage
echo "Fixing config.rs network usage..."
sed -i 's/match config.network {/match config.network {/g' src/config.rs

# 5. Fix UTXOStorage clear method signature
echo "Fixing UTXOStorage clear method..."
sed -i 's/fn clear(&self) -> Result<(), UTXOError>;/fn clear(&mut self) -> Result<(), UTXOError>;/g' src/database/utxo_set.rs

# 6. Fix MemoryUTXOStorage clear implementation
sed -i 's/fn clear(&self) -> Result<(), UTXOError> {/fn clear(&mut self) -> Result<(), UTXOError> {/g' src/database/utxo_set.rs
sed -i 's/let mut storage = self.outputs.clone();/ /g' src/database/utxo_set.rs
sed -i 's/storage.clear();/self.outputs.clear();/g' src/database/utxo_set.rs

# 7. Fix protocol.rs hash computation
echo "Fixing protocol.rs hash computation..."
sed -i '248s/Ok(hash_transaction(&serialized))/Ok(hash_transaction(&serialized?))/' src/network/protocol.rs

# 8. Fix protocol.rs verify_signature return type
sed -i '253,254s/self.signature.verify(&message)/self.signature.verify(&message).map(|_| true)/' src/network/protocol.rs

# 9. Fix protocol.rs merkle tree usage
sed -i '320s/let merkle_tree = MerkleTree::new(&tx_hashes)/let merkle_tree = MerkleTree::new(&tx_hashes.iter().map(|tx| {\n            let mut hash = [0u8; 64];\n            if tx.len() >= 64 {\n                hash.copy_from_slice(&tx[..64]);\n            }\n            hash\n        }).collect::<Vec<_>>())/' src/network/protocol.rs
sed -i '324s/merkle_tree.root()/merkle_tree?.root()/' src/network/protocol.rs

# 10. Add serde_bytes attributes for [u8; 64] arrays
echo "Adding serde_bytes attributes..."
# MessageHeader
sed -i '118s/pub checksum: \[u8; 64\],/    #[serde(with = "serde_bytes")]\n    pub checksum: [u8; 64],/g' src/network/protocol.rs

# InventoryVector
sed -i '211s/pub hash: \[u8; 64\],/    #[serde(with = "serde_bytes")]\n    pub hash: [u8; 64],/g' src/network/protocol.rs

# BlockHeader
sed -i '286s/pub prev_block_hash: \[u8; 64\],/    #[serde(with = "serde_bytes")]\n    pub prev_block_hash: [u8; 64],/g' src/network/protocol.rs
sed -i '288s/pub merkle_root: \[u8; 64\],/    #[serde(with = "serde_bytes")]\n    pub merkle_root: [u8; 64],/g' src/network/protocol.rs

# GetBlocksMessage
sed -i '335s/pub block_locator_hashes: Vec<\[u8; 64\]>,/    #[serde(with = "serde_bytes")]\n    pub block_locator_hashes: Vec<[u8; 64]>,/g' src/network/protocol.rs
sed -i '337s/pub hash_stop: \[u8; 64\],/    #[serde(with = "serde_bytes")]\n    pub hash_stop: [u8; 64],/g' src/network/protocol.rs

# 11. Fix unused variable warnings
echo "Fixing unused variable warnings..."
sed -i '178s/height, is_coinbase, spent/_height, _is_coinbase, spent/g' src/database/utxo_set.rs

# 12. Remove unused imports
echo "Removing unused imports..."
sed -i '/use Instant;/d' src/network/sync.rs
sed -i '/sha512_hash_string,/d' src/network/protocol.rs
sed -i '/create_outpoint,/d' src/database/mod.rs
sed -i '/use Signer;/d' src/crypto/signatures.rs

# 13. Fix the error conversion in error.rs
echo "Fixing error conversion..."
sed -i 's/error.to_string()/format!("{}", error)/g' src/error.rs

echo "All fixes applied! Please run 'cargo build' to check for remaining issues."