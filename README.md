# BTPC Quantum-Resistant Blockchain

Project overview:

BTPC2 is a work‑in‑progress implementation of a quantum‑resistant blockchain. The project uses the Rust programming language and is organized into several crates that collectively implement block creation, Merkle tree hashing, UTXO management, a custom peer‑to‑peer network protocol, and proof‑of‑work consensus. Its goal is to build a small, modular blockchain for educational or research purposes, using SHA‑512 for hashing and simple cryptographic primitives.

Repository metadata:

The project is hosted on GitHub under the public repository Shion‑Kamoto/BTPC2. The default branch is master, and the codebase is only about 94 KB in size【561494489005035†L19-L29】. As of 3 September 2025 there are no open issues【506881018627247†L0-L12】. A GitHub Actions workflow has been configured to run formatting (cargo fmt), linting (cargo clippy with -D warnings) and the test suite on every push or pull request. This continuous‑integration setup ensures that contributions conform to the project’s coding standards and that the test suite remains green.

Commit history:

The repository has a very short history with only four commits. The initial commit, labelled “TEST BTPC”, sets up the repository and test harness【234344612045487†L4-L14】. Two subsequent commits have the message “updated1” but do not describe any particular changes【234344612045487†L17-L32】. The most recent commit, “chore: add CI, clippy config, and pre‑commit hook”, introduces a GitHub Actions workflow, a Clippy configuration file and a pre‑commit script【234344612045487†L42-L54】. These changes suggest that the maintainer is preparing the project for more structured development and enforcing code quality through automated checks.

Project structure:

The blockchain crate defines BlockHeader and Block, implements Merkle trees for transaction hash aggregation, and contains reward logic and supply emission schedules.

The network crate defines a custom P2P protocol. It includes version handshakes, ping/pong messages, inventory and data requests, and framing of wire messages with checksums. Recent clean‑ups removed unused re‑exports and aligned type names.

The crypto crate provides low‑level primitives such as SHA‑512 hashing and key handling. It defines domain‑specific types like Hash and helper functions such as Hash::from_bytes([u8;64]).

The database crate implements a UTXO set, with in‑memory and RocksDB back‑ends, and round‑trip tests to ensure consistency.

The consensus crate contains proof‑of‑work and difficulty adjustment logic.

Entry points in main.rs and lib.rs glue these components together and expose them as a library.

Current status and future directions:

All existing tests pass, indicating that the core functionality works correctly. The newly added CI pipeline ensures that formatting, lint checks and tests are run automatically on each contribution. While the project is still small and lacks a full implementation, it provides a solid foundation for a more comprehensive quantum‑resistant blockchain. Future work could include expanding network features (peer discovery and block propagation), enhancing the cryptographic toolkit, adding transaction scripting and wallet functionality, and refining consensus mechanisms. The repository currently lacks a licence file; adding one would clarify usage rights for potential contributors.
## Building

```bash
cargo build --release
