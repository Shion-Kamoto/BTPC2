//! P2P protocol types: messages, headers, peer info, and helper crypto.
//! Heavy Clippy cleanup, consistent `Hash` newtype, explicit errors.

use bincode;
use hex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha512};
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::blockchain::merkle::MerkleTree;
use crate::database::utxo_set::{hash_transaction as hash512_tx, OutPoint};

/// Public, 64-byte SHA-512 hash newtype (binary form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash(pub [u8; 64]);

impl Hash {
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }
    /// Clippy-friendly (consumes self for a `Copy` type).
    pub fn into_bytes(self) -> [u8; 64] {
        self.0
    }
}

/// Human-readable hex format for JSON, binary for bincode.
impl Serialize for Hash {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(self.0))
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
            if bytes.len() != 64 {
                return Err(serde::de::Error::custom("expected 64 bytes of hex"));
            }
            let mut arr = [0u8; 64];
            arr.copy_from_slice(&bytes);
            Ok(Hash(arr))
        } else {
            // Binary: accept exactly 64 raw bytes.
            let v: Vec<u8> = Deserialize::deserialize(deserializer)?;
            if v.len() != 64 {
                return Err(serde::de::Error::custom("expected 64 raw bytes"));
            }
            let mut arr = [0u8; 64];
            arr.copy_from_slice(&v);
            Ok(Hash(arr))
        }
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// Protocol-level error type.
#[derive(Debug, Clone)]
pub enum ProtocolError {
    InvalidMessage,
    InvalidSignature,
    InvalidVersion,
    InvalidNonce,
    SerializationError(String),
    NetworkError(String),
    PeerError(String),
    Timeout,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::InvalidMessage => write!(f, "Invalid message format"),
            ProtocolError::InvalidSignature => write!(f, "Invalid message signature"),
            ProtocolError::InvalidVersion => write!(f, "Invalid protocol version"),
            ProtocolError::InvalidNonce => write!(f, "Invalid nonce"),
            ProtocolError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            ProtocolError::NetworkError(e) => write!(f, "Network error: {}", e),
            ProtocolError::PeerError(e) => write!(f, "Peer error: {}", e),
            ProtocolError::Timeout => write!(f, "Timeout"),
        }
    }
}

impl std::error::Error for ProtocolError {}

/// Maximum message size (bytes) for safety.
pub const MAX_MESSAGE_SIZE: usize = 2 * 1024 * 1024; // 2 MiB

/// Wire protocol version.
pub const PROTOCOL_VERSION: u32 = 1;

/// Builtin minimal crypto primitives for demo network.
mod builtin_crypto {
    use super::*;

    /// Simple public key newtype (demo).
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
    pub struct PublicKeyBuiltin {
        pub key: [u8; 32],
    }

    impl PublicKeyBuiltin {
        pub fn as_bytes(&self) -> [u8; 32] {
            self.key
        }
    }

    /// SHA-512 helper.
    pub fn sha512_hash(data: &[u8]) -> [u8; 64] {
        let mut hasher = Sha512::new();
        hasher.update(data);
        let digest = hasher.finalize();
        let mut out = [0u8; 64];
        out.copy_from_slice(&digest);
        out
    }

    #[allow(dead_code)]
    pub fn sha512_hash_string(s: &str) -> [u8; 64] {
        sha512_hash(s.as_bytes())
    }
}

use builtin_crypto::{sha512_hash, PublicKeyBuiltin as PublicKey};

/// Compact transaction reference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvEntry {
    pub kind: u8, // 1 = block, 2 = tx
    pub hash: Hash,
}

/// Version handshake message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionMessage {
    pub version: u32,
    pub services: u64,
    pub timestamp: u64,
    pub receiver: NetAddr,
    pub sender: NetAddr,
    pub nonce: u64,
    pub user_agent: String,
    pub start_height: u32,
    pub relay: bool,
}

/// Verack message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerackMessage;

/// Ping/Pong.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PingMessage {
    pub nonce: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PongMessage {
    pub nonce: u64,
}

/// Address request / response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetAddrMessage;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AddrMessage {
    pub addresses: Vec<PeerInfo>,
}

/// Inventory / getdata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvMessage {
    pub items: Vec<InvEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetDataMessage {
    pub items: Vec<InvEntry>,
}

/// Block header (simplified).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlockHeader {
    pub version: u32,
    pub prev_block: Hash,
    pub merkle_root: Hash,
    pub time: u32,
    pub bits: u32,
    pub nonce: u32,
}

impl BlockHeader {
    pub fn hash(&self) -> Hash {
        let serialized = bincode::serialize(self).expect("header serialize");
        Hash(hash512_tx(&serialized))
    }
}

/// Block + transactions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    pub header: BlockHeader,
    pub tx_hashes: Vec<Hash>, // simplified: just tx ids
}

impl Block {
    pub fn merkle_root(&self) -> Hash {
        if self.tx_hashes.is_empty() {
            return Hash([0u8; 64]);
        }
        let leaves: Vec<[u8; 64]> = self.tx_hashes.iter().map(|h| *h.as_bytes()).collect();
        let t = MerkleTree::new(&leaves).expect("merkle requires at least one leaf");
        Hash(t.root())
    }
}

/// Transaction (compact demo form).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub inputs: Vec<OutPoint>,
    pub outputs: Vec<(OutPoint, u64)>, // (outpoint, value)
}

