#!/usr/bin/env bash
# btpc2_repo_patch.sh
# Purpose: Apply repo-wide fixes for BTPC2:
#  - Add/GetBlocksMessage and re-export it
#  - Fix network/mod.rs re-exports (and add legacy aliases)
#  - Rewrite Block::merkle_root to use MerkleTree::new(&[[u8; 64]])
#  - Ensure Hash::zero() exists (or keep call sites working)
#  - Fix sync.rs to use inv.items/item.kind and avoid Hash::zero() dependency
# Usage:
#   chmod +x btpc2_repo_patch.sh
#   ./btpc2_repo_patch.sh
# Then:
#   cargo clippy -- -D warnings
#   cargo test -q

set -euo pipefail

ROOT="src"
PROTO="$ROOT/network/protocol.rs"
NET_MOD="$ROOT/network/mod.rs"
SYNC="$ROOT/network/sync.rs"

echo "[*] BTPC2 patch starting..."

for f in "$PROTO" "$NET_MOD" "$SYNC"; do
  if [[ ! -f "$f" ]]; then
    echo "[!] Missing $f. Run this script from the repo root (where Cargo.toml is)." >&2
    exit 1
  fi
done

# Portable sed in-place
sedi() {
  if sed --version >/dev/null 2>&1; then
    sed -i "$@"
  else
    sed -i '' "$@"
  fi
}

# ------------------------------
# 1) network/mod.rs re-exports
# ------------------------------
cat > "$NET_MOD" <<'EOF'
//! Network module: protocol types/messages and sync management.

pub mod protocol;
pub mod sync;

// ---- Re-exports: Protocol layer ----
pub use self::protocol::{
    AddrMessage, Block, BlockHeader, GetAddrMessage, GetDataMessage, GetBlocksMessage, Hash,
    InvEntry, InvMessage, MessageHeader, NetAddr, NetworkMessage, PeerInfo,
    PingMessage, PongMessage, ProtocolError, Transaction, VerackMessage, VersionMessage,
};

// ---- Re-exports: Sync layer ----
pub use self::sync::{BlockLocator, SyncError, SyncManager, SyncScheduler, SyncState, SyncStatus};

// Legacy aliases for older module paths/names:
pub use self::protocol::NetAddr as NetworkAddress;
pub use self::protocol::InvEntry as InventoryVector;
EOF
echo "[*] Patched $NET_MOD re-exports and aliases"

# ------------------------------
# 2) protocol.rs patches
# ------------------------------

# 2a) Remove unused alias (PublicKeyBuiltin as Public) if present
if grep -q 'PublicKeyBuiltin as Public\b' "$PROTO"; then
  sedi -E 's/,\s*PublicKeyBuiltin as Public\b//g' "$PROTO"
  echo "[*] Removed unused alias 'Public' in $PROTO"
fi

# 2b) Ensure Hash::zero() exists (append impl block if not present)
if ! grep -qE 'impl\s+Hash\s*\{[^}]*\bfn\s+zero\s*\(' "$PROTO"; then
  cat >> "$PROTO" <<'EOF'

impl Hash {
    /// All-zero 64-byte hash convenience.
    #[inline]
    pub fn zero() -> Self { Hash([0u8; 64]) }
}
EOF
  echo "[*] Added Hash::zero() to $PROTO"
fi

# 2c) Add GetBlocksMessage if missing
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

# 2d) Rewrite Block::merkle_root(&self) -> Hash to use MerkleTree::new(&[[u8;64]]).
#     We do a brace-aware replacement with awk. If already patched, skip.
if ! grep -q 'merkle requires at least one leaf' "$PROTO"; then
  awk '
    BEGIN { in_fn=0; depth=0; }
    {
      if (!in_fn && $0 ~ /pub[ \t]+fn[ \t]+merkle_root[ \t]*\([ \t]*&self[ \t]*\)[ \t]*->[ \t]*Hash[ \t]*\{/) {
        print "    pub fn merkle_root(&self) -> Hash {";
        print "        if self.tx_hashes.is_empty() {";
        print "            return Hash([0u8; 64]);";
        print "        }";
        print "        let leaves: Vec<[u8; 64]> = self.tx_hashes.iter().map(|h| *h.as_bytes()).collect();";
        print "        let t = MerkleTree::new(&leaves).expect(\"merkle requires at least one leaf\");";
        print "        Hash(t.root())";
        print "    }";
        in_fn=1;
        depth=1; # we are inside the new function now
        next;
      }
      if (in_fn) {
        # Skip old body by tracking braces until balanced
        open=gsub(/\{/, "{"); close=gsub(/\}/, "}");
        depth += open - close;
        if (depth <= 0) { in_fn=0; }
        next;
      }
      print $0;
    }
  ' "$PROTO" > "$PROTO.tmp" && mv "$PROTO.tmp" "$PROTO"
  echo "[*] Rewrote Block::merkle_root in $PROTO"
fi

# Fix needless borrow in hasher.update(&x.to_bytes()) if any
if grep -q 'hasher\.update(&' "$PROTO"; then
  sedi -E 's/hasher\.update\(\&([A-Za-z0-9_\.]+\.to_bytes\(\))\)/hasher.update(\1)/g' "$PROTO"
  echo "[*] Cleaned needless borrows in hasher.update(...) in $PROTO"
fi

# ------------------------------
# 3) sync.rs patches
# ------------------------------

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

# Hash::zero() -> Hash::from_bytes([0u8; 64]) (avoid dependency if impl omitted)
if grep -q 'Hash::zero()' "$SYNC"; then
  sedi 's/Hash::zero()/Hash::from_bytes([0u8; 64])/g' "$SYNC"
  echo "[*] Replaced Hash::zero() call sites in $SYNC"
fi

# Ensure Result<(), _> returns use Ok(())
if grep -qE '->\s*Result<\(\),\s*' "$SYNC"; then
  # only replace bare Ok() (not Ok(something))
  sedi -E 's/\bOk\(\s*\)/Ok(())/g' "$SYNC" || true
fi

echo "[*] All patches applied."
echo "Next:"
echo "  cargo clippy -- -D warnings"
echo "  cargo test -q"