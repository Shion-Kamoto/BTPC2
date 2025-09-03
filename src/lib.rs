//! Crate root.

pub mod blockchain;
pub mod config;
pub mod crypto;
pub mod database;
pub mod error;
pub mod network;

// Remove broken re-exports that caused E0432:
// pub use blockchain::QuantumResistantBlockchain;
// pub use network::P2PManager;

// If you want public re-exports of known-good items, do it like this:
//
// pub use network::{
//     protocol::{Hash, NetworkMessage, PeerInfo, ProtocolError, MessageHeader, MessageBuilder},
//     sync::{SyncManager, SyncScheduler, SyncState, SyncStatus, SyncError, BlockLocator},
// };
