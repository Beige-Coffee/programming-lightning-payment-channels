use bitcoin::locktime::absolute::LockTime;
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use bitcoin::transaction::Version;
use bitcoin::{Amount, OutPoint, Sequence, Transaction, TxIn, TxOut, Witness};

use crate::keys::derive_revocation_public_key;
use crate::scripts::create_to_local_script;
use crate::transactions::fees::calculate_htlc_tx_fee;
use crate::types::CommitmentKeys;

// ============================================================================
// HTLC TRANSACTIONS
// ============================================================================

/// Exercise 28: Create HTLC-success transaction (unsigned)
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
    let fee = calculate_htlc_tx_fee(feerate_per_kw);
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

/// Exercise 29: Create HTLC-timeout transaction (unsigned)
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
    let fee = calculate_htlc_tx_fee(feerate_per_kw);
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
        lock_time: LockTime::from_consensus(cltv_expiry),
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

// ============================================================================
// HTLC WITNESS CONSTRUCTION
// ============================================================================

/// Create witness for HTLC-success transaction
/// 
/// This function constructs the witness for claiming an HTLC with the payment preimage.
/// It signs the transaction with the local HTLC key and combines it with the remote
/// signature to create the complete witness stack.
/// 
/// In a real Lightning implementation, you would:
/// 1. Create the unsigned HTLC transaction
/// 2. Send it to your counterparty to get their signature
/// 3. Sign it yourself with your local key
/// 4. Combine both signatures to create the witness
/// 
/// Witness stack: [0, remote_htlc_sig, local_htlc_sig, payment_preimage, htlc_script]
pub fn create_htlc_success_witness(
    remote_htlc_signature: Vec<u8>,
    local_htlc_signature: Vec<u8>,
    payment_preimage: [u8; 32],
    htlc_script: &ScriptBuf,
) -> Witness {

    // Build witness stack
    Witness::from_slice(&[
        &[][..],                        // OP_0 for CHECKMULTISIG bug
        &remote_htlc_signature[..],
        &local_htlc_signature[..],
        &payment_preimage[..],
        htlc_script.as_bytes(),
    ])
}

/// Create witness for HTLC-timeout transaction
/// 
/// This function constructs the witness for claiming an HTLC after it times out.
/// It signs the transaction with the local HTLC key and combines it with the remote
/// signature to create the complete witness stack.
/// 
/// In a real Lightning implementation, you would:
/// 1. Create the unsigned HTLC transaction
/// 2. Send it to your counterparty to get their signature
/// 3. Sign it yourself with your local key
/// 4. Combine both signatures to create the witness
/// 
/// Witness stack: [0, remote_htlc_sig, local_htlc_sig, 0 (false), htlc_script]
pub fn create_htlc_timeout_witness(
    remote_htlc_signature: Vec<u8>,
    local_htlc_signature: Vec<u8>,
    htlc_script: &ScriptBuf,
) -> Witness {

    // Build witness stack
    Witness::from_slice(&[
        &[][..],                        // OP_0 for CHECKMULTISIG bug
        &remote_htlc_signature[..],
        &local_htlc_signature[..],
        &[][..],                        // OP_FALSE for timeout path
        htlc_script.as_bytes(),
    ])
}
