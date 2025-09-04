//! Demonstration of mining blocks and crediting a quantum‑resistant
//! wallet with the block rewards.  This script ties together the
//! consensus proof‑of‑work (SHA‑512), the linear reward schedule and
//! a simple wallet that holds a Dilithium5 keypair and tracks its
//! balance.  Each mined block contains a single coinbase transaction
//! paying the block reward to the wallet’s address.  After mining a
//! block, the wallet balance is updated accordingly.
//!
//! To run this example:
//!
//! ```bash
//! cargo run --bin mine_send_wallet
//! ```
//!
//! Ensure that your `Cargo.toml` exposes the consensus module (see
//! `mine_chain_consensus.rs`) and that the `pqcrypto` crate is
//! available for Dilithium5 key generation.

use btpc_quantum_resistant_chain::blockchain::merkle::MerkleTree;
use btpc_quantum_resistant_chain::blockchain::reward::calculate_block_reward;
use btpc_quantum_resistant_chain::consensus::{DifficultyManager, DifficultyParams};
// Use ed25519 for key generation in the wallet.  Ed25519 is not quantum‑
// resistant, but this avoids the need for external PQCrypto dependencies.
// To use a quantum‑resistant scheme, replace this with a suitable
// implementation and add the corresponding crate to Cargo.toml.
// For demonstration we derive a wallet address by hashing the current timestamp.
// We do not generate a cryptographic keypair here to avoid RNG version conflicts.
use sha2::{Digest, Sha512};
use hex;
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use std::time::Duration;

/// Convert a byte slice into a lowercase hex string using SHA‑512.
fn sha512_hex(data: &[u8]) -> String {
    let hash = Sha512::digest(data);
    hex::encode(hash)
}

/// A simple wallet that holds an Ed25519 keypair, derives an
/// address from the public key and tracks a balance in base units.
struct Wallet {
    pub address: String,
    pub balance: u64,
}

impl Wallet {
    /// Create a new wallet by generating an Ed25519 keypair and
    /// deriving the address via SHA‑512 of the public key.
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        // Derive the address by hashing the timestamp.  In a real wallet,
        // use a proper keypair and address scheme.
        let address = sha512_hex(&now.to_be_bytes());
        Wallet {
            address,
            balance: 0,
        }
    }

    /// Credit the wallet with the given amount (in base units).  In
    /// BTPC, 1 BTP equals 100 000 000 base units.
    pub fn credit(&mut self, amount: u64) {
        self.balance = self
            .balance
            .checked_add(amount)
            .expect("Balance overflow");
    }

    /// Return the balance in BTP as a floating‑point string for display.
    pub fn balance_btp(&self) -> String {
        let btp = self.balance as f64 / 100_000_000f64;
        format!("{:.8}", btp)
    }
}

/// A minimal block header used for PoW mining.  This header omits
/// transaction count and other fields for brevity.  The `bits` field
/// stores the difficulty in Bitcoin’s compact format (nBits).
struct BlockHeader {
    version: u32,
    prev_hash: [u8; 64],
    merkle_root: [u8; 64],
    timestamp: u64,
    bits: u32,
    nonce: u64,
}

impl BlockHeader {
    /// Serialize the header into a big‑endian byte vector.  The order
    /// of fields is deterministic: version, prev_hash, merkle_root,
    /// timestamp, bits, nonce.
    fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(4 + 64 + 64 + 8 + 4 + 8);
        v.extend(&self.version.to_be_bytes());
        v.extend(&self.prev_hash);
        v.extend(&self.merkle_root);
        v.extend(&self.timestamp.to_be_bytes());
        v.extend(&self.bits.to_be_bytes());
        v.extend(&self.nonce.to_be_bytes());
        v
    }
}

/// Compute double SHA‑512 of the given data.
fn double_sha512(data: &[u8]) -> [u8; 64] {
    let first = Sha512::digest(data);
    let second = Sha512::digest(&first);
    let mut out = [0u8; 64];
    out.copy_from_slice(&second);
    out
}

/// Convert a difficulty (u64) into its compact representation (nBits).
/// This is the same helper used in `mine_chain_consensus.rs`.
fn difficulty_to_compact(difficulty: u64) -> u32 {
    if difficulty == 0 {
        return 0;
    }
    let leading = difficulty.leading_zeros();
    let bits = 64 - leading;
    let mut size = ((bits + 7) / 8) as u8;
    let mut compact: u32 = if size <= 3 {
        (difficulty << (8 * (3 - size))) as u32
    } else {
        (difficulty >> (8 * (size - 3))) as u32
    };
    if compact & 0x0080_0000 != 0 {
        compact >>= 8;
        size += 1;
    }
    compact | ((size as u32) << 24)
}

