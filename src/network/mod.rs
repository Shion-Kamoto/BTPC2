//! Network module: protocol types/messages and sync management.
//!
//! This module re-exports the public protocol and sync types so other code
//! can import from `crate::network::{...}` without reaching into submodules.

pub mod protocol;
pub mod sync;

// ---- Re-exports: Protocol layer ----
pub use self::protocol::{
    AddrMessage, Block, BlockHeader, GetAddrMessage, GetDataMessage, Hash, HeadersMessage,
    InvMessage, InventoryVector, MessageBuilder, MessageHeader, NetworkAddress, NetworkMessage,
    PeerInfo, PingMessage, PongMessage, ProtocolError, Transaction, TxInput, TxOutput,
    VerackMessage, VersionMessage,
};

// ---- Re-exports: Sync layer ----
pub use self::sync::{BlockLocator, SyncError, SyncManager, SyncScheduler, SyncState, SyncStatus};
