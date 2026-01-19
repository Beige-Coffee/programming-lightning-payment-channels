# Programming Lightning: An Introduction to Payment Channels

Welcome to **Programming Lightning** - an educational resource that teaches developers and technically-inclined individuals how the Lightning Network works by coding important pieces of the protocol from scratch.

Inspired by *[Programming Bitcoin](https://github.com/jimmysong/programmingbitcoin)*, this course provides hands-on, protocol-level learning to help you deeply understand Lightning's inner workings, preparing you to contribute to Lightning implementations, protocol design, or application development.

## About This Module

**Intro to Payment Channels** is the first module of the larger Programming Lightning course. It guides students through building a simple off-chain wallet and Lightning payment channel from scratch using Rust and the `rust-bitcoin` library.

By the end of this course, your implementation will pass some of the major [BOLT 3 test vectors](https://github.com/lightning/bolts/blob/master/03-transactions.md#appendix-b-funding-transaction-test-vectors), meaning you will have successfully implemented functionality such as:

- Key derivation
- Obscured commitment numbers
- HTLC second-stage transactions (success and timeout)
- Revocation mechanisms

## Prerequisites

This course assumes you have a working understanding of:
- Bitcoin transactions and Script
- Basic cryptography concepts
- Programming fundamentals (Rust knowledge helpful but not required)

If you'd like to brush up beforehand, check out these resources:
- [Learn Me a Bitcoin: Script](https://learnmeabitcoin.com/technical/script/) (Free)
- [Learn Me a Bitcoin: Transactions](https://learnmeabitcoin.com/technical/transaction/) (Free)
- [Base58: Bitcoin Transactions Course](https://www.udemy.com/course/base58-bitcoin-transactions-one/) ($120)

## Getting Started

You have two options for completing this course:

### Option 1: Replit (Recommended)

The course is optimized for Replit, where each Repl comes with Bitcoin Core running in the background. This allows you to easily generate Lightning transactions, broadcast them, and decode raw transactions as you complete exercises.

**[Launch on Replit →](https://replit.com/@austin-f/Programming-Lightning-Intro-to-Payment-Channels)**

### Option 2: Run Locally

#### Prerequisites
- Rust 1.70 or higher
- Git

#### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/Beige-Coffee/programming-lightning-payment-channels.git
   cd programming-lightning-payment-channels
   ```

2. **Install dependencies**
   ```bash
   cargo build
   ```

3. **Run the tests**
   ```bash
   # Run tests with your exercise implementations
   cargo test

   # Run tests with solution code (to see expected behavior)
   cargo test --features use-solutions
   ```

## Course Structure

The course is located within the `.tutorial` folder. Each section includes:
- Detailed explanations with diagrams
- Hands-on coding exercises
- Tests to verify your implementations

## How to Use This Course

1. **Read the Tutorial**: Open the `.tutorial` folder and start with `1.0-introduction.md`
2. **Complete Exercises**: Work through exercises in `src/exercises/`
3. **Run Tests**: Verify your implementations with `cargo test`
4. **Check Solutions**: If stuck, reference `src/solutions/` or run `cargo test --features use-solutions`

## What's Next

With continued grant support from Spiral and HRF through 2026, future modules will focus on:

- **Lightning Payments**: Authentication, onion routing, and gossip protocol
- **Invoices**: BOLT 11 and BOLT 12 implementations
- **Advanced Features**: Taproot channels, splicing, dual-funding, etc.

## Contributing

Feedback and corrections are welcome! Please feel free to:
- Open an issue for bugs or suggestions
- Submit a pull request for improvements
- Reach out directly at hello@programminglightning.com

## Acknowledgments

This course was made possible through grants from:
- [Spiral](https://spiral.xyz/)
- [Human Rights Foundation (HRF)](https://hrf.org/)
