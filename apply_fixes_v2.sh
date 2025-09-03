#!/usr/bin/env bash
# apply_fixes_v2.sh
# Purpose: Robustly apply code fixes to resolve the reported errors in BTPC2.
# Run from repository root (where Cargo.toml is).

set -euo pipefail

ROOT="src"
PROTO="$ROOT/network/protocol.rs"
NET_MOD="$ROOT/network/mod.rs"
SYNC="$ROOT/network/sync.rs"

echo "[*] Starting patch sequence..."

# ---------- Sanity checks ----------
for f in "$PROTO" "$NET_MOD" "$SYNC"; do
  if [[ ! -f "$f" ]]; then
    echo "[!] Missing $f. Are you in the repo root? (where Cargo.toml is)" >&2
    exit 1
  fi
done

# ---------- 0) Helper: Linux vs macOS sed ----------
sedi() {
  # Portable in-place sed (works on macOS and Linux)
  if sed --version >/dev/null 2>&1; then
    sed -i "$@"
  else
    # macOS/BSD
    sed -i '' "$@"
  fi
}

# ---------- 1) network/mod.rs re-exports ----------
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

# ---------- 2) protocol.rs fixes ----------

# 2a) Remove unused alias 'PublicKeyBuiltin as Public'
if grep -q 'PublicKeyBuiltin as Public\b' "$PROTO"; then
  sedi -E 's/,\s*PublicKeyBuiltin as Public\b//g' "$PROTO"
  echo "[*] Removed unused alias 'Public' in $PROTO"
fi

# 2b) Ensure Hash::zero() exists.
if ! grep -qE 'impl\s+Hash\s*\{[^}]*\bfn\s+zero\s*\(' "$PROTO"; then
  cat >> "$PROTO" <<'EOF'

impl Hash {
    /// Return the all-zero 64-byte hash.
    #[inline]
    pub fn zero() -> Self { Hash([0u8; 64]) }
}
EOF
  echo "[*] Added Hash::zero() in $PROTO (new impl block)"
fi

# 2c) Ensure GetBlocksMessage exists (used by sync).
if ! grep -qE 'struct\s+GetBlocksMessage' "$PROTO"; then
  cat >> "$PROTO" <<'EOF'

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetBlocksMessage {
    pub version: u32,
    pub block_locator_hashes: Vec<Hash>,
    pub hash_stop: Hash,
}
EOF
  echo "[*] Added GetBlocksMessage in $PROTO"
fi

# 2d) Rewrite Block::merkle_root to use MerkleTree::new(&[[u8;64]])
#     Step 1: If we already wrote the correct body once, skip.
if ! grep -q 'merkle requires at least one leaf' "$PROTO"; then
  # Try a brace-balanced replacement with awk:
  awk '
    BEGIN { in_fn=0; depth=0; }
    {
      if (!in_fn && $0 ~ /pub[ \t]+fn[ \t]+merkle_root[ \t]*\([ \t]*&self[ \t]*\)[ \t]*->[ \t]*Hash[ \t]*\{/) {
        # Print the replacement function body
        print "    pub fn merkle_root(&self) -> Hash {";
        print "        if self.tx_hashes.is_empty() {";
        print "            return Hash([0u8; 64]);";
        print "        }";
        print "        let leaves: Vec<[u8; 64]> = self.tx_hashes.iter().map(|h| *h.as_bytes()).collect();";
        print "        let t = MerkleTree::new(&leaves).expect(\"merkle requires at least one leaf\");";
        print "        Hash(t.root())";
        print "    }";
        in_fn=1;
        # Initialize depth to 1 for the function opening brace on this line
        line=$0;
        gsub(/[^{]/, "", line);
        depth = length(line); # number of '{' on the signature line
        # Now consume lines until we close the original function
        next;
      }

      if (in_fn) {
        # Track braces to skip old body
        open=gsub(/\{/, "{");
        close=gsub(/\}/, "}");
        depth += open - close;
        if (depth <= 0) {
          in_fn=0;
        }
        next;
      }

      # Default: print line
      print $0;
    }
  ' "$PROTO" > "$PROTO.tmp" && mv "$PROTO.tmp" "$PROTO"
  echo "[*] Attempted brace-balanced rewrite of Block::merkle_root in $PROTO"

  # Step 2 (fallback): If placeholder call remains, do a targeted Perl substitution.
  if grep -q 'MerkleTree::new(/\* &\[\[u8; 64\]\] \*/)' "$PROTO" || grep -q 'mt\.push' "$PROTO"; then
    perl -0777 -i -pe '
      s/let\s+mut\s+mt\s*=\s*MerkleTree::new\([^)]*\);\s*for\s*\(\s*h\s+in\s+&self\.tx_hashes\s*\)\s*\{\s*mt\.push\(h\.as_bytes\(\)\);\s*\}\s*Hash\(mt\.root\(\)\)/
        if self.tx_hashes.is_empty() {
            return Hash([0u8; 64]);
        }
        let leaves: Vec<[u8; 64]> = self.tx_hashes.iter().map(|h| *h.as_bytes()).collect();
        let t = MerkleTree::new(&leaves).expect("merkle requires at least one leaf");
        Hash(t.root())
      /sg;
    ' "$PROTO" || true
    echo "[*] Applied fallback substitution for merkle_root body"
  fi
fi

# 2e) Remove needless `&` in hasher.update(&x.to_bytes())
if grep -q 'hasher\.update(&' "$PROTO"; then
  sedi -E 's/hasher\.update\(\&([A-Za-z0-9_\.]+\.to_bytes\(\))\)/hasher.update(\1)/g' "$PROTO"
  echo "[*] Cleaned needless borrows in hasher.update(...) in $PROTO"
fi

# ---------- 3) network/sync.rs fixes ----------
# 3a) inv.inventory -> inv.items
if grep -q 'inv\.inventory' "$SYNC"; then
  sedi 's/inv\.inventory/inv.items/g' "$SYNC"
  echo "[*] Replaced inv.inventory -> inv.items in $SYNC"
fi

# 3b) item.type_id -> item.kind
if grep -q 'item\.type_id' "$SYNC"; then
  sedi 's/item\.type_id/item.kind/g' "$SYNC"
  echo "[*] Replaced item.type_id -> item.kind in $SYNC"
fi

# 3c) As a *safety net*, if Hash::zero() still fails to exist, replace the call site.
if ! grep -qE 'impl\s+Hash\s*\{[^}]*\bfn\s+zero\s*\(' "$PROTO"; then
  sedi 's/Hash::zero()/Hash::from_bytes([0u8; 64])/g' "$SYNC"
  echo "[*] Fallback: replaced Hash::zero() with Hash::from_bytes([0u8; 64]) in $SYNC"
fi

echo "[*] Patches applied successfully."

echo "
Next steps:
  cargo clippy -- -D warnings
  cargo test
"
