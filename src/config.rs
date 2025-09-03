use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

/// Preferred way to configure bincode going forward (replaces deprecated `bincode::config`).
#[allow(dead_code)]
pub fn bincode_options() -> impl bincode::Options {
    bincode::options()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum NetworkType {
    #[default]
    Mainnet,
    Testnet,
    Regtest,
}

impl NetworkType {
    pub fn default_port(&self) -> u16 {
        match self {
            NetworkType::Mainnet => 8333,
            NetworkType::Testnet => 18333,
            NetworkType::Regtest => 18444,
        }
    }

    pub fn magic_bytes(&self) -> u32 {
        match self {
            NetworkType::Mainnet => 0xD9B4_BEF9,
            NetworkType::Testnet => 0x0709_110B,
            NetworkType::Regtest => 0xDAB5_BFFA,
        }
    }

    pub fn default_rpc_port(&self) -> u16 {
        match self {
            NetworkType::Mainnet => 8334,
            NetworkType::Testnet => 18334,
            NetworkType::Regtest => 18445,
        }
    }

    pub fn genesis_timestamp(&self) -> u64 {
        match self {
            NetworkType::Mainnet => 1_231_006_505, // Bitcoin genesis timestamp
            NetworkType::Testnet | NetworkType::Regtest => 1_296_688_602,
        }
    }
}

/* ------------------------- Linear Decay Model Constants -------------------------
   These mirror the economics used in `blockchain::reward`:

   - 10-minute blocks => 52_560 blocks/year
   - Linear decay: 32.375 BTP -> 0.5 BTP over 24 years
   - Tail emission afterwards at 0.5 BTP
----------------------------------------------------------------------------- */
const BLOCKS_PER_YEAR: u64 = 52_560;
const DECAY_PERIOD_YEARS: u64 = 24;
const DECAY_PERIOD_BLOCKS: u64 = BLOCKS_PER_YEAR * DECAY_PERIOD_YEARS; // 1_261_440

// Initial reward in base units (no float math at runtime): 32.375 * 100_000_000
const INITIAL_BLOCK_REWARD_SATS: u64 = 3_237_500_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Directory for on-disk DB or cache artifacts used by the DB layer.
    /// Added for compatibility with callers constructing DatabaseConfig directly.
    pub data_dir: String,

    /// Upper bound cache size (in entries or implementation-defined units).
    /// Added for compatibility with the `main.rs` initializer.
    pub max_cache_size: usize,

    // Existing tuning fields kept as-is:
    pub cache_size_mb: usize,
    pub max_open_files: i32,
    pub compaction_style: String,
    pub write_buffer_size: usize,
    pub max_write_buffer_number: i32,
    pub target_file_size_base: u64,
    pub max_background_compactions: i32,
    pub max_background_flushes: i32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            // New fields defaulted sensibly
            data_dir: "./data".to_string(),
            max_cache_size: 10_000,

            // Existing defaults preserved
            cache_size_mb: 512,
            max_open_files: 512,
            compaction_style: "level".to_string(),
            write_buffer_size: 64 * 1024 * 1024, // 64MB
            max_write_buffer_number: 4,
            target_file_size_base: 64 * 1024 * 1024, // 64MB
            max_background_compactions: 4,
            max_background_flushes: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub listen_addr: SocketAddr,
    pub external_addr: Option<SocketAddr>,
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub message_timeout: Duration,
    pub peer_discovery_interval: Duration,
    pub dns_seeds: Vec<String>,
    pub enable_upnp: bool,
    pub ban_threshold: u32,
    pub ban_duration: Duration,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:8333".parse().unwrap(),
            external_addr: None,
            max_connections: 125,
            connection_timeout: Duration::from_secs(30),
            message_timeout: Duration::from_secs(120),
            peer_discovery_interval: Duration::from_secs(300),
            dns_seeds: vec![
                "seed.bitcoin.sipa.be".to_string(),
                "dnsseed.bitcoin.dashjr.org".to_string(),
                "seed.bitcoinstats.com".to_string(),
                "seed.bitcoin.jonasschnelli.ch".to_string(),
                "seed.btc.petertodd.org".to_string(),
            ],
            enable_upnp: true,
            ban_threshold: 100,
            ban_duration: Duration::from_secs(86_400), // 24 hours
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningConfig {
    pub enabled: bool,
    pub threads: usize,
    pub difficulty_target: u32,
    pub block_size_limit: usize,
    pub transaction_fee: u64,
    /// For linear model this is the **initial** block reward in base units.
    pub block_reward: u64,
    /// For linear model, this field is used as **decay period (blocks)**.
    /// (Kept name for backward compatibility.)
    pub halving_interval: u64,
    pub coinbase_maturity: u32,
    pub pow_algorithm: String,
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threads: 4,
            difficulty_target: 0x1f00_ffff, // initial PoW difficulty
            block_size_limit: 4 * 1024 * 1024, // 4MB
            transaction_fee: 1_000,         // base units
            block_reward: INITIAL_BLOCK_REWARD_SATS, // 32.375 BTP in base units
            halving_interval: DECAY_PERIOD_BLOCKS, // used as decay period blocks (1_261_440)
            coinbase_maturity: 100,
            pow_algorithm: "sha512".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    pub enabled: bool,
    pub listen_addr: SocketAddr,
    pub username: Option<String>,
    pub password: Option<String>,
    pub max_connections: usize,
    pub timeout: Duration,
    pub enable_cors: bool,
    pub cors_origin: Vec<String>,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            listen_addr: "127.0.0.1:8334".parse().unwrap(),
            username: None,
            password: None,
            max_connections: 10,
            timeout: Duration::from_secs(30),
            enable_cors: false,
            cors_origin: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_path: Option<PathBuf>,
    pub max_files: usize,
    pub max_size_mb: usize,
    pub enable_console: bool,
    pub enable_file: bool,
    pub log_network: bool,
    pub log_database: bool,
    pub log_consensus: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_path: Some(PathBuf::from("logs/btpc.log")),
            max_files: 7,
            max_size_mb: 100,
            enable_console: true,
            enable_file: true,
            log_network: true,
            log_database: true,
            log_consensus: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub enabled: bool,
    pub default_wallet: Option<String>,
    pub keypool_size: usize,
    pub transaction_fee: u64,
    pub reserve_balance: u64,
    pub avoid_reuse: bool,
    pub enable_psbt: bool,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_wallet: Some("default".to_string()),
            keypool_size: 1000,
            transaction_fee: 1_000,
            reserve_balance: 0,
            avoid_reuse: true,
            enable_psbt: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_tls: bool,
    pub cert_file: Option<PathBuf>,
    pub key_file: Option<PathBuf>,
    pub ca_file: Option<PathBuf>,
    pub require_client_cert: bool,
    pub max_request_size: usize,
    pub rate_limit_requests: u32,
    pub rate_limit_period: Duration,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_tls: false,
            cert_file: None,
            key_file: None,
            ca_file: None,
            require_client_cert: false,
            max_request_size: 16 * 1024 * 1024, // 16MB
            rate_limit_requests: 1000,
            rate_limit_period: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub network: NetworkType,
    pub data_dir: PathBuf,
    pub user_agent: String,
    pub database: DatabaseConfig,
    pub network_config: NetworkConfig,
    pub mining: MiningConfig,
    pub rpc: RpcConfig,
    pub logging: LoggingConfig,
    pub wallet: WalletConfig,
    pub security: SecurityConfig,
    pub enable_testnet_faucet: bool,
    pub prune_blocks: bool,
    pub prune_depth: u32,
    pub max_mempool_size: usize,
    pub mempool_expiry: Duration,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("./data"))
            .join("btpc");

        Self {
            network: NetworkType::Mainnet,
            data_dir,
            user_agent: "BTPC-QRC/0.1.0".to_string(),
            database: DatabaseConfig::default(),
            network_config: NetworkConfig::default(),
            mining: MiningConfig::default(),
            rpc: RpcConfig::default(),
            logging: LoggingConfig::default(),
            wallet: WalletConfig::default(),
            security: SecurityConfig::default(),
            enable_testnet_faucet: false,
            prune_blocks: false,
            prune_depth: 288,                    // ~2 days of blocks
            max_mempool_size: 300 * 1024 * 1024, // 300MB
            mempool_expiry: Duration::from_secs(14 * 24 * 60 * 60), // 14 days
        }
    }
}

