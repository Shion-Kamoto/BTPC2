//! Sync manager for headers/blocks with type-safe SHA-512 Hash newtype.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::database::DatabaseManager;
use crate::network::{GetBlocksMessage, Hash, InvMessage, PeerInfo, ProtocolError};

#[derive(Debug, Clone)]
pub enum SyncError {
    Protocol(ProtocolError),
    NoPeers,
    Timeout,
    InvalidChain,
    DatabaseError(String),
    AlreadySyncing,
}

impl From<ProtocolError> for SyncError {
    fn from(err: ProtocolError) -> Self {
        SyncError::Protocol(err)
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SyncError::Protocol(e) => write!(f, "Protocol error: {}", e),
            SyncError::NoPeers => write!(f, "No peers available for synchronization"),
            SyncError::Timeout => write!(f, "Synchronization timeout"),
            SyncError::InvalidChain => write!(f, "Invalid blockchain"),
            SyncError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            SyncError::AlreadySyncing => write!(f, "Already synchronizing"),
        }
    }
}

impl std::error::Error for SyncError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub current_height: u64,
    pub target_height: u64,
    pub progress: f64,
    pub status: SyncStatus,
    pub peers_connected: usize,
    pub blocks_downloaded: u64,
    pub bytes_transferred: u64,
    pub start_time: u64,
    pub estimated_time_remaining: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    Idle,
    DiscoveringPeers,
    FetchingHeaders,
    DownloadingBlocks,
    VerifyingBlocks,
    Completed,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct BlockLocator {
    pub hashes: Vec<Hash>,
    pub stop_hash: Hash,
}

impl BlockLocator {
    pub fn new(known_hashes: Vec<Hash>, stop_hash: Hash) -> Self {
        Self {
            hashes: known_hashes,
            stop_hash,
        }
    }

