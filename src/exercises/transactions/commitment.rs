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

    unimplemented!();

    // Create a SHA256 hash engine

    // Hash both payment basepoints together

    // Finalize the hash

    // Extract lower 48 bits (last 6 bytes) of the hash
}

/// Exercise 17: Set obscured commitment number in transaction
pub fn set_obscured_commitment_number(
    tx: &mut Transaction,
    commitment_number: u64,
    initiator_payment_basepoint: &PublicKey,
    receiver_payment_basepoint: &PublicKey,
) {
    
    unimplemented!();
    
    // Get obscure factor from payment basepoints

    // XOR commitment number with obscure factor

    // Encode lower 24 bits in locktime

    // Encode upper 24 bits in sequence
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

    unimplemented!();

    // Create a vector to store the outputs

    // Create to_remote output if above dust limit

    // Create to_local output if above dust limit (subtract fee from our balance)

    // Return the outputs

}

/// Exercise 27: Create HTLC outputs
pub fn create_htlc_outputs(
    commitment_keys: &CommitmentKeys,
    offered_htlcs: &[HTLCOutput],
    received_htlcs: &[HTLCOutput],
) -> Vec<OutputWithMetadata> {

    unimplemented!();

    // Create a vector to store the outputs

    // Create outputs for HTLCs we offered

    // Create outputs for HTLCs we received

    // Return the outputs

}

/// Exercise 19: Sort outputs according to BOLT 3
pub fn sort_outputs(outputs: &mut Vec<OutputWithMetadata>) {
    
    unimplemented!();

    // Sort by value, then script, then CLTV expiry (BIP69-style)

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

    unimplemented!();

    // Calculate commitment transaction fee
    //let num_htlcs = offered_htlcs.len() + received_htlcs.len();
    //let fee = calculate_commitment_tx_fee(feerate_per_kw, num_htlcs);

    // Create a vector to store the output metadata

    // Create to_local and to_remote outputs

    // Create HTLC outputs

    // Combine all outputs

    // Sort outputs per BOLT 3

    // Convert to TxOut format

    // Build transaction spending from funding output

    // Set obscured commitment number in locktime and sequence

    // Return transaction
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

    unimplemented!();

    // Get the local funding private key

    // Sign the transaction input with the local funding private key

    // Build witness stack with signatures in correct order (include OP_0 for CHECKMULTISIG bug)

    // Attach witness to transaction

    // Return Transaction

}