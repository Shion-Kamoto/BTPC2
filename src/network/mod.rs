//! Network module: protocol types/messages and sync management.

pub mod protocol;
pub mod sync;

// ---- Re-exports: Protocol layer ----
pub use self::protocol::{
    AddrMessage, Block, BlockHeader, GetAddrMessage, GetBlocksMessage, GetDataMessage, Hash,
    InvEntry, InvMessage, MessageHeader, NetAddr, NetworkMessage, PeerInfo, PingMessage,
    PongMessage, ProtocolError, Transaction, VerackMessage, VersionMessage,
};

// ---- Re-exports: Sync layer ----
pub use self::sync::{BlockLocator, SyncError, SyncManager, SyncScheduler, SyncState, SyncStatus};

// Legacy aliases for older module paths/names:
pub use self::protocol::InvEntry as InventoryVector;
pub use self::protocol::NetAddr as NetworkAddress;
