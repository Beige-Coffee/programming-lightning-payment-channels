use bitcoin::locktime::absolute::LockTime;
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use bitcoin::transaction::Version;
use bitcoin::{Amount, OutPoint, Sequence, Transaction, TxIn, TxOut, Witness};

use crate::keys::derive_revocation_public_key;
use crate::scripts::create_to_local_script;
use crate::transactions::fees::{calculate_htlc_success_tx_fee, calculate_htlc_timeout_tx_fee};
use crate::types::{CommitmentKeys, ChannelKeyManager};

/// Exercise 23: Create HTLC-timeout transaction
pub fn create_htlc_timeout_transaction(
    htlc_outpoint: OutPoint,
    htlc_amount: u64,
    cltv_expiry: u32,
    local_keys: &CommitmentKeys,
    to_self_delay: u16,
    feerate_per_kw: u64,
) -> Transaction {
    // Calculate fee and output amount
    let fee = calculate_htlc_timeout_tx_fee(feerate_per_kw);
    let output_amount = htlc_amount.saturating_sub(fee);

    // Create a secp256k1 context
    let secp = Secp256k1::new();

    // Create to_local script for the output
    let to_local_script = create_to_local_script(
        &local_keys.revocation_key,
        &local_keys.local_delayed_payment_key,
        to_self_delay,
    );

    // Build input spending from HTLC output
    let tx_in = TxIn {
        previous_output: htlc_outpoint,
        script_sig: ScriptBuf::new(),
        sequence: Sequence::ZERO,
        witness: Witness::new(),
    };

    // Create output to to_local script
    let tx_out = TxOut {
        value: Amount::from_sat(output_amount),
        script_pubkey: to_local_script.to_p2wsh(),
    };

    // Set locktime to CLTV expiry for timeout path
    Transaction {
    version: Version::TWO,
    lock_time: LockTime::from_consensus(cltv_expiry),
    input: vec![tx_in],
    output: vec![tx_out],
    }
}

/// Exercise 24: Finalize HTLC-timeout transaction
pub fn finalize_htlc_timeout(
    keys_manager: ChannelKeyManager,
    tx: Transaction,
    input_index: usize,
    htlc_script: &ScriptBuf,
    htlc_amount: u64,
    remote_htlc_signature: Vec<u8>,
) -> Transaction {

    // Get the local HTLC private key
    let local_htlc_privkey = keys_manager.htlc_basepoint_secret;

    // Sign the transaction input with the local HTLC private key
    let local_htlc_signature = keys_manager.sign_transaction_input_sighash_all(
        &tx,
        input_index,
        &htlc_script,
        htlc_amount,
        &local_htlc_privkey,
    );

    // Build witness: [0, remote_sig, local_sig, 0 (false for timeout), script]
    let witness = Witness::from_slice(&[
        &[][..], 
        &remote_htlc_signature[..],
        &local_htlc_signature[..],
        &[][..],
        htlc_script.as_bytes(),
    ]);

    // Attach witness to transaction
    let mut signed_tx = tx;
    signed_tx.input[input_index].witness = witness;

    // Return Transaction
    signed_tx

}


/// Exercise 26: Create HTLC-success transaction
pub fn create_htlc_success_transaction(
    htlc_outpoint: OutPoint,
    htlc_amount: u64,
    local_keys: &CommitmentKeys,
    to_self_delay: u16,
    feerate_per_kw: u64,
) -> Transaction {
    // Calculate fee and output amount
    let fee = calculate_htlc_success_tx_fee(feerate_per_kw);
    let output_amount = htlc_amount.saturating_sub(fee);

    // Create a secp256k1 context
    let secp = Secp256k1::new();

    // Create to_local script for the output
    let to_local_script = create_to_local_script(
        &local_keys.revocation_key,
        &local_keys.local_delayed_payment_key,
        to_self_delay,
    );

    // Build transaction with no locktime (immediate claim with preimage)
    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: htlc_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ZERO,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(output_amount),
            script_pubkey: to_local_script.to_p2wsh(),
        }],
    }
}

/// Exercise 27: Finalize HTLC-success transaction
pub fn finalize_htlc_success(
    keys_manager: ChannelKeyManager,
    tx: Transaction,
    input_index: usize,
    htlc_script: &ScriptBuf,
    htlc_amount: u64,
    remote_htlc_signature: Vec<u8>,
    payment_preimage: [u8; 32],
) -> Transaction {

    // Get the local HTLC private key
    let local_htlc_privkey = keys_manager.htlc_basepoint_secret;

    // Sign the transaction input with the local HTLC private key
    let local_htlc_signature = keys_manager.sign_transaction_input_sighash_all(
        &tx,
        input_index,
        &htlc_script,
        htlc_amount,
        &local_htlc_privkey,
    );

    // Build witness: [0, remote_sig, local_sig, preimage, script]
    let witness = Witness::from_slice(&[
        &[][..],
        &remote_htlc_signature[..],
        &local_htlc_signature[..],
        &payment_preimage[..],
        htlc_script.as_bytes(),
    ]);

    // Attach witness to transaction
    let mut signed_tx = tx;
    signed_tx.input[input_index].witness = witness;

    // Return Transaction
    signed_tx

}
