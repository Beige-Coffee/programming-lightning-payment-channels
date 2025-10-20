#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_must_use,
    non_snake_case
)]

// Re-export commonly used external types
pub use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey, All};
pub use bitcoin::Network;

// Module declarations - pulling from exercises folder
#[path = "exercises/types.rs"]
pub mod types;

#[path = "exercises/keys/mod.rs"]
pub mod keys;

#[path = "exercises/scripts/mod.rs"]
pub mod scripts;

#[path = "exercises/transactions/mod.rs"]
pub mod transactions;

#[path = "exercises/signing.rs"]
pub mod signing;

#[path = "exercises/workflows.rs"]
pub mod workflows;

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
pub use signing::*;
pub use workflows::*;

// Constants
pub const INITIAL_COMMITMENT_NUMBER: u64 = (1 << 48) - 1;

// ============================================================================
// DEMO BINARY
// ============================================================================

use bitcoin::{Transaction, TxIn, TxOut, OutPoint, Sequence, Witness, Amount};
use bitcoin::script::ScriptBuf;
use bitcoin::transaction::Version;
use bitcoin::locktime::absolute::LockTime;
use bitcoin::hashes::{Hash};

fn main() {
    println!("=== Lightning Network Channel Opening Demo ===\n");
    
    // Step 1: Initialize local node's keys
    let local_seed = [42u8; 32];
    let network = Network::Bitcoin;
    let local_keys_manager = new_keys_manager(local_seed, network);
    
    println!("✓ Local KeysManager initialized");
    
    // Step 2: Initialize remote node's keys
    let remote_seed = [99u8; 32];
    let remote_keys_manager = new_keys_manager(remote_seed, network);
    
    println!("✓ Remote KeysManager initialized");
    
    // Step 3: Get node public keys
    let secp = Secp256k1::new();
    let local_node_secret = local_keys_manager.get_node_secret();
    let local_node_pubkey = PublicKey::from_secret_key(&secp, &local_node_secret);
    
    let remote_node_secret = remote_keys_manager.get_node_secret();
    let remote_node_pubkey = PublicKey::from_secret_key(&secp, &remote_node_secret);
    
    println!("✓ Node public keys derived");
    
    // Step 4: Generate channel seed
    let channel_value_satoshis = 1_000_000;
    let nonce = [1u8; 32];
    
    let channel_seed = generate_channel_seed(
        channel_value_satoshis,
        &local_node_pubkey,
        &remote_node_pubkey,
        nonce,
    );
    
    println!("✓ Channel seed generated");

    let channel_index = 0;
    
    // Step 5: Derive channel keys
    let local_channel_keys = local_keys_manager.derive_channel_keys(channel_index.clone(), channel_seed);
    let remote_channel_keys = remote_keys_manager.derive_channel_keys(channel_index.clone(), channel_seed);
    
    let local_funding_pubkey = PublicKey::from_secret_key(&secp, &local_channel_keys.funding_key);
    let local_htlc_basepoint = PublicKey::from_secret_key(&secp, &local_channel_keys.htlc_base_key);
    let remote_funding_pubkey = PublicKey::from_secret_key(&secp, &remote_channel_keys.funding_key);
    
    let remote_payment_basepoint = PublicKey::from_secret_key(&secp, &remote_channel_keys.payment_base_key);
    let remote_htlc_basepoint = PublicKey::from_secret_key(&secp, &remote_channel_keys.htlc_base_key);
    let remote_revocation_basepoint = PublicKey::from_secret_key(&secp, &remote_channel_keys.revocation_base_key);
    
    println!("✓ Channel keys derived for both parties");
    
    // Step 6: Create funding script
    let funding_script = create_funding_script(&local_funding_pubkey, &remote_funding_pubkey);
    
    println!("✓ Funding script created (2-of-2 multisig)");
    println!("  Script: {}", funding_script);
    
    // Step 7: Create funding transaction
    let funding_tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![
            TxIn {
                previous_output: OutPoint {
                    txid: bitcoin::Txid::from_slice(&[0u8; 32]).unwrap(),
                    vout: 0,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            }
        ],
        output: vec![
            TxOut {
                value: Amount::from_sat(channel_value_satoshis),
                script_pubkey: funding_script.to_p2wsh(),
            }
        ],
    };
    
    let funding_txid = funding_tx.compute_txid();
    println!("✓ Funding transaction created");
    println!("  TXID: {}", funding_txid);
    println!("  Amount: {} sats", channel_value_satoshis);
    
    // Step 8: Create funding outpoint
    let funding_outpoint = OutPoint {
        txid: funding_txid,
        vout: 0,
    };
    
    // Step 9: Set channel parameters
    let to_local_msat = 700_000_000;
    let to_remote_msat = 300_000_000;
    let commitment_number = 0;
    let to_self_delay = 144;
    let dust_limit_satoshis = 546;
    let feerate_per_kw = 5000;
    
    println!("\n=== Channel Parameters ===");
    println!("  Local balance: {} sats", to_local_msat / 1000);
    println!("  Remote balance: {} sats", to_remote_msat / 1000);
    println!("  Commitment number: {}", commitment_number);
    println!("  Self delay: {} blocks", to_self_delay);
    println!("  Fee rate: {} sat/kw", feerate_per_kw);
    
    // Step 10: Build commitment transaction
    // Uses Exercise 31 (production path) which derives keys internally
    let commitment_tx = build_commitment_from_channel_keys(
        funding_outpoint,
        &local_channel_keys,
        &remote_payment_basepoint,
        &remote_revocation_basepoint,
        &remote_htlc_basepoint,
        &local_htlc_basepoint,
        to_local_msat,
        to_remote_msat,
        vec![],  // No offered HTLCs
        vec![],  // No received HTLCs
        commitment_number,
        to_self_delay,
        dust_limit_satoshis,
        feerate_per_kw,
    );
    
    let commitment_txid = commitment_tx.compute_txid();
    
    println!("\n✓ First commitment transaction created");
    println!("  TXID: {}", commitment_txid);
    println!("  Inputs: {}", commitment_tx.input.len());
    println!("  Outputs: {}", commitment_tx.output.len());
    
    for (i, output) in commitment_tx.output.iter().enumerate() {
        println!("  Output {}: {} sats", i, output.value.to_sat());
    }
    
    // Step 11: Sign the commitment transaction
    let commitment_witness = create_commitment_witness(
        &commitment_tx,
        &funding_script,
        channel_value_satoshis,
        &local_channel_keys.funding_key,
        &remote_channel_keys.funding_key,
        &secp,
    );
    
    let mut signed_commitment_tx = commitment_tx.clone();
    signed_commitment_tx.input[0].witness = commitment_witness;
    
    println!("\n✓ Commitment transaction signed");
    println!("  Witness items: {}", signed_commitment_tx.input[0].witness.len());
    
    println!("\n=== Demo Complete ===");
    println!("\nThis demo shows the PRODUCTION PATH (Exercise 31):");
    println!("  1. Start with ChannelKeys containing base keys (Exercise 5)");
    println!("  2. Derive commitment-specific keys internally (Exercise 10)");
    println!("  3. Build transaction with derived keys (Exercises 28-30)");
    println!("\nFor TESTING with BOLT 3 vectors (Exercise 32):");
    println!("  Use CommitmentKeys.from_keys() to inject exact keys");
    println!("  See tests/commitment_tests.rs for examples.");
}
