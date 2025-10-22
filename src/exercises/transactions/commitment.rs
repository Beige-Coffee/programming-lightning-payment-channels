use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::HashEngine;
use bitcoin::hashes::{sha256, Hash};
use bitcoin::locktime::absolute::LockTime;
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::PublicKey;
use bitcoin::transaction::Version;
use bitcoin::{Amount, OutPoint, Sequence, Transaction, TxIn, TxOut, Witness};

use crate::scripts::{create_offered_htlc_script, create_received_htlc_script};
use crate::scripts::{create_to_local_script, create_to_remote_script};
use crate::transactions::fees::calculate_commitment_tx_fee;
use crate::types::{CommitmentKeys, OutputWithMetadata};
use crate::INITIAL_COMMITMENT_NUMBER;

// ============================================================================
// SECTION 7: COMMITMENT NUMBER OBSCURING
// ============================================================================
// These exercises teach how to obscure the commitment number in the transaction
// to provide privacy about the channel state.

/// Exercise 24: Calculate obscure factor for commitment number
pub fn get_commitment_transaction_number_obscure_factor(
    broadcaster_payment_basepoint: &PublicKey,
    countersignatory_payment_basepoint: &PublicKey,
    outbound_from_broadcaster: bool,
) -> u64 {
    let mut sha = Sha256::engine();

    if outbound_from_broadcaster {
        sha.input(&broadcaster_payment_basepoint.serialize());
        sha.input(&countersignatory_payment_basepoint.serialize());
    } else {
        sha.input(&countersignatory_payment_basepoint.serialize());
        sha.input(&broadcaster_payment_basepoint.serialize());
    }
    let res = Sha256::from_engine(sha).to_byte_array();

    ((res[26] as u64) << 5 * 8)
        | ((res[27] as u64) << 4 * 8)
        | ((res[28] as u64) << 3 * 8)
        | ((res[29] as u64) << 2 * 8)
        | ((res[30] as u64) << 1 * 8)
        | ((res[31] as u64) << 0 * 8)
}

/// Exercise 27: Set obscured commitment number in transaction
/// The commitment number is split across locktime (lower 24 bits) and
/// sequence (upper 24 bits) to prevent privacy leaks
pub fn set_obscured_commitment_number(
    tx: &mut Transaction,
    commitment_number: u64,
    local_payment_basepoint: &PublicKey,
    remote_payment_basepoint: &PublicKey,
    outbound_from_broadcaster: bool,
) {
    let commitment_transaction_number_obscure_factor =
        get_commitment_transaction_number_obscure_factor(
            &local_payment_basepoint,
            &remote_payment_basepoint,
            outbound_from_broadcaster,
        );

    let obscured_commitment_transaction_number = commitment_transaction_number_obscure_factor
        ^ (INITIAL_COMMITMENT_NUMBER - commitment_number);

    // Upper 24 bits in locktime
    let locktime_value =
        ((0x20 as u32) << 8 * 3) | ((obscured_commitment_transaction_number & 0xffffffu64) as u32);
    tx.lock_time = LockTime::from_consensus(locktime_value);

    // Lower 24 bits in sequence
    let sequence_value = Sequence(
        ((0x80 as u32) << 8 * 3) | ((obscured_commitment_transaction_number >> 3 * 8) as u32),
    );
    tx.input[0].sequence = sequence_value;
}

// ============================================================================
// SECTION 8: COMMITMENT TRANSACTION OUTPUT CREATION
// ============================================================================
// These exercises teach how to create the actual outputs for commitment transactions
// using pre-derived keys (from Exercise 10 or 13).

/// Exercise 25: Create commitment transaction outputs (using pre-derived keys)
///
/// This function accepts CommitmentKeys which contain all the derived keys
/// needed for this specific commitment transaction. This allows us to:
/// 1. Use keys derived from basepoints (production path - Exercise 10)
/// 2. Use exact keys from test vectors (testing path - from_keys method)
///
/// Creates to_local and to_remote outputs based on channel balances
///
/// Note: This does NOT sort outputs - sorting is handled by the transaction builder
fn create_commitment_transaction_outputs(
    to_local_value: u64,
    to_remote_value: u64,
    commitment_keys: &CommitmentKeys,
    remote_payment_basepoint: &PublicKey,
    to_self_delay: u16,
    fee: u64,
) -> Vec<OutputWithMetadata> {
    let mut outputs = Vec::new();

    // Create to_remote output (goes to counterparty, immediately spendable)
    if to_remote_value >= fee / 2 {
        let to_remote_script = create_to_remote_script(remote_payment_basepoint);
        outputs.push(OutputWithMetadata {
            value: to_remote_value,
            script: to_remote_script,
            cltv_expiry: None,
        });
    }

    // Create to_local output (goes to us, revocable with delay)
    if to_local_value >= fee / 2 {
        let to_local_script = create_to_local_script(
            &commitment_keys.revocation_key,
            &commitment_keys.local_delayed_payment_key,
            to_self_delay,
        );

        outputs.push(OutputWithMetadata {
            value: to_local_value - fee,
            script: to_local_script.to_p2wsh(),
            cltv_expiry: None,
        });
    }

    outputs
}

