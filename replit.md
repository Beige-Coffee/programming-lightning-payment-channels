# Programming Lightning: Intro to Payment Channels

## Overview

This is an educational Rust project that teaches developers how the Lightning Network works by implementing key pieces of the protocol from scratch. Students build a simple off-chain wallet and Lightning payment channel, working through exercises that pass BOLT 3 test vectors for key derivation, obscured commitment numbers, HTLC transactions, and revocation mechanisms.

The course follows a progressive structure where students implement cryptographic key management, Bitcoin scripts, and transaction construction to understand Lightning's fairness protocol.

## User Preferences

Preferred communication style: Simple, everyday language.

## System Architecture

### Project Structure
- **`src/exercises/`** - Contains student exercises organized by topic:
  - `keys/` - Key derivation and channel key management
  - `scripts/` - Bitcoin Script construction (funding, commitment, HTLC scripts)
  - `transactions/` - Transaction building (funding, commitment, HTLC timeout)
  - `transactions.txt` - Storage for generated transaction data during exercises

- **`src/interactive/`** - Command-line tools for generating and testing transactions:
  - `funding.rs` - Generates funding transactions
  - `commitment.rs` - Generates commitment transactions
  - `htlc.rs` - Generates HTLC commitment transactions
  - `htlc_timeout.rs` - Generates HTLC timeout transactions

- **`src/internal/`** - Helper utilities and Bitcoin Core client integration

- **`.tutorial/`** - Course content in Markdown with embedded diagrams

### Core Design Patterns

1. **BIP32 Hierarchical Deterministic Keys**: All channel keys derive from a single seed using HD key derivation, enabling deterministic wallet recovery.

2. **Asymmetric Commitment Transactions**: Each party maintains their own version of commitment transactions with mirrored fund distributions but different spending conditions.

3. **Revocation Mechanism**: Uses derived revocation keys combining basepoints and per-commitment points to enable punishment of cheating parties.

4. **HTLC Second-Stage Transactions**: Separates timelock requirements into HTLC Success and HTLC Timeout transactions to avoid conflicting absolute and relative timelocks.

### Technology Stack
- **Rust** - Primary programming language
- **rust-bitcoin** - Bitcoin primitives and transaction construction
- **Bitcoin Core (bitcoind)** - Runs on regtest for transaction broadcasting and validation
- **Cargo** - Build system and test runner

### Testing Approach
Tests validate implementations against BOLT 3 specification test vectors. Run tests with the Replit "Run" button or `cargo test`.

### Command-Line Interface
The project provides CLI commands for generating transactions:
- `cargo run -- funding` - Generate funding transaction
- `cargo run -- commitment -t <txid>` - Generate commitment transaction
- `cargo run -- htlc -t <txid>` - Generate HTLC commitment transaction
- `cargo run -- htlc-timeout -t <txid>` - Generate HTLC timeout transaction

## External Dependencies

### Bitcoin Core
- **bitcoind** runs as a background daemon on regtest network
- Started via `./start.sh` script
- Interacted with via `bitcoin-cli` commands
- Custom `mine` alias for block generation

### Rust Crates
- `bitcoin` (rust-bitcoin) - Core Bitcoin primitives
- `secp256k1` - Elliptic curve cryptography
- Standard async runtime for bitcoind client communication

### Network Configuration
- Runs on Bitcoin regtest (local test network)
- No external network dependencies required
- All transactions are local to the Replit environment