use bitcoin::{Transaction, TxIn, TxOut, OutPoint, Sequence, Witness, Amount};
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{Secp256k1, PublicKey};
use bitcoin::transaction::Version;
use bitcoin::locktime::absolute::LockTime;

use crate::keys::commitment::CommitmentKeys;
use crate::scripts::create_to_local_script;
use crate::keys::derive_revocation_public_key;
use crate::transactions::fees::calculate_htlc_tx_fee;

// ============================================================================
// HTLC TRANSACTIONS
// ============================================================================

/// Exercise 28: Create HTLC-success transaction
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
        to_self_delay
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

/// Exercise 29: Create HTLC-timeout transaction
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
        to_self_delay
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