/// Exercise 26: Create HTLC outputs (using pre-derived keys)
/// Creates outputs for all offered and received HTLCs using the commitment keys
///
/// Note: This does NOT sort outputs - sorting is handled by the transaction builder
fn create_htlc_outputs(
    commitment_keys: &CommitmentKeys,
    offered_htlcs: &[(u64, [u8; 32])],
    received_htlcs: &[(u64, [u8; 32], u32)],
) -> Vec<OutputWithMetadata> {
    let mut outputs = Vec::new();

    // Create offered HTLC outputs (we offered, they can claim with preimage)
    for (amount, payment_hash) in offered_htlcs {
        let script = create_offered_htlc_script(
            &commitment_keys.revocation_key,
            &commitment_keys.local_htlc_key,
            &commitment_keys.remote_htlc_key,
            payment_hash,
        );
        outputs.push(OutputWithMetadata {
            value: *amount,
            script: script.to_p2wsh(),
            cltv_expiry: None,
        });
    }

    // Create received HTLC outputs (they offered, we can claim with preimage)
    for (amount, payment_hash, cltv_expiry) in received_htlcs {
        let script = create_received_htlc_script(
            &commitment_keys.revocation_key,
            &commitment_keys.local_htlc_key,
            &commitment_keys.remote_htlc_key,
            payment_hash,
            *cltv_expiry,
        );

        outputs.push(OutputWithMetadata {
            value: *amount,
            script: script.to_p2wsh(),
            cltv_expiry: Some(*cltv_expiry),
        });
    }

    outputs
}

/// Sort outputs according to BOLT 3 (BIP69-style):
/// First by value, then by script pubkey, then by CLTV expiry
pub fn sort_outputs(outputs: &mut Vec<OutputWithMetadata>) {
    outputs.sort_by(|a, b| {
        a.value
            .cmp(&b.value)
            .then(a.script.cmp(&b.script))
            .then(a.cltv_expiry.cmp(&b.cltv_expiry))
    });
}

/// Build all outputs and sort them once
///
/// Simple approach:
/// 1. Create all outputs (to_local, to_remote, all HTLCs)
/// 2. Sort everything once at the end
/// 3. Done!
fn build_and_sort_all_outputs(
    to_local_value: u64,
    to_remote_value: u64,
    commitment_keys: &CommitmentKeys,
    remote_payment_basepoint: &PublicKey,
    to_self_delay: u16,
    fee: u64,
    offered_htlcs: &[(u64, [u8; 32])],
    received_htlcs: &[(u64, [u8; 32], u32)],
) -> Vec<OutputWithMetadata> {
    let mut outputs = Vec::new();

    // Add to_local and to_remote outputs
    outputs.extend(create_commitment_transaction_outputs(
        to_local_value,
        to_remote_value,
        commitment_keys,
        remote_payment_basepoint,
        to_self_delay,
        fee,
    ));

    // Add all HTLC outputs
    outputs.extend(create_htlc_outputs(
        commitment_keys,
        offered_htlcs,
        received_htlcs,
    ));

    // Sort everything once
    sort_outputs(&mut outputs);

    outputs
}

// ============================================================================
// SECTION 9: COMMITMENT TRANSACTION CONSTRUCTION
// ============================================================================
// These exercises combine everything above to build complete commitment transactions.

/// Exercise 28: Create complete commitment transaction with HTLCs (using pre-derived keys)
///
/// Simple approach:
/// - Creates to_local and to_remote outputs
/// - Creates all HTLC outputs
/// - Sorts everything once
/// - Builds the complete transaction
pub fn create_commitment_transaction(
    funding_outpoint: OutPoint,
    to_local_value: u64,
    to_remote_value: u64,
    commitment_keys: &CommitmentKeys,
    remote_payment_basepoint: &PublicKey,
    to_self_delay: u16,
    feerate_per_kw: u64,
    offered_htlcs: Vec<(u64, [u8; 32])>,
    received_htlcs: Vec<(u64, [u8; 32], u32)>,
) -> Transaction {
    // Calculate fee based on number of HTLCs
    let num_htlcs = offered_htlcs.len() + received_htlcs.len();
    let fee = calculate_commitment_tx_fee(feerate_per_kw, num_htlcs);

    // Build and sort ALL outputs at once (HTLCs + to_local + to_remote)
    let all_outputs = build_and_sort_all_outputs(
        to_local_value,
        to_remote_value,
        commitment_keys,
        remote_payment_basepoint,
        to_self_delay,
        fee,
        &offered_htlcs,
        &received_htlcs,
    );

    // Convert to TxOut
    let outputs: Vec<TxOut> = all_outputs
        .iter()
        .map(|meta| TxOut {
            value: Amount::from_sat(meta.value),
            script_pubkey: meta.script.clone(),
        })
        .collect();

    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: funding_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: outputs,
    }
}

// ============================================================================
// WITNESS CONSTRUCTION
// ============================================================================

/// Exercise 31: Create witness for commitment transaction
/// 
/// In a real Lightning implementation:
/// 1. You create the unsigned commitment transaction
/// 2. You send it to your counterparty to get their signature
/// 3. You sign it with your local funding key (via the signer)
/// 4. You combine both signatures to create the witness (this function)
/// 
/// This function takes the signer, transaction, and remote signature to construct
/// the complete witness for the commitment transaction's funding input.
/// 
/// Witness stack: [0, sig1, sig2, witnessScript]
pub fn create_commitment_witness(
    tx: &Transaction,
    funding_script: &ScriptBuf,
    funding_amount: u64,
    local_funding_signature: Vec<u8>,
    remote_funding_signature: Vec<u8>,
) -> Witness {
    
    // Build witness stack: [0, sig1, sig2, witnessScript]
    Witness::from_slice(&[
        &[][..],                      // OP_0 for CHECKMULTISIG bug
        &local_funding_signature[..],
        &remote_funding_signature[..],
        funding_script.as_bytes(),
    ])
}
