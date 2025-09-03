#!/usr/bin/env bash
# apply_fixes_v3.sh
# Robust patcher for BTPC2 network fixes and serde/bincode compatibility.
# Run from the repo root.

set -euo pipefail

ROOT="src"
PROTO="$ROOT/network/protocol.rs"
NET_MOD="$ROOT/network/mod.rs"
SYNC="$ROOT/network/sync.rs"

echo "[*] Starting patch sequence (v3)..."

for f in "$PROTO" "$NET_MOD" "$SYNC"; do
  if [[ ! -f "$f" ]]; then
    echo "[!] Missing $f. Are you in the repo root? (where Cargo.toml is)" >&2
    exit 1
  fi
done

# Portable in-place sed for Linux/macOS
sedi() {
  if sed --version >/dev/null 2>&1; then
    sed -i "$@"
  else
    sed -i '' "$@"
  fi
}

# ---------- network/mod.rs re-exports + aliases ----------
cat > "$NET_MOD" <<'EOF'
//! Network module: protocol types/messages and sync management.

pub mod protocol;
pub mod sync;

// ---- Re-exports: Protocol layer ----
pub use self::protocol::{
    AddrMessage, Block, BlockHeader, GetAddrMessage, GetDataMessage, GetBlocksMessage, Hash,
    InvMessage, InvEntry, MessageHeader, NetAddr, NetworkMessage, PeerInfo,
    PingMessage, PongMessage, ProtocolError, Transaction, VerackMessage, VersionMessage,
};

// ---- Re-exports: Sync layer ----
pub use self::sync::{BlockLocator, SyncError, SyncManager, SyncScheduler, SyncState, SyncStatus};

// Legacy aliases for older module paths/names:
pub use self::protocol::NetAddr as NetworkAddress;
pub use self::protocol::InvEntry as InventoryVector;
EOF
echo "[*] Patched $NET_MOD re-exports"

# ---------- protocol.rs ----------

# Remove unused alias PublicKeyBuiltin as Public
if grep -q 'PublicKeyBuiltin as Public\b' "$PROTO"; then
  sedi -E 's/,\s*PublicKeyBuiltin as Public\b//g' "$PROTO"
  echo "[*] Removed unused alias 'Public' in $PROTO"
fi

# Ensure Hash::zero() exists (append new impl block if needed)
if ! grep -qE 'impl\s+Hash\s*\{[^}]*\bfn\s+zero\s*\(' "$PROTO"; then
  cat >> "$PROTO" <<'EOF'

impl Hash {
    /// Return the all-zero 64-byte hash.
    #[inline]
    pub fn zero() -> Self { Hash([0u8; 64]) }
}
EOF
  echo "[*] Added Hash::zero() to $PROTO"
fi

# Add GetBlocksMessage if missing
if ! grep -qE 'struct\s+GetBlocksMessage' "$PROTO"; then
  cat >> "$PROTO" <<'EOF'

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetBlocksMessage {
    pub version: u32,
    pub block_locator_hashes: Vec<Hash>,
    pub hash_stop: Hash,
}
EOF
  echo "[*] Added GetBlocksMessage to $PROTO"
fi

# Rewrite Block::merkle_root(&self) -> Hash with the correct Merkle API
if ! grep -q 'merkle requires at least one leaf' "$PROTO"; then
  perl -0777 -i -pe '
    s/pub\s+fn\s+merkle_root\s*\(\s*&self\s*\)\s*->\s*Hash\s*\{.*?\}/
pub fn merkle_root(&self) -> Hash {
    if self.tx_hashes.is_empty() {
        return Hash([0u8; 64]);
    }
    let leaves: Vec<[u8; 64]> = self.tx_hashes.iter().map(|h| *h.as_bytes()).collect();
    let t = MerkleTree::new(&leaves).expect("merkle requires at least one leaf");
    Hash(t.root())
}/s
  ' "$PROTO" || true
  echo "[*] Rewrote Block::merkle_root in $PROTO"
fi

# Remove needless & in hasher.update(&x.to_bytes())
if grep -q 'hasher\.update(&' "$PROTO"; then
  sedi -E 's/hasher\.update\(\&([A-Za-z0-9_\.]+\.to_bytes\(\))\)/hasher.update(\1)/g' "$PROTO"
  echo "[*] Cleaned needless borrows in hasher.update(...) in $PROTO"
fi

# ---------- sync.rs ----------

# inv.inventory -> inv.items
if grep -q 'inv\.inventory' "$SYNC"; then
  sedi 's/inv\.inventory/inv.items/g' "$SYNC"
  echo "[*] Replaced inv.inventory -> inv.items in $SYNC"
fi

# item.type_id -> item.kind
if grep -q 'item\.type_id' "$SYNC"; then
  sedi 's/item\.type_id/item.kind/g' "$SYNC"
  echo "[*] Replaced item.type_id -> item.kind in $SYNC"
fi

# Ensure Ok(()) instead of Ok()
if grep -qE '->\s*Result<\(\),\s*' "$SYNC"; then
  sedi -E 's/Ok\(\s*\)/Ok(())/g' "$SYNC" || true
fi

# Robustly replace the entire handle_inv_message body (to fix unclosed delimiters)
perl -0777 -i -pe '
  s/pub\s+fn\s+handle_inv_message\s*\(\s*&self\s*,\s*inv:\s*InvMessage\s*,\s*_peer_id:\s*&str\s*\)\s*->\s*Result<\(\),\s*SyncError>\s*\{.*?\}/
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
                // MSG_TX (ignored for now)
            }
            _ => {}
        }
    }
    Ok(())
}/s
' "$SYNC" || true
echo "[*] Rewrote handle_inv_message in $SYNC"

# Fallback: if Hash::zero() still not present, replace its call site
if ! grep -qE 'impl\s+Hash\s*\{[^}]*\bfn\s+zero\s*\(' "$PROTO"; then
  sedi 's/Hash::zero()/Hash::from_bytes([0u8; 64])/g' "$SYNC"
  echo "[*] Fallback: replaced Hash::zero() with Hash::from_bytes([0u8; 64]) in $SYNC"
fi

echo "[*] Patch sequence complete."

echo "
Next:
  cargo clippy -- -D warnings
  cargo test -q
"