impl Config {
    pub fn new(network: NetworkType, data_dir: Option<PathBuf>) -> Self {
        // Construct with the `network` set to avoid "field_reassign_with_default"
        let mut config = Self {
            network,
            ..Self::default()
        };

        if let Some(dir) = data_dir {
            config.data_dir = dir;
        }

        // Adjust network-specific settings
        match config.network {
            NetworkType::Testnet => {
                config.network_config.listen_addr =
                    format!("0.0.0.0:{}", config.network.default_port())
                        .parse()
                        .unwrap();
                config.rpc.listen_addr = format!("127.0.0.1:{}", config.network.default_rpc_port())
                    .parse()
                    .unwrap();

                // Keep linear model defaults, but you can lower reward here if desired.
                config.mining.block_reward = INITIAL_BLOCK_REWARD_SATS;
                config.enable_testnet_faucet = true;
            }
            NetworkType::Regtest => {
                config.network_config.listen_addr =
                    format!("0.0.0.0:{}", config.network.default_port())
                        .parse()
                        .unwrap();
                config.rpc.listen_addr = format!("127.0.0.1:{}", config.network.default_rpc_port())
                    .parse()
                    .unwrap();

                // Very low difficulty for regtest; keep reward matching linear model start.
                config.mining.difficulty_target = 0x207f_ffff;
                config.mining.block_reward = INITIAL_BLOCK_REWARD_SATS;
                config.mining.enabled = true; // Enable mining by default on regtest
            }
            NetworkType::Mainnet => { /* defaults */ }
        }

        config
    }

