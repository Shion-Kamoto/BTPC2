//! Binary entry point for btpc-quantum-resistant-chain.

use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use tokio::signal;

// ----- Crate imports -----
use btpc_quantum_resistant_chain::database::utxo_set::MemoryUTXOStorage;
use btpc_quantum_resistant_chain::database::{DatabaseConfig, DatabaseManager};
use btpc_quantum_resistant_chain::network::{SyncManager, SyncScheduler, SyncState};

#[derive(Debug, Clone)]
struct NodeConfig {
    /// How often the sync scheduler ticks, in seconds.
    sync_interval_secs: u64,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            sync_interval_secs: 5,
        }
    }
}

impl NodeConfig {
    fn from_env_args() -> Self {
        let mut cfg = Self::default();
        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--sync-interval-secs" => {
                    if let Some(val) = args.next() {
                        match u64::from_str(&val) {
                            Ok(n) => cfg.sync_interval_secs = n,
                            Err(_) => eprintln!(
                                "Invalid --sync-interval-secs: {} (default {})",
                                val, cfg.sync_interval_secs
                            ),
                        }
                    } else {
                        eprintln!(
                            "Missing value after --sync-interval-secs (default {})",
                            cfg.sync_interval_secs
                        );
                    }
                }
                "--help" | "-h" => {
                    print_help_and_exit();
                }
                other => {
                    eprintln!("Unknown argument: {}", other);
                    print_help_and_exit();
                }
            }
        }

        cfg
    }
}

fn print_help_and_exit() -> ! {
    eprintln!(
        "\
btpc-quantum-resistant-chain

USAGE:
  btpc-quantum-resistant-chain [FLAGS]

FLAGS:
  --sync-interval-secs <u64>   How often the sync scheduler ticks (default 5)
  -h, --help                   Show this help and exit
"
    );
    std::process::exit(0);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[allow(unused)]
    {
        // let _ = env_logger::builder().is_test(false).try_init();
    }

    let cfg = NodeConfig::from_env_args();
    log::info!(
        "Starting node with sync interval: {}s",
        cfg.sync_interval_secs
    );

    // --- DatabaseManager using MemoryUTXOStorage ---
    let storage = Box::new(MemoryUTXOStorage::new());

    let db_cfg = DatabaseConfig {
        data_dir: "./data".to_string().into(),
        // change path if needed
        max_cache_size: 10_000,
        // tune cache size for your workload
    };

    let db_manager = Arc::new(DatabaseManager::new(storage, db_cfg));
    // ------------------------------------------------

    let sync_manager = Arc::new(SyncManager::new(db_manager));

    let scheduler = SyncScheduler::new(
        Arc::clone(&sync_manager),
        Duration::from_secs(cfg.sync_interval_secs),
    );

    tokio::spawn(async move {
        scheduler.start().await;
    });

    let sm_for_log = Arc::clone(&sync_manager);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            let state: SyncState = sm_for_log.get_state();
            log::info!(
                "sync status = {:?}, height {}/{} ({:.1}%), peers={}, downloaded={}",
                state.status,
                state.current_height,
                state.target_height,
                state.progress,
                state.peers_connected,
                state.blocks_downloaded
            );
        }
    });

    log::info!("Node running. Press Ctrl+C to stop.");
    signal::ctrl_c().await?;
    log::info!("Shutdown signal received. Exiting…");
    Ok(())
}
