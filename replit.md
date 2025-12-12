# Programming Lightning: Intro to Payment Channels

## Overview

This is an educational workshop teaching Bitcoin Lightning Network development through hands-on Rust programming exercises. The project guides learners through building payment channel primitives, from basic Bitcoin transaction concepts to implementing HTLC (Hash Time-Locked Contract) transactions and Lightning-specific key derivation schemes.

The workshop uses a tutorial-driven approach where users read markdown files in `.tutorial/` and complete corresponding exercises in `src/exercises/`. The codebase leverages the `rust-bitcoin` library for Bitcoin primitives and transaction building.

## User Preferences

Preferred communication style: Simple, everyday language.

## System Architecture

### Project Structure
- **Tutorial Content**: Located in `.tutorial/` as numbered markdown files (e.g., `1.01-welcome.md`, `1.02-setup.md`)
- **Exercise Code**: Located in `src/exercises/` containing Rust implementations
- **Key Files**:
  - `src/exercises/exercises.rs` - Main exercise implementations
  - `src/exercises/transactions.txt` - Transaction data for exercises
  - `start.sh` - Environment setup script for bitcoind

### Core Components

**Bitcoin Transaction Building**
- Uses `rust-bitcoin` library for all Bitcoin primitives
- Implements P2WPKH (Pay-to-Witness-Public-Key-Hash) and P2WSH (Pay-to-Witness-Script-Hash) scripts
- Supports multisig witness scripts (2-of-2 for channel funding)

**Lightning Key Management**
- Hierarchical deterministic (HD) wallet structure following BIP32
- Key derivation paths for: Node keys, Channel keys, Commitment keys
- Basepoints: Revocation, Payment, Delayed Payment, HTLC
- Per-commitment secrets derived from commitment seed

**Lightning Channel Constructs**
- Funding transactions (2-of-2 multisig)
- Commitment transactions with asymmetric outputs
- `to_local` outputs with revocation and delayed spending paths
- `to_remote` outputs as simple P2WPKH
- HTLC outputs with timeout/success paths

**Timelock Implementation**
- Absolute timelocks via `OP_CHECKLOCKTIMEVERIFY`
- Relative timelocks via `OP_CHECKSEQUENCEVERIFY`
- Obscured commitment numbers in locktime/sequence fields

### Design Patterns

**Asymmetric Commitment Transactions**
- Each party holds their own version of each commitment state
- Self-paying outputs include revocation paths for penalty mechanism
- Remote-paying outputs are simple P2WPKH

**Penalty Mechanism**
- Revocation keys derived from both parties' cryptographic material
- Old state publication allows counterparty to claim all funds
- Timelocked self-outputs provide window for revocation claims

**HTLC Structure**
- Second-stage transactions (HTLC-Timeout, HTLC-Success)
- Separates absolute timelock (payment expiry) from relative timelock (revocation window)

## External Dependencies

### Bitcoin Core (bitcoind)
- Runs on regtest network for local testing
- Provides transaction broadcast and block mining capabilities
- Accessed via shell aliases: `mine`, `sendtx`, `decodetx`, `gettx`, `getutxo`

### Rust Libraries
- `rust-bitcoin` - Bitcoin primitives, transaction building, script construction
- Standard Rust cryptographic libraries for SHA256, HASH160

### Development Environment
- Replit-based execution environment
- Pre-configured bitcoind binaries and wallet setup
- Shell environment with helpful Bitcoin CLI aliases