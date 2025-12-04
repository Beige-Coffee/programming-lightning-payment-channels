use bitcoin::locktime::absolute::LockTime;
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use bitcoin::transaction::Version;
use bitcoin::{Amount, OutPoint, Sequence, Transaction, TxIn, TxOut, Witness};

use crate::keys::derive_revocation_public_key;
use crate::scripts::create_to_local_script;
use crate::transactions::fees::{calculate_htlc_success_tx_fee, calculate_htlc_timeout_tx_fee};
use crate::types::{CommitmentKeys, ChannelKeyManager};

// ============================================================================
// HTLC TRANSACTIONS
// ============================================================================

/// Exercise 22: Create HTLC-timeout transaction (unsigned)
/// 
/// This function creates an HTLC-timeout transaction structure that spends an HTLC output
/// on the commitment transaction after the CLTV timeout. The transaction is returned unsigned -
/// use the signing functions to add signatures and witness data.
pub fn create_htlc_timeout_transaction(
    htlc_outpoint: OutPoint,
    htlc_amount: u64,
    cltv_expiry: u32,
    local_keys: &CommitmentKeys,
    to_self_delay: u16,
    feerate_per_kw: u64,
) -> Transaction {
    let fee = calculate_htlc_timeout_tx_fee(feerate_per_kw);
    let output_amount = htlc_amount.saturating_sub(fee);

    let secp = Secp256k1::new();

    // Create to_local script
    let to_local_script = create_to_local_script(
        &local_keys.revocation_key,
        &local_keys.local_delayed_payment_key,
        to_self_delay,
    );

    let tx_in = TxIn {
        previous_output: htlc_outpoint,
        script_sig: ScriptBuf::new(),
        sequence: Sequence::ZERO,
        witness: Witness::new(),
    };

    let tx_out = TxOut {
        value: Amount::from_sat(output_amount),
        script_pubkey: to_local_script.to_p2wsh(),
    };

    Transaction {
    version: Version::TWO,
    lock_time: LockTime::from_consensus(cltv_expiry),
    input: vec![tx_in],
    output: vec![tx_out],
    }
}

/// Exercise 23: Finalize an HTLC-timeout transaction by signing it and attaching the witness
/// Returns the fully signed and finalized transaction ready for broadcast.
/// Witness stack: [0, remote_htlc_sig, local_htlc_sig, 0 (false), htlc_script]
pub fn finalize_htlc_timeout(
    keys_manager: ChannelKeyManager,
    tx: Transaction,
    input_index: usize,
    htlc_script: &ScriptBuf,
    htlc_amount: u64,
    remote_htlc_signature: Vec<u8>,
) -> Transaction {

    let local_htlc_privkey = keys_manager.htlc_basepoint_secret;

    let local_htlc_signature = keys_manager.sign_transaction_input(
        &tx,
        input_index,
        &htlc_script,
        htlc_amount,
        &local_htlc_privkey,
    );

    // Build witness stack
    let witness = Witness::from_slice(&[
        &[][..],                        // OP_0 for CHECKMULTISIG bug
        &remote_htlc_signature[..],
        &local_htlc_signature[..],
        &[][..],                        // OP_FALSE for timeout path
        htlc_script.as_bytes(),
    ]);

    let mut signed_tx = tx;
    signed_tx.input[0].witness = witness;

    signed_tx

}


/// Exercise 25: Create HTLC-success transaction (unsigned)
/// 
/// This function creates an HTLC-success transaction structure that spends an HTLC output
/// on the commitment transaction. The transaction is returned unsigned - use the signing
/// functions to add signatures and witness data.
pub fn create_htlc_success_transaction(
    htlc_outpoint: OutPoint,
    htlc_amount: u64,
    local_keys: &CommitmentKeys,
    to_self_delay: u16,
    feerate_per_kw: u64,
) -> Transaction {
    let fee = calculate_htlc_success_tx_fee(feerate_per_kw);
    let output_amount = htlc_amount.saturating_sub(fee);

    let secp = Secp256k1::new();

    // Create to_local script
    let to_local_script = create_to_local_script(
        &local_keys.revocation_key,
        &local_keys.local_delayed_payment_key,
        to_self_delay,
    );

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

/// Exercise 26: Finalize an HTLC-success transaction by signing it and attaching the witness
/// Returns the fully signed and finalized transaction ready for broadcast.
/// Witness stack: [0, remote_htlc_sig, local_htlc_sig, payment_preimage, htlc_script]
pub fn finalize_htlc_success(
    keys_manager: ChannelKeyManager,
    tx: Transaction,
    input_index: usize,
    htlc_script: &ScriptBuf,
    htlc_amount: u64,
    remote_htlc_signature: Vec<u8>,
    payment_preimage: [u8; 32],
) -> Transaction {

    let local_htlc_privkey = keys_manager.htlc_basepoint_secret;

    let local_htlc_signature = keys_manager.sign_transaction_input(
        &tx,
        input_index,
        &htlc_script,
        htlc_amount,
        &local_htlc_privkey,
    );

    let witness = Witness::from_slice(&[
        &[][..],                        // OP_0 for CHECKMULTISIG bug
        &remote_htlc_signature[..],
        &local_htlc_signature[..],
        &payment_preimage[..],
        htlc_script.as_bytes(),
    ]);

    let mut signed_tx = tx;
    signed_tx.input[0].witness = witness;

    signed_tx

}
