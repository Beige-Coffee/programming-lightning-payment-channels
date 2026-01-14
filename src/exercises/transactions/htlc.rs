use bitcoin::locktime::absolute::LockTime;
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use bitcoin::transaction::Version;
use bitcoin::{Amount, OutPoint, Sequence, Transaction, TxIn, TxOut, Witness};

use crate::keys::derive_revocation_public_key;
use crate::scripts::create_to_local_script;
use crate::transactions::fees::{calculate_htlc_success_tx_fee, calculate_htlc_timeout_tx_fee};
use crate::types::{CommitmentKeys, ChannelKeyManager};

/// Exercise 22: Create HTLC-timeout transaction
pub fn create_htlc_timeout_transaction(
    htlc_outpoint: OutPoint,
    htlc_amount: u64,
    cltv_expiry: u32,
    local_keys: &CommitmentKeys,
    to_self_delay: u16,
    feerate_per_kw: u64,
) -> Transaction {

    unimplemented!();

    // Calculate fee and output amount
    //let fee = calculate_htlc_timeout_tx_fee(feerate_per_kw);
    //let output_amount = htlc_amount.saturating_sub(fee);

    // Create a secp256k1 context

    // Create to_local script for the output

    // Build input spending from HTLC output

    // Create output to to_local script

    // Set locktime to CLTV expiry for timeout path

}

/// Exercise 23: Finalize HTLC-timeout transaction
pub fn finalize_htlc_timeout(
    keys_manager: ChannelKeyManager,
    tx: Transaction,
    input_index: usize,
    htlc_script: &ScriptBuf,
    htlc_amount: u64,
    remote_htlc_signature: Vec<u8>,
) -> Transaction {

    unimplemented!();

    // Get the local HTLC private key

    // Sign the transaction input with the local HTLC private key

    // Build witness: [0, remote_sig, local_sig, 0 (false for timeout), script]

    // Attach witness to transaction

    // Return Transaction

}


/// Exercise 25: Create HTLC-success transaction
pub fn create_htlc_success_transaction(
    htlc_outpoint: OutPoint,
    htlc_amount: u64,
    local_keys: &CommitmentKeys,
    to_self_delay: u16,
    feerate_per_kw: u64,
) -> Transaction {

    unimplemented!();

    // Calculate fee and output amount
    //let fee = calculate_htlc_success_tx_fee(feerate_per_kw);
    //let output_amount = htlc_amount.saturating_sub(fee);

    // Create a secp256k1 context

    // Create to_local script for the output

    // Build transaction with no locktime (immediate claim with preimage)

}

/// Exercise 26: Finalize HTLC-success transaction
pub fn finalize_htlc_success(
    keys_manager: ChannelKeyManager,
    tx: Transaction,
    input_index: usize,
    htlc_script: &ScriptBuf,
    htlc_amount: u64,
    remote_htlc_signature: Vec<u8>,
    payment_preimage: [u8; 32],
) -> Transaction {

    unimplemented!();

    // Get the local HTLC private key

    // Sign the transaction input with the local HTLC private key

    // Build witness: [0, remote_sig, local_sig, preimage, script]

    // Attach witness to transaction
    
    // Return Transaction

}
