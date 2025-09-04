//! A standalone BTPC wallet using the Dilithium5 signature scheme.
//!
//! This program demonstrates how to generate a Dilithium5 keypair, derive a
//! wallet address from the public key using SHA‑512 and hex encoding, and
//! persist the key material to disk.  The wallet file stores the public key,
//! secret key, derived address and an on‑chain balance (initially zero).
//!
//! Usage examples:
//!
//! ```bash
//! # Generate a new wallet and save it to wallet.json
//! cargo run --bin btpc_wallet_dilithium -- generate --file wallet.json
//!
//! # Show the address contained in an existing wallet file
//! cargo run --bin btpc_wallet_dilithium -- address --file wallet.json
//!
//! # Display the current balance recorded in the wallet file
//! cargo run --bin btpc_wallet_dilithium -- balance --file wallet.json
//! ```
//!
//! Note: this example depends on the `pqcrypto` crate for Dilithium5 key
//! generation.  To compile successfully you must add the following to your
//! `Cargo.toml` dependencies:
//!
//! ```toml
//! pqcrypto = "0.7"
//! serde_json = "1.0"
//! clap = { version = "4.0", features = ["derive"] }
//! ```
//!
//! The BTPC repository currently includes SPHINCS+ support only.  By adding
//! `pqcrypto` as shown above you gain access to the Dilithium5 module used
//! here.  See the pqcrypto crate documentation for more details.

use clap::{Parser, Subcommand};
use pqcrypto::sign::dilithium5;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::fs;
use std::io::{self, Write};

/// Convert a byte slice into a lowercase hex string using SHA‑512.
fn derive_address(pub_key: &[u8]) -> String {
    let hash = Sha512::digest(pub_key);
    hex::encode(hash)
}

/// Structure of a wallet file on disk.  The secret key is stored as a
/// byte vector; in a production wallet you should encrypt this field before
/// writing to disk.
#[derive(Debug, Serialize, Deserialize)]
struct WalletFile {
    pub public_key: Vec<u8>,
    pub secret_key: Vec<u8>,
    pub address: String,
    pub balance: u64,
}

/// Top‑level CLI definition using `clap`.  Subcommands operate on a
/// wallet file.
#[derive(Parser)]
#[command(name = "btpc-wallet-dilithium", version, about = "BTPC Dilithium5 wallet CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Subcommands supported by the wallet CLI.
#[derive(Subcommand)]
enum Commands {
    /// Generate a new wallet and persist it to the given JSON file.  If the
    /// file already exists it will be overwritten.  The command prints the
    /// derived address to STDOUT.
    Generate {
        /// Path to the wallet JSON file to create.
        #[arg(short, long)]
        file: String,
    },
    /// Display the address stored in the given wallet file.
    Address {
        /// Path to the wallet JSON file to read.
        #[arg(short, long)]
        file: String,
    },
    /// Show the current balance recorded in the wallet file (base units).
    Balance {
        /// Path to the wallet JSON file to read.
        #[arg(short, long)]
        file: String,
    },
    /// Sign an arbitrary message with the wallet's secret key.  The
    /// signature is printed as a hex string.  Note: BTPC transactions
    /// require structured data; this is merely a demonstration.
    Sign {
        /// Path to the wallet JSON file to read.
        #[arg(short, long)]
        file: String,
        /// Message to sign.
        #[arg(short, long)]
        message: String,
    },
}

fn main() {
    // Parse command line arguments
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate { file } => {
            // Generate a new Dilithium5 keypair
            let (public_key, secret_key) = dilithium5::keypair();
            let address = derive_address(public_key.as_bytes());

            let wallet = WalletFile {
                public_key: public_key.as_bytes().to_vec(),
                secret_key: secret_key.as_bytes().to_vec(),
                address: address.clone(),
                balance: 0,
            };

            // Serialize wallet to JSON and write to disk
            let json = serde_json::to_string_pretty(&wallet)
                .expect("Failed to serialize wallet");
            fs::write(&file, json).expect("Failed to write wallet file");
            println!("New wallet generated and saved to {}", file);
            println!("Address: {}", address);
            println!("WARNING: The secret key stored in this file is unencrypted.\n  Encrypt or protect the file in a real application.");
        }
        Commands::Address { file } => {
            let wallet = read_wallet(&file).expect("Failed to read wallet file");
            println!("Address: {}", wallet.address);
        }
        Commands::Balance { file } => {
            let wallet = read_wallet(&file).expect("Failed to read wallet file");
            println!("Balance: {} base units ({:.8} BTP)", wallet.balance, wallet.balance as f64 / 100_000_000f64);
        }
        Commands::Sign { file, message } => {
            let wallet = read_wallet(&file).expect("Failed to read wallet file");
            let secret_key = dilithium5::SecretKey::from_bytes(&wallet.secret_key)
                .expect("Invalid secret key bytes");
            let signature = dilithium5::sign(message.as_bytes(), &secret_key);
            println!("Signature (hex): {}", hex::encode(signature.as_bytes()));
        }
    }
}

/// Read a wallet file from disk.  Returns None if the file cannot be read
/// or parsed.
fn read_wallet(file: &str) -> Option<WalletFile> {
    let json = fs::read_to_string(file).ok()?;
    serde_json::from_str(&json).ok()
}