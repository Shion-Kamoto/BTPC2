# BTPC Quantum-Resistant Blockchain

Project overview/structure:

The blockchain crate defines BlockHeader and Block, implements Merkle trees for transaction hash aggregation, and contains reward logic and supply emission schedules.

The network crate defines a custom P2P protocol. It includes version handshakes, ping/pong messages, inventory and data requests, and framing of wire messages with checksums. Recent clean‑ups removed unused re‑exports and aligned type names.

The crypto crate provides low‑level primitives such as SHA‑512 hashing and key handling. It defines domain‑specific types like Hash and helper functions such as Hash::from_bytes([u8;64]).

The database crate implements a UTXO set, with in‑memory and RocksDB back‑ends, and round‑trip tests to ensure consistency.

The consensus crate contains proof‑of‑work and difficulty adjustment logic.

Entry points in main.rs and lib.rs glue these components together and expose them as a library.


```bash
cargo build --release
