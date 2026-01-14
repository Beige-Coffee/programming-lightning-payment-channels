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
use crate::types::{ChannelKeyManager, CommitmentKeys, OutputWithMetadata, HTLCOutput};

/// Exercise 16: Calculate obscure factor for commitment number
pub fn get_commitment_transaction_number_obscure_factor(
    initiator_payment_basepoint: &PublicKey,
    receiver_payment_basepoint: &PublicKey,
) -> u64 {
    // Create a SHA256 hash engine
    let mut sha = Sha256::engine();

    // Hash both payment basepoints together
    sha.input(&initiator_payment_basepoint.serialize());
    sha.input(&receiver_payment_basepoint.serialize());

    // Finalize the hash
    let res = Sha256::from_engine(sha).to_byte_array();

    // Extract lower 48 bits (last 6 bytes) of the hash
    ((res[26] as u64) << 5 * 8)
        | ((res[27] as u64) << 4 * 8)
        | ((res[28] as u64) << 3 * 8)
        | ((res[29] as u64) << 2 * 8)
        | ((res[30] as u64) << 1 * 8)
        | ((res[31] as u64) << 0 * 8)
}

/// Exercise 17: Set obscured commitment number in transaction
pub fn set_obscured_commitment_number(
    tx: &mut Transaction,
    commitment_number: u64,
    initiator_payment_basepoint: &PublicKey,
    receiver_payment_basepoint: &PublicKey,
) {
    // Get obscure factor from payment basepoints
    let commitment_transaction_number_obscure_factor =
        get_commitment_transaction_number_obscure_factor(
            &initiator_payment_basepoint,
            &receiver_payment_basepoint,
        );

    // XOR commitment number with obscure factor
    let obscured_commitment_transaction_number = commitment_transaction_number_obscure_factor ^ commitment_number;

    // Encode lower 24 bits in locktime
    let locktime_value =
        ((0x20 as u32) << 8 * 3) | ((obscured_commitment_transaction_number & 0xffffffu64) as u32);
    tx.lock_time = LockTime::from_consensus(locktime_value);

    // Encode upper 24 bits in sequence
    let sequence_value = Sequence(
        ((0x80 as u32) << 8 * 3) | ((obscured_commitment_transaction_number >> 3 * 8) as u32),
    );
    tx.input[0].sequence = sequence_value;
}

