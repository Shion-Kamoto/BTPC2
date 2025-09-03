#!/bin/bash

# BTPC Quantum-Resistant Blockchain Development Starter Script
# ------------------------------------------------------------
# This script sets up the development environment for the quantum-resistant blockchain
# with SHA512 algorithm and linear decay rewards

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if running in PyCharm terminal
if [[ -z "${PYCHARM_HOSTED}" ]]; then
    echo -e "${YELLOW}Warning: Not running in PyCharm terminal. Some features may not work properly.${NC}"
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Function to check command success
check_success() {
    if [ $? -ne 0 ]; then
        echo -e "${RED}Error: $1 failed${NC}"
        exit 1
    fi
}

# Function to print section header
print_section() {
    echo -e "${BLUE}=== $1 ===${NC}"
}

# 1. Install Rust if not present
print_section "Installing Rust"
if ! command -v rustup &> /dev/null; then
    echo -e "${GREEN}Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    check_success "Rust installation"
    source "$HOME"/.cargo/env
else
    echo -e "${GREEN}Rust is already installed${NC}"
fi

# 2. Update Rust
echo -e "${GREEN}Updating Rust toolchain...${NC}"
rustup update
check_success "Rust update"

# 3. Install required components
echo -e "${GREEN}Installing Rust components...${NC}"
rustup component add rustfmt clippy
rustup target add x86_64-unknown-linux-gnu
check_success "Rust component installation"

# 4. Install system dependencies (Ubuntu/Debian)
print_section "Installing System Dependencies"
echo -e "${GREEN}Installing system dependencies...${NC}"
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    cmake \
    libclang-dev \
    libssl-dev \
    pkg-config \
    python3-pip \
    git \
    protobuf-compiler \
    libsqlite3-dev \
    libzmq3-dev
check_success "System dependencies installation"

# 5. Create Python virtual environment
print_section "Setting up Python Environment"
echo -e "${GREEN}Setting up Python virtual environment...${NC}"
python3 -m venv .venv
source .venv/bin/activate
pip install --upgrade pip
pip install \
    pre-commit \
    black \
    mypy \
    pylint \
    pytest \
    toml \
    requests \
    python-dotenv
check_success "Python environment setup"

# 6. Configure pre-commit hooks
print_section "Setting up Pre-commit Hooks"
echo -e "${GREEN}Setting up pre-commit hooks...${NC}"
cat > .pre-commit-config.yaml <<EOL
repos:
- repo: https://github.com/pre-commit/pre-commit-hooks
  rev: v4.4.0
  hooks:
    - id: trailing-whitespace
    - id: end-of-file-fixer
    - id: check-yaml
    - id: check-added-large-files
    - id: check-toml
- repo: https://github.com/rust-lang/rustfmt
  rev: stable
  hooks:
    - id: rustfmt
- repo: https://github.com/doublify/pre-commit-rust
  rev: master
  hooks:
    - id: clippy
      args: [--all-features, --all-targets, -- -D warnings]
    - id: cargo-check
    - id: cargo-test
EOL

pre-commit install
check_success "Pre-commit setup"

# 7. Create project structure
print_section "Creating Project Structure"
echo -e "${GREEN}Creating project directories...${NC}"
mkdir -p src/blockchain src/network src/crypto src/database src/consensus src/config src/models src/error
mkdir -p config tests scripts docs .idea/runConfigurations

# Create basic Rust module files
touch src/blockchain/mod.rs src/blockchain/chain.rs src/blockchain/block.rs src/blockchain/reward.rs
touch src/network/mod.rs src/network/p2p.rs src/network/sync.rs src/network/protocol.rs
touch src/crypto/mod.rs src/crypto/sha512.rs src/crypto/signatures.rs src/crypto/merkle.rs
touch src/database/mod.rs src/database/rocksdb.rs src/database/utxo_set.rs
touch src/consensus/mod.rs src/consensus/pow.rs src/consensus/difficulty.rs
touch src/config/mod.rs src/models/mod.rs src/error/mod.rs
touch src/main.rs src/lib.rs

# 8. Create PyCharm run configurations
print_section "Creating PyCharm Run Configurations"
echo -e "${GREEN}Creating PyCharm run configurations...${NC}"

# Mainnet node configuration
cat > .idea/runConfigurations/Mainnet_Node.xml <<EOL
<component name="ProjectRunConfigurationManager">
  <configuration default="false" name="Mainnet Node" type="CargoCommandRunConfiguration" factoryName="Cargo Command">
    <option name="command" value="run --release" />
    <option name="workingDirectory" value="file://\$PROJECT_DIR\$" />
    <option name="emulateTerminal" value="true" />
    <option name="channel" value="DEFAULT" />
    <option name="requiredFeatures" value="true" />
    <option name="allFeatures" value="false" />
    <option name="envs">
      <map>
        <entry key="RUST_LOG" value="info,btpc=debug" />
        <entry key="RUST_BACKTRACE" value="1" />
      </map>
    </option>
    <option name="additionalArguments" value="-- --config config/mainnet.toml" />
    <method v="2">
      <option name="CARGO.BUILD_TASK_PROVIDER" enabled="true" />
    </method>
  </configuration>