    pub fn to_getblocks(&self, version: u32) -> GetBlocksMessage {
        GetBlocksMessage {
            version,
            block_locator_hashes: self.hashes.clone(),
            hash_stop: self.stop_hash,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SyncManager {
    state: Arc<RwLock<SyncState>>,
    known_peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    active_peers: Arc<RwLock<HashSet<String>>>,
    block_queue: Arc<RwLock<VecDeque<Hash>>>,
    requested_blocks: Arc<RwLock<HashSet<Hash>>>,
    _db_manager: Arc<DatabaseManager>,
}

impl SyncManager {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        let state = SyncState {
            current_height: 0,
            target_height: 0,
            progress: 0.0,
            status: SyncStatus::Idle,
            peers_connected: 0,
            blocks_downloaded: 0,
            bytes_transferred: 0,
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            estimated_time_remaining: 0,
        };

        Self {
            state: Arc::new(RwLock::new(state)),
            known_peers: Arc::new(RwLock::new(HashMap::new())),
            active_peers: Arc::new(RwLock::new(HashSet::new())),
            block_queue: Arc::new(RwLock::new(VecDeque::new())),
            requested_blocks: Arc::new(RwLock::new(HashSet::new())),
            _db_manager: db_manager,
        }
    }

    pub fn get_state(&self) -> SyncState {
        self.state.read().unwrap().clone()
    }

    pub fn update_state(&self, update: impl FnOnce(&mut SyncState)) {
        let mut state = self.state.write().unwrap();
        update(&mut state);
    }

    pub fn add_peer(&self, peer: PeerInfo) {
        self.known_peers.write().unwrap().insert(peer.id(), peer);
    }

    pub fn remove_peer(&self, peer_id: &str) {
        self.known_peers.write().unwrap().remove(peer_id);
        self.active_peers.write().unwrap().remove(peer_id);
    }

    pub fn mark_peer_active(&self, peer_id: &str) {
        self.active_peers
            .write()
            .unwrap()
            .insert(peer_id.to_string());
    }

    pub fn get_best_peers(&self, count: usize) -> Vec<PeerInfo> {
        self.known_peers
            .read()
            .unwrap()
            .values()
            .filter(|p| p.is_valid())
            .take(count)
            .cloned()
            .collect()
    }

    pub async fn start_sync(&self) -> Result<(), SyncError> {
        {
            let mut state = self.state.write().unwrap();
            if state.status != SyncStatus::Idle {
                return Err(SyncError::AlreadySyncing);
            }
            state.status = SyncStatus::DiscoveringPeers;
        }

        self.discover_peers().await?;
        self.fetch_headers().await?;
        self.download_blocks().await?;

        self.update_state(|state| {
            state.status = SyncStatus::Completed;
            state.progress = 100.0;
        });

        Ok(())
    }

    async fn discover_peers(&self) -> Result<(), SyncError> {
        self.update_state(|state| {
            state.status = SyncStatus::DiscoveringPeers;
        });

        tokio::time::sleep(Duration::from_secs(2)).await;

        let peers = self.get_best_peers(5);
        if peers.is_empty() {
            return Err(SyncError::NoPeers);
        }

        self.update_state(|state| {
            state.peers_connected = peers.len();
        });

        Ok(())
    }

    async fn fetch_headers(&self) -> Result<(), SyncError> {
        self.update_state(|state| {
            state.status = SyncStatus::FetchingHeaders;
        });

        let locator = self.create_block_locator().await?;

        let peers = self.get_best_peers(3);
        for peer in peers {
            if let Err(e) = self.request_headers(&peer, &locator).await {
                log::warn!("Failed to get headers from peer {}: {}", peer.id(), e);
            }
        }

        Ok(())
    }

    async fn request_headers(
        &self,
        _peer: &PeerInfo,
        _locator: &BlockLocator,
    ) -> Result<(), SyncError> {
        tokio::time::sleep(Duration::from_secs(1)).await;
        Ok(())
    }

    async fn download_blocks(&self) -> Result<(), SyncError> {
        self.update_state(|state| {
            state.status = SyncStatus::DownloadingBlocks;
        });

        let blocks_to_download = self.get_blocks_to_download().await?;

        for block_hash in blocks_to_download {
            if let Err(e) = self.download_block(block_hash).await {
                log::warn!("Failed to download block: {}", e);
                continue;
            }

            self.update_state(|state| {
                state.blocks_downloaded += 1;
                if state.target_height > 0 {
                    state.progress =
                        (state.blocks_downloaded as f64 / state.target_height as f64) * 100.0;
                }
            });
        }

        Ok(())
    }

    async fn download_block(&self, block_hash: Hash) -> Result<(), SyncError> {
        {
            let mut requested = self.requested_blocks.write().unwrap();
            if requested.contains(&block_hash) {
                return Ok(());
            }
            requested.insert(block_hash);
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        self.process_block(block_hash).await?;

        self.requested_blocks.write().unwrap().remove(&block_hash);

        Ok(())
    }

    async fn process_block(&self, _block_hash: Hash) -> Result<(), SyncError> {
        // TODO: Verify/persist block (header/work, txs, UTXO, chain state).
        Ok(())
    }

    async fn create_block_locator(&self) -> Result<BlockLocator, SyncError> {
        Ok(BlockLocator::new(vec![], Hash::from_bytes([0u8; 64])))
    }

    async fn get_blocks_to_download(&self) -> Result<Vec<Hash>, SyncError> {
        Ok(vec![])
    }

    pub fn handle_inv_message(&self, inv: InvMessage, _peer_id: &str) -> Result<(), SyncError> {
        for item in inv.items {
            match item.kind {
                2 => {
                    // MSG_BLOCK
                    if !self.is_block_known(&item.hash) {
                        self.block_queue.write().unwrap().push_back(item.hash);
                    }
                }
                1 => {
                    // MSG_TX
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn is_block_known(&self, _block_hash: &Hash) -> bool {
        false
    }

    pub fn get_blocks_for_download(&self, max_count: usize) -> Vec<Hash> {
        let mut queue = self.block_queue.write().unwrap();
        let mut blocks = Vec::new();

        while let Some(block_hash) = queue.pop_front() {
            if blocks.len() >= max_count {
                break;
            }
            blocks.push(block_hash);
        }

        blocks
    }
}

pub struct SyncScheduler {
    sync_manager: Arc<SyncManager>,
    interval: Duration,
}

impl SyncScheduler {
    pub fn new(sync_manager: Arc<SyncManager>, interval: Duration) -> Self {
        Self {
            sync_manager,
            interval,
        }
    }

    /// Periodic loop. Avoids holding any non-Send guards across `.await`, so this future is Send.
    pub async fn start(self) {
        let mut interval = tokio::time::interval(self.interval);

        loop {
            interval.tick().await;

            let status = {
                // quick read; don't hold lock across await
                self.sync_manager.get_state().status
            };

            if status == SyncStatus::Idle {
                if let Err(e) = self.sync_manager.start_sync().await {
                    log::error!("Sync failed: {}", e);
                }
            }
        }
    }
}