/// Exercise 18: Create commitment transaction outputs
pub fn create_commitment_transaction_outputs(
    to_local_value: u64,
    to_remote_value: u64,
    commitment_keys: &CommitmentKeys,
    remote_payment_basepoint: &PublicKey,
    to_self_delay: u16,
    dust_limit_satoshis: u64,
    fee: u64,
) -> Vec<OutputWithMetadata> {
    // Create a vector to store the outputs
    let mut outputs = Vec::new();

    // Create to_remote output if above dust limit
    if to_remote_value >= dust_limit_satoshis {
        let to_remote_script = create_to_remote_script(remote_payment_basepoint);
        outputs.push(OutputWithMetadata {
            value: to_remote_value,
            script: to_remote_script,
            cltv_expiry: None,
        });
    }

    // Create to_local output if above dust limit (subtract fee from our balance)
    if to_local_value >= dust_limit_satoshis {
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

    // Return the outputs
    outputs
}

/// Exercise 27: Create HTLC outputs
pub fn create_htlc_outputs(
    commitment_keys: &CommitmentKeys,
    offered_htlcs: &[HTLCOutput],
    received_htlcs: &[HTLCOutput],
) -> Vec<OutputWithMetadata> {
    // Create a vector to store the outputs
    let mut outputs = Vec::new();

    // Create outputs for HTLCs we offered
    for htlc in offered_htlcs {
        let script = create_offered_htlc_script(
            &commitment_keys.revocation_key,
            &commitment_keys.local_htlc_key,
            &commitment_keys.remote_htlc_key,
            &htlc.payment_hash,
        );
        outputs.push(OutputWithMetadata {
            value: htlc.amount_sat,
            script: script.to_p2wsh(),
            cltv_expiry: None,
        });
    }

    // Create outputs for HTLCs we received
    for htlc in received_htlcs {
        let script = create_received_htlc_script(
            &commitment_keys.revocation_key,
            &commitment_keys.local_htlc_key,
            &commitment_keys.remote_htlc_key,
            &htlc.payment_hash,
            htlc.cltv_expiry,
        );

        outputs.push(OutputWithMetadata {
            value: htlc.amount_sat,
            script: script.to_p2wsh(),
            cltv_expiry: Some(htlc.cltv_expiry),
        });
    }

    // Return the outputs
    outputs
}

/// Exercise 19: Sort outputs according to BOLT 3
pub fn sort_outputs(outputs: &mut Vec<OutputWithMetadata>) {
    // Sort by value, then script, then CLTV expiry (BIP69-style)
    outputs.sort_by(|a, b| {
        a.value
            .cmp(&b.value)
            .then(a.script.cmp(&b.script))
            .then(a.cltv_expiry.cmp(&b.cltv_expiry))
    });
}

/// Exercise 20: Create complete commitment transaction
pub fn create_commitment_transaction(
    funding_outpoint: OutPoint,
    to_local_value: u64,
    to_remote_value: u64,
    commitment_keys: &CommitmentKeys,
    local_payment_basepoint: &PublicKey,
    remote_payment_basepoint: &PublicKey,
    commitment_number: u64,
    to_self_delay: u16,
    dust_limit_satoshis: u64,
    feerate_per_kw: u64,
    offered_htlcs: &[HTLCOutput],
    received_htlcs: &[HTLCOutput],
) -> Transaction {
    // Calculate commitment transaction fee
    let num_htlcs = offered_htlcs.len() + received_htlcs.len();
    let fee = calculate_commitment_tx_fee(feerate_per_kw, num_htlcs);

    // Create a vector to store the output metadata
    let mut output_metadata = Vec::new();

    // Create to_local and to_remote outputs
    let channel_outputs = create_commitment_transaction_outputs(
        to_local_value,
        to_remote_value,
        commitment_keys,
        remote_payment_basepoint,
        to_self_delay,
        dust_limit_satoshis,
        fee,
    );

    // Create HTLC outputs
    let htlc_outputs = create_htlc_outputs(&commitment_keys, &offered_htlcs, &received_htlcs);

    // Combine all outputs
    output_metadata.extend(channel_outputs);
    output_metadata.extend(htlc_outputs);

    // Sort outputs per BOLT 3
    sort_outputs(&mut output_metadata);

    // Convert to TxOut format
    let outputs: Vec<TxOut> = output_metadata
        .iter()
        .map(|meta| TxOut {
            value: Amount::from_sat(meta.value),
            script_pubkey: meta.script.clone(),
        })
        .collect();

    // Build transaction spending from funding output
    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: funding_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: outputs,
    };

    // Set obscured commitment number in locktime and sequence
    set_obscured_commitment_number(
        &mut tx,
        commitment_number,
        local_payment_basepoint,
        remote_payment_basepoint,
    );

    tx
}

/// Exercise 20: Finalize holder commitment transaction
pub fn finalize_holder_commitment(
    keys_manager: ChannelKeyManager,
    tx: Transaction,
    input_index: usize,
    funding_script: &ScriptBuf,
    funding_amount: u64,
    remote_funding_signature: Vec<u8>,
    local_sig_first: bool,
) -> Transaction {

    // Get the local funding private key
    let local_funding_privkey = keys_manager.funding_key;

    // Sign the transaction input with the local funding private key
    let local_funding_signature = keys_manager.sign_transaction_input_sighash_all(
        &tx,
        input_index,
        &funding_script,
        funding_amount,
        &local_funding_privkey,
    );

    // Build witness stack with signatures in correct order (include OP_0 for CHECKMULTISIG bug)
    let witness =if local_sig_first {
        Witness::from_slice(&[
            &[][..],
            &local_funding_signature[..],
            &remote_funding_signature[..],
            funding_script.as_bytes(),
        ])
    } else {
        Witness::from_slice(&[
            &[][..],
            &remote_funding_signature[..],
            &local_funding_signature[..],
            funding_script.as_bytes(),
        ])

    };

    // Attach witness to transaction
    let mut signed_tx = tx;
    signed_tx.input[0].witness = witness;

    signed_tx

}