</component>
EOL

# Testnet node configuration
cat > .idea/runConfigurations/Testnet_Node.xml <<EOL
<component name="ProjectRunConfigurationManager">
  <configuration default="false" name="Testnet Node" type="CargoCommandRunConfiguration" factoryName="Cargo Command">
    <option name="command" value="run --release" />
    <option name="workingDirectory" value="file://\$PROJECT_DIR\$" />
    <option name="emulateTerminal" value="true" />
    <option name="channel" value="DEFAULT" />
    <option name="requiredFeatures" value="true" />
    <option name="allFeatures" value="false" />
    <option name="envs">
      <map>
        <entry key="RUST_LOG" value="debug,btpc=trace" />
        <entry key="RUST_BACKTRACE" value="full" />
      </map>
    </option>
    <option name="additionalArguments" value="-- --config config/testnet.toml" />
    <method v="2">
      <option name="CARGO.BUILD_TASK_PROVIDER" enabled="true" />
    </method>
  </configuration>
</component>
EOL

# Regtest miner configuration
cat > .idea/runConfigurations/Regtest_Miner.xml <<EOL
<component name="ProjectRunConfigurationManager">
  <configuration default="false" name="Regtest Miner" type="CargoCommandRunConfiguration" factoryName="Cargo Command">
    <option name="command" value="run --release" />
    <option name="workingDirectory" value="file://\$PROJECT_DIR\$" />
    <option name="emulateTerminal" value="true" />
    <option name="channel" value="DEFAULT" />
    <option name="requiredFeatures" value="true" />
    <option name="allFeatures" value="false" />
    <option name="envs">
      <map>
        <entry key="RUST_LOG" value="trace" />
        <entry key="RUST_BACKTRACE" value="full" />
      </map>
    </option>
    <option name="additionalArguments" value="-- --config config/regtest.toml --mine" />
    <method v="2">
      <option name="CARGO.BUILD_TASK_PROVIDER" enabled="true" />
    </method>
  </configuration>
</component>
EOL

# 9. Create configuration files
print_section "Creating Configuration Files"
echo -e "${GREEN}Creating network configuration files...${NC}"

# Create the config files we discussed earlier
cat > config/mainnet.toml <<'EOL'
[network]
bootnodes = [
    "/ip4/18.144.1.23/tcp/8333/p2p/12D3KooWMainNode1",
    "/ip4/52.34.156.78/tcp/8333/p2p/12D3KooWMainNode2",
    "/ip4/138.197.212.45/tcp/8333/p2p/12D3KooWMainNode3"
]
listen_addresses = [
    "/ip4/0.0.0.0/tcp/8333",
    "/ip4/0.0.0.0/tcp/8334/ws"
]
max_peers = 125

[consensus]
target_block_time = 600
difficulty_adjustment_blocks = 2016
initial_difficulty = "00000000ffffffffffffffffffffffffffffffffffffffffffffffffffffffff"

[rewards]
initial_reward = 32375000000
tail_reward = 50000000
decay_period_years = 24
EOL

cat > config/testnet.toml <<'EOL'
[network]
bootnodes = [
    "/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWTestNode1",
    "/ip4/127.0.0.1/tcp/30334/p2p/12D3KooWTestNode2"
]
listen_addresses = [
    "/ip4/0.0.0.0/tcp/30333",
    "/ip4/0.0.0.0/tcp/30334/ws"
]
max_peers = 50

[consensus]
target_block_time = 300
difficulty_adjustment_blocks = 2016
initial_difficulty = "0000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"

[rewards]
initial_reward = 32375000000
tail_reward = 50000000
decay_period_years = 24
EOL

cat > config/regtest.toml <<'EOL'
[network]
bootnodes = []
listen_addresses = [
    "/ip4/0.0.0.0/tcp/18444",
    "/ip4/0.0.0.0/tcp/18445/ws"
]
max_peers = 10

[consensus]
target_block_time = 60
difficulty_adjustment_blocks = 144
initial_difficulty = "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"

[rewards]
initial_reward = 100000000000
tail_reward = 500000000
decay_period_years = 100
EOL

# 10. Create Cargo.toml with correct dependencies
print_section "Creating Cargo.toml"
echo -e "${GREEN}Creating Cargo.toml with quantum-resistant dependencies...${NC}"

cat > Cargo.toml <<'EOL'
[package]
name = "btpc-quantum-resistant-chain"
version = "0.1.0"
edition = "2021"
description = "Quantum-resistant blockchain with SHA512 and linear decay rewards"
license = "MIT OR Apache-2.0"
authors = ["BTPC Developer <developer@btpc.org>"]
repository = "https://github.com/btpc/quantum-resistant-chain"