impl Transaction {
    pub fn txid(&self) -> Hash {
        let serialized = bincode::serialize(self)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
            .unwrap();
        Hash(hash512_tx(&serialized))
    }
}

/// Peer info record used in Addr.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeerInfo {
    pub address: SocketAddr,
    pub public_key: PublicKey,
    pub last_seen: u64,
    pub user_agent: String,
    pub version: u32,
    pub services: u64,
    pub connection_time: u64,
}

impl PeerInfo {
    pub fn new(address: SocketAddr, public_key: PublicKey, user_agent: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            address,
            public_key,
            last_seen: now,
            user_agent,
            version: PROTOCOL_VERSION,
            services: 0x01, // NODE_NETWORK
            connection_time: now,
        }
    }

    pub fn id(&self) -> String {
        let mut hasher = Sha512::new();
        hasher.update(self.address.to_string());
        hasher.update(self.public_key.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn is_valid(&self) -> bool {
        self.version >= PROTOCOL_VERSION && !self.user_agent.is_empty() && self.last_seen > 0
    }
}

/// A network address with service bits and timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetAddr {
    pub services: u64,
    pub ip: IpAddr,
    pub port: u16,
}

impl NetAddr {
    pub fn new(ip: IpAddr, port: u16) -> Self {
        Self {
            services: 0x01,
            ip,
            port,
        }
    }
}

/// All P2P messages in one enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetworkMessage {
    Version(VersionMessage),
    Verack(VerackMessage),
    Ping(PingMessage),
    Pong(PongMessage),
    GetAddr(GetAddrMessage),
    Addr(AddrMessage),
    Inv(InvMessage),
    GetData(GetDataMessage),
    Block(Block),
    // Add more: Tx, GetHeaders, Headers, Reject, etc.
}

impl NetworkMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        bincode::serialize(self).map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        bincode::deserialize(bytes).map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }

    pub fn checksum(&self) -> Result<Hash, ProtocolError> {
        let bytes = self.to_bytes()?;
        Ok(Hash(sha512_hash(&bytes)))
    }
}

/// Message header with checksum to guard payload integrity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageHeader {
    pub magic: u32,
    pub command: [u8; 12],
    pub length: u32,
    pub checksum: Hash,
}

impl MessageHeader {
    pub fn new(command: &str, payload: &[u8]) -> Result<Self, ProtocolError> {
        if payload.len() > MAX_MESSAGE_SIZE {
            return Err(ProtocolError::InvalidMessage);
        }
        let mut cmd = [0u8; 12];
        let b = command.as_bytes();
        if b.len() > 12 {
            return Err(ProtocolError::InvalidMessage);
        }
        cmd[..b.len()].copy_from_slice(b);
        let checksum = Hash(sha512_hash(payload));
        Ok(Self {
            magic: 0xD9B4BEF9, // bitcoin mainnet magic (demo)
            command: cmd,
            length: payload.len() as u32,
            checksum,
        })
    }

    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        Hash(sha512_hash(payload)) == self.checksum
    }
}

/// Framed wire message (header + payload).
#[derive(Debug, Clone, PartialEq)]
pub struct FramedMessage {
    pub header: MessageHeader,
    pub payload: Vec<u8>,
}

impl FramedMessage {
    pub fn new(command: &str, message: &NetworkMessage) -> Result<Self, ProtocolError> {
        let payload = message.to_bytes()?;
        let header = MessageHeader::new(command, &payload)?;
        Ok(Self { header, payload })
    }

    pub fn parse_message(
        header: &MessageHeader,
        payload: &[u8],
    ) -> Result<NetworkMessage, ProtocolError> {
        if !header.verify_checksum(payload) {
            return Err(ProtocolError::InvalidMessage);
        }
        if payload.len() > MAX_MESSAGE_SIZE {
            return Err(ProtocolError::InvalidMessage);
        }
        NetworkMessage::from_bytes(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn peer_info_valid() {
        let pk = PublicKey { key: [0u8; 32] };
        let addr: SocketAddr = "127.0.0.1:8333".parse().unwrap();
        let peer_info = PeerInfo::new(addr, pk, "test/1.0".to_string());
        assert!(peer_info.is_valid());
        let _id = peer_info.id();
    }

    #[test]
    fn checksum_len() {
        let ping_msg = NetworkMessage::Ping(PingMessage { nonce: 12345 });
        let checksum = ping_msg.checksum().unwrap();
        assert_eq!(checksum.as_bytes().len(), 64);
    }

    #[test]
    fn message_roundtrip() {
        let receiver = NetAddr {
            services: 1,
            ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 8333,
        };
        let sender = receiver.clone();

        let msg = NetworkMessage::Version(VersionMessage {
            version: PROTOCOL_VERSION,
            services: 1,
            timestamp: 1_700_000_000,
            receiver,
            sender,
            nonce: 42,
            user_agent: "btpc/0.1".to_string(),
            start_height: 0,
            relay: true,
        });

        let bytes = msg.to_bytes().expect("serialize");
        let restored = NetworkMessage::from_bytes(&bytes).expect("deserialize");
        assert_eq!(msg, restored);
        assert!(matches!(restored, NetworkMessage::Version(_)));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetBlocksMessage {
    pub version: u32,
    pub block_locator_hashes: Vec<Hash>,
    pub hash_stop: Hash,
}

impl Hash {
    /// Return the all-zero 64-byte hash.
    #[inline]
    pub fn zero() -> Self {
        Hash([0u8; 64])
    }
}