    pub fn from_file(path: &PathBuf) -> Result<Self, ConfigError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ConfigError::IoError(e.to_string()))?;

        let config: Config =
            toml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        Ok(config)
    }

    pub fn to_file(&self, path: &PathBuf) -> Result<(), ConfigError> {
        let content =
            toml::to_string_pretty(self).map_err(|e| ConfigError::SerializeError(e.to_string()))?;

        std::fs::write(path, content).map_err(|e| ConfigError::IoError(e.to_string()))?;

        Ok(())
    }

    pub fn get_network_magic(&self) -> u32 {
        self.network.magic_bytes()
    }

    pub fn get_network_port(&self) -> u16 {
        self.network.default_port()
    }

    pub fn get_rpc_port(&self) -> u16 {
        self.network.default_rpc_port()
    }

    pub fn get_data_subdir(&self, subdir: &str) -> PathBuf {
        self.data_dir.join(subdir)
    }

    pub fn get_blocks_dir(&self) -> PathBuf {
        self.get_data_subdir("blocks")
    }

    pub fn get_chainstate_dir(&self) -> PathBuf {
        self.get_data_subdir("chainstate")
    }

    pub fn get_wallets_dir(&self) -> PathBuf {
        self.get_data_subdir("wallets")
    }

    pub fn get_logs_dir(&self) -> PathBuf {
        self.get_data_subdir("logs")
    }

    pub fn get_config_file(&self) -> PathBuf {
        self.data_dir.join("config.toml")
    }

    pub fn get_peers_file(&self) -> PathBuf {
        self.data_dir.join("peers.json")
    }

    pub fn get_genesis_timestamp(&self) -> u64 {
        self.network.genesis_timestamp()
    }
}

#[derive(Debug)]
pub enum ConfigError {
    IoError(String),
    ParseError(String),
    SerializeError(String),
    ValidationError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(msg) => write!(f, "IO error: {}", msg),
            ConfigError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ConfigError::SerializeError(msg) => write!(f, "Serialize error: {}", msg),
            ConfigError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Utility functions for config management
pub fn get_default_config_path(network: NetworkType) -> PathBuf {
    let base_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("./data"))
        .join("btpc");

    match network {
        NetworkType::Mainnet => base_dir.join("config.toml"),
        NetworkType::Testnet => base_dir.join("testnet").join("config.toml"),
        NetworkType::Regtest => base_dir.join("regtest").join("config.toml"),
    }
}

pub fn create_default_config(network: NetworkType) -> Result<Config, ConfigError> {
    let config = Config::new(network, None);

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(&config.data_dir).map_err(|e| ConfigError::IoError(e.to_string()))?;

    // Create subdirectories
    for dir in ["blocks", "chainstate", "wallets", "logs"] {
        std::fs::create_dir_all(config.get_data_subdir(dir))
            .map_err(|e| ConfigError::IoError(e.to_string()))?;
    }

    // Save default config
    let config_path = config.get_config_file();
    config.to_file(&config_path)?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_creation() {
        let config = Config::default();
        assert_eq!(config.network, NetworkType::Mainnet);
        assert!(config.data_dir.exists() || config.data_dir.to_str().unwrap().contains("btpc"));
    }

    #[test]
    fn test_network_type_methods() {
        assert_eq!(NetworkType::Mainnet.default_port(), 8333);
        assert_eq!(NetworkType::Testnet.default_port(), 18333);
        assert_eq!(NetworkType::Regtest.default_port(), 18444);
        assert_eq!(NetworkType::Mainnet.magic_bytes(), 0xD9B4_BEF9);
    }

    #[test]
    fn test_config_serialization() -> Result<(), ConfigError> {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config = Config::default();
        config.to_file(&config_path)?;

        let loaded_config = Config::from_file(&config_path)?;
        assert_eq!(config.network, loaded_config.network);

        Ok(())
    }

    #[test]
    fn test_network_specific_config() {
        let testnet_config = Config::new(NetworkType::Testnet, None);
        assert_eq!(testnet_config.network, NetworkType::Testnet);
        assert!(testnet_config.enable_testnet_faucet);

        let regtest_config = Config::new(NetworkType::Regtest, None);
        assert_eq!(regtest_config.network, NetworkType::Regtest);
        assert!(regtest_config.mining.enabled);
    }
}