[dependencies]
rocksdb = { version = "0.21", features = ["multi-threaded-cf"] }
tokio = { version = "1.0", features = ["full", "rt-multi-thread", "macros"] }
sha2 = "0.10"
blake3 = "1.0"
pqcrypto = { version = "0.16", features = ["dilithium5"] }
rayon = "1.8"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
thiserror = "1.0"
log = "0.4"
env_logger = "0.10"
hex = "0.4"
num-bigint = "0.4"
num-traits = "0.2"
chrono = { version = "0.4", features = ["serde"] }
config = "0.13"
clap = { version = "4.0", features = ["derive"] }
lazy_static = "1.4"
arc-swap = "1.6"
dashmap = "5.0"
futures = "0.3"
async-trait = "0.1"
tracing = "0.1"
tracing-subscriber = "0.3"

# Libp2p with correct features
libp2p = { version = "0.52", features = [
    "tcp",
    "websocket",
    "dns",
    "kad",
    "gossipsub",
    "identify",
    "ping",
    "noise",
    "yamux",
    "tls"
] }

[dev-dependencies]
tempfile = "3.0"
rstest = "0.18"
tokio-test = "0.4"

[profile.release]
lto = true
codegen-units = 1
panic = "abort"
opt-level = 3
EOL

# 11. Create basic Rust source files
print_section "Creating Basic Source Files"
echo -e "${GREEN}Creating basic Rust source files...${NC}"

cat > src/main.rs <<'EOL'
use btpc_quantum_resistant_chain::{
    blockchain::QuantumResistantBlockchain,
    config::Config,
    network::P2PManager,
};
use anyhow::Result;
use clap::Parser;
use log::info;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "config/mainnet.toml")]
    config: String,

    #[arg(long)]
    mine: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    info!("Starting BTPC Quantum-Resistant Blockchain...");

    // Load configuration
    let config = Config::load(&cli.config)?;
    info!("Loaded configuration from: {}", cli.config);

    // Initialize blockchain
    let blockchain = QuantumResistantBlockchain::new(config.network_type).await?;
    info!("Blockchain initialized");

    // Initialize network
    let p2p_manager = P2PManager::new(config.network_config).await?;
    info!("P2P network initialized");

    if cli.mine {
        info!("Starting in mining mode...");
        // Start mining service
    }

    info!("BTPC Quantum-Resistant Blockchain is running!");

    // Keep the main thread alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");

    Ok(())
}
EOL

cat > src/lib.rs <<'EOL'
pub mod blockchain;
pub mod network;
pub mod crypto;
pub mod database;
pub mod consensus;
pub mod config;
pub mod models;
pub mod error;

pub use error::{BlockchainError, Result};
EOL

# 12. Build the project
print_section "Building the Project"
echo -e "${GREEN}Building the quantum-resistant blockchain...${NC}"
cargo build --release
check_success "Cargo build"

# 13. Generate documentation
echo -e "${GREEN}Generating documentation...${NC}"
cargo doc --no-deps
check_success "Documentation generation"

# 14. Create development scripts
print_section "Creating Development Scripts"
echo -e "${GREEN}Creating development utility scripts...${NC}"

cat > scripts/start_testnet.sh <<'EOL'
#!/bin/bash
cargo run --release -- --config config/testnet.toml
EOL

cat > scripts/start_regtest.sh <<'EOL'
#!/bin/bash
cargo run --release -- --config config/regtest.toml --mine
EOL

cat > scripts/clean.sh <<'EOL'
#!/bin/bash
cargo clean
rm -rf data/ logs/ target/
EOL

chmod +x scripts/*.sh

# 15. Final setup
print_section "Setup Complete"
echo -e "${GREEN}🎉 BTPC Quantum-Resistant Blockchain setup complete!${NC}"
echo -e ""
echo -e "${YELLOW}Next steps:${NC}"
echo -e "1. Open the project in PyCharm"
echo -e "2. Set up the Rust plugin if not already installed"
echo -e "3. Configure the Rust toolchain in PyCharm:"
echo -e "   - Go to Preferences > Languages & Frameworks > Rust"
echo -e "   - Set the toolchain to the version installed by rustup"
echo -e "4. Run configurations are pre-created:"
echo -e "   - 'Mainnet Node' for main network"
echo -e "   - 'Testnet Node' for test network"
echo -e "   - 'Regtest Miner' for local mining"
echo -e "5. Start developing your quantum-resistant blockchain!"
echo -e ""
echo -e "${GREEN}Quick commands:${NC}"
echo -e "  ./scripts/start_testnet.sh    # Start testnet node"
echo -e "  ./scripts/start_regtest.sh    # Start regtest miner"
echo -e "  ./scripts/clean.sh            # Clean build artifacts"
echo -e "  cargo test -- --nocapture     # Run tests with output"

# Open PyCharm if installed
if command -v pycharm &> /dev/null; then
    echo -e "${GREEN}Opening project in PyCharm...${NC}"
    pycharm .
elif command -v charm &> /dev/null; then
    echo -e "${GREEN}Opening project in PyCharm...${NC}"
    charm .
else
    echo -e "${YELLOW}PyCharm not found in PATH. Please open the project manually.${NC}"
fi