fn main() {
    // Number of additional blocks to mine after genesis.
    const NUM_BLOCKS: u64 = 20;
    // Create the wallet.  In a real application you would load an
    // existing wallet from disk or allow the user to input a seed.
    let mut wallet = Wallet::new();
    println!("Generated wallet address: {}", wallet.address);
    println!();

    // Difficulty manager: start with a difficulty of 1 for easy mining.
    let diff_params = DifficultyParams::default();
    let diff_mgr = DifficultyManager::new(diff_params, 1);

    // Keep track of the previous block hash and a global nonce.
    let mut prev_hash: [u8; 64] = [0u8; 64];
    let mut next_nonce: u64 = 0;

    // Mine the genesis block first.  The coinbase transaction
    // includes a custom message; we also credit the reward to the
    // wallet for demonstration purposes.
    {
        let message = "BTPC Genesis: Reward to quantum wallet";
        let tx_hash = double_sha512(message.as_bytes());
        let leaves: [[u8; 64]; 1] = [tx_hash];
        let merkle_root = MerkleTree::new(&leaves)
            .expect("merkle tree")
            .root();
        let difficulty = diff_mgr.get_difficulty();
        let target32 = DifficultyManager::difficulty_to_target(difficulty);
        // To make mining feasible in this demo, replicate the last 8 bytes of the
        // 256‑bit target across all 64 bytes.  This yields a very large target
        // (effectively 0xFFFF_FFFF_FFFF_FFFF repeated), so the first nonce will
        // always satisfy the difficulty check.
        let last8 = &target32[24..32];
        let mut target64 = [0u8; 64];
        for i in 0..8 {
            target64[i * 8..(i + 1) * 8].copy_from_slice(last8);
        }
        let bits = difficulty_to_compact(difficulty);
        let version: u32 = 1;
        let timestamp: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let mut nonce = next_nonce;
        loop {
            let header = BlockHeader {
                version,
                prev_hash,
                merkle_root,
                timestamp,
                bits,
                nonce,
            };
            let hash = double_sha512(&header.to_bytes());
            if hash <= target64 {
                prev_hash = hash;
                // Use 0.0 for the genesis height when calling calculate_block_reward
                wallet.credit(calculate_block_reward(0.0));
                println!("Genesis block mined!");
                println!("Nonce: {}", nonce);
                println!("Hash: {}", hex::encode(hash));
                println!("Difficulty: {}", difficulty);
                println!("Bits (nBits): 0x{:08x}", bits);
                println!("Merkle root: {}", hex::encode(merkle_root));
                println!("wallet address!: {}", wallet.address);
                println!("Wallet balance: {} BTP", wallet.balance_btp());
                println!("timestamp!: {}", timestamp);
                println!();
                next_nonce = nonce.checked_add(1).expect("nonce overflow");
                break;
            }
            nonce = nonce.checked_add(1).expect("nonce overflow");
        }
    }

    // Mine subsequent blocks and credit the wallet with each block’s reward
    // and a fixed transaction fee.  In this demonstration we assign a
    // constant `TX_FEE` per block to simulate fee collection.  A real
    // implementation would sum (inputs − outputs) across transactions.
    const TX_FEE: u64 = 1_0; // 0.00001 BTP per block in base units
    for height in 1..=NUM_BLOCKS {
        // Base reward using linear decay schedule
        let reward = calculate_block_reward(height as f64);
        // Total reward credited to miner includes transaction fees
        let total_reward = reward + TX_FEE;
        // Build a coinbase payload string.  The transaction fee is not part
        // of the coinbase payload, but the wallet address and height are.
        let coinbase_data = format!("coinbase:{}:{}:{}", height, total_reward, wallet.address);
        let tx_hash = double_sha512(coinbase_data.as_bytes());
        let leaves: [[u8; 64]; 1] = [tx_hash];
        let merkle_root = MerkleTree::new(&leaves)
            .expect("merkle tree")
            .root();
        let difficulty = diff_mgr.get_difficulty();
        let target32 = DifficultyManager::difficulty_to_target(difficulty);
        // Replicate the last 8 bytes of the 256-bit target to build a 64-byte
        // target for SHA-512 proof-of-work.  This makes mining trivial in
        // this demonstration.  For realistic mining, derive a proper 512-bit
        // target from the difficulty.
        let last8 = &target32[24..32];
        let mut target64 = [0u8; 64];
        for i in 0..8 {
            target64[i * 8..(i + 1) * 8].copy_from_slice(last8);
        }
        let bits = difficulty_to_compact(difficulty);
        let version: u32 = 1;
        let timestamp: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let mut nonce = next_nonce;
        loop {
            let header = BlockHeader {
                version,
                prev_hash,
                merkle_root,
                timestamp,
                bits,
                nonce,
            };
            let hash = double_sha512(&header.to_bytes());
            if hash <= target64 {
                prev_hash = hash;
                // Credit block reward and fees to the wallet
                wallet.credit(total_reward);
                println!("Mined block {}!", height);
                println!("Nonce: {}", nonce);
                println!("Hash: {}", hex::encode(hash));
                println!("Difficulty: {}", difficulty);
                println!("Bits (nBits): 0x{:08x}", bits);
                println!("Merkle root: {}", hex::encode(merkle_root));
                println!("Block reward: {:.8} BTP ({} base units)", reward as f64 / 100_000_000f64, reward);
                println!("Transaction fees: {:.8} BTP ({} base units)", TX_FEE as f64 / 100_000_000f64, TX_FEE);
                println!("Total credited: {:.8} BTP ({} base units)", total_reward as f64 / 100_000_000f64, total_reward);
                println!("wallet address!: {}", wallet.address);
                println!("Wallet balance: {} BTP", wallet.balance_btp());
                println!("timestamp!: {}", timestamp);
                println!();
                next_nonce = nonce.checked_add(1).expect("nonce overflow");
                // Sleep to simulate passage of time between blocks.  Adjust
                // or remove as desired.  Replace 5 with 600 for a 10 minute delay.
                thread::sleep(Duration::from_secs(60));
                break;
            }
            nonce = nonce.checked_add(1).expect("nonce overflow");
        }
    }
}