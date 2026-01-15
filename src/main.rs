#![allow(dead_code, unused_imports, unused_variables, unused_must_use)]

use clap::{Parser, Subcommand};
use sha2::{Sha256, Digest};
use ripemd::{Ripemd160};

// Re-export commonly used external types
pub use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey, All};
pub use bitcoin::Network;

// Module declarations - pulling from exercises or solutions folder
// 
// By default, modules are loaded from src/exercises/ (for students to complete).
// To use src/solutions/ instead (e.g., for testing or development), enable the
// "use-solutions" feature flag:
//
//   cargo test --features use-solutions
//   cargo build --features use-solutions
//
#[cfg(not(feature = "use-solutions"))]
#[path = "exercises/types.rs"]
pub mod types;

#[cfg(feature = "use-solutions")]
#[path = "solutions/types.rs"]
pub mod types;

#[cfg(not(feature = "use-solutions"))]
#[path = "exercises/keys/mod.rs"]
pub mod keys;

#[cfg(feature = "use-solutions")]
#[path = "solutions/keys/mod.rs"]
pub mod keys;

#[cfg(not(feature = "use-solutions"))]
#[path = "exercises/scripts/mod.rs"]
pub mod scripts;

#[cfg(feature = "use-solutions")]
#[path = "solutions/scripts/mod.rs"]
pub mod scripts;

#[cfg(not(feature = "use-solutions"))]
#[path = "exercises/transactions/mod.rs"]
pub mod transactions;

#[cfg(feature = "use-solutions")]
#[path = "solutions/transactions/mod.rs"]
pub mod transactions;

#[cfg(not(feature = "use-solutions"))]
#[path = "exercises/workflows.rs"]
pub mod workflows;

#[cfg(feature = "use-solutions")]
#[path = "solutions/workflows.rs"]
pub mod workflows;

// Internal utilities
pub mod internal;

// Interactive CLI modules
pub mod interactive;

#[cfg(test)]
mod tests;

// Re-export main types for convenience
pub use types::*;
pub use keys::derivation::*;
pub use keys::commitment::*;
pub use scripts::funding::*;
pub use scripts::commitment::*;
pub use scripts::htlc::*;
pub use transactions::fees::*;
pub use transactions::commitment::*;
pub use transactions::htlc::*;
pub use workflows::*;

/// Main CLI structure
#[derive(Parser)]
#[command(name = "Programming Lightning CLI")]
#[command(version = "1.0")]
#[command(about = "CLI for Programming Lightning", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// CLI Subcommands
#[derive(Subcommand)]
enum Commands {
    /// Create a Funding Transaction for a Lightning channel
    Funding,
    /// Create a commitment transaction for a Lightning channel
    Commitment {
        #[arg(short = 't', long, help = "Funding Tx ID")]
        funding_txid: String,
    },
    /// Create a commitment transaction with HTLC for a Lightning channel
    Htlc {
        #[arg(short = 't', long, help = "Funding Tx ID")]
        funding_txid: String,
    },
    /// Create an HTLC Timeout for a Lightning channel
    HtlcTimeout {
        #[arg(short = 't', long, help = "Commitment Tx ID")]
        commitment_txid: String,
    },
    SimpleHtlc,
    SimpleHtlcClaim {
        #[arg(short = 't', long, help = "Simple HTLC Tx ID")]
        simple_htlc_txid: String,
    },
    /// Calculate SHA256 hash of hex input
    Sha256 {
        #[arg(short = 'd', long, help = "Input string to hash (hex)")]
        input_string: String,
    },
    
    /// Calculate RIPEMD160(SHA256()) hash
    RipemdSha {
        #[arg(short = 'd', long, help = "Input string to hash")]
        input_string: String,
    },
    
    /// Convert string to hex
    ToHex {
        #[arg(short = 'd', long, help = "Input string to convert to hex")]
        input_string: String,
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    
    match &cli.command {
        Commands::Funding => {
            interactive::funding::run().await;
        },
        Commands::Commitment { funding_txid } => {
            interactive::commitment::run(funding_txid.clone()).await;
        },
        Commands::Htlc { funding_txid } => {
            interactive::htlc::run(funding_txid.clone()).await;
        },
        Commands::HtlcTimeout { commitment_txid } => {
            interactive::htlc_timeout::run(commitment_txid.clone()).await;
        },
        Commands::SimpleHtlc => {
            interactive::simple_htlc::run().await;
        },
        Commands::SimpleHtlcClaim { simple_htlc_txid } => {
            interactive::simple_htlc_claim::run(simple_htlc_txid.clone()).await;
        },
        Commands::Sha256 { input_string } => {
            let mut hasher = Sha256::new();
            let data = hex::decode(input_string).unwrap();
            hasher.update(&data);
            let result = hasher.finalize();
            println!("SHA256 Hash: {:x}", result);
        },
        
        Commands::RipemdSha { input_string } => {
            let mut sha_hasher = Sha256::new();
            let bytes = input_string.clone().into_bytes();
            sha_hasher.update(&bytes);
            let sha_result = sha_hasher.finalize();
            
            let mut ripemd_hasher = Ripemd160::new();
            ripemd_hasher.update(sha_result);
            let ripemd_result = ripemd_hasher.finalize();
            println!("RIPEMD160(SHA256()) Hash: {:x}", ripemd_result);
        },
        
        Commands::ToHex { input_string } => {
            let data = hex::encode(input_string);
            println!("Hex: {:?}", data);
        }
    }
}
