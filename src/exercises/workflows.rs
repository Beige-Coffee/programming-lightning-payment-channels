use bitcoin::secp256k1::{PublicKey, Secp256k1};
use bitcoin::{OutPoint, Transaction};
use hex;

use crate::transactions::commitment::{
    create_commitment_transaction, set_obscured_commitment_number,
};
use crate::transactions::fees::is_htlc_dust;
use crate::types::{Bolt3Htlc, Bolt3TestVector, InMemorySigner, ChannelKeys, CommitmentKeys, HtlcDirection};
use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::{sha256, Hash};

// ============================================================================
// SECTION 10: HIGH-LEVEL WORKFLOW FUNCTIONS
// ============================================================================
// These exercises combine all previous concepts into complete workflows
// for building commitment transactions in production and testing scenarios.

/// Exercise 30 (Updated): Build complete commitment transaction with all features
///
/// This follows the LDK-style pattern:
/// 1. Accept pre-derived CommitmentKeys (from Exercise 10 or 13)
/// 2. Trim dust HTLCs (Exercise 22-23)
/// 3. Build transaction with ALL outputs at once (Exercise 28 - updated)
/// 4. Set obscured commitment number (Exercise 27)
///
/// This is the main function that combines all previous exercises.
pub fn build_complete_commitment_transaction(
    funding_outpoint: OutPoint,
    commitment_keys: &CommitmentKeys, // Accept pre-derived keys!
    remote_payment_basepoint: &PublicKey,
    local_payment_basepoint: &PublicKey,
    to_local_value_msat: u64,
    to_remote_value_msat: u64,
    offered_htlcs: Vec<(u64, [u8; 32])>,
    received_htlcs: Vec<(u64, [u8; 32], u32)>,
    commitment_number: u64,
    to_self_delay: u16,
    dust_limit_satoshis: u64,
    feerate_per_kw: u64,
) -> Transaction {
    // Convert msat to sat
    let to_local_value = to_local_value_msat / 1000;
    let to_remote_value = to_remote_value_msat / 1000;

    // Trim dust HTLCs
    let offered_trimmed: Vec<_> = offered_htlcs
        .iter()
        .filter(|(amt, _)| !is_htlc_dust(*amt, dust_limit_satoshis, feerate_per_kw))
        .cloned()
        .collect();

    let received_trimmed: Vec<_> = received_htlcs
        .iter()
        .filter(|(amt, _, _)| !is_htlc_dust(*amt, dust_limit_satoshis, feerate_per_kw))
        .cloned()
        .collect();

    // Create complete commitment tx with ALL outputs at once (LDK-style)
    // This is more efficient than creating the base tx and then adding HTLCs
    let mut tx = create_commitment_transaction(
        funding_outpoint,
        to_local_value,
        to_remote_value,
        commitment_keys, // Pre-derived keys!
        remote_payment_basepoint,
        to_self_delay,
        feerate_per_kw,
        offered_trimmed,  // HTLCs included from the start
        received_trimmed, // HTLCs included from the start
    );

    // Set obscured commitment number
    set_obscured_commitment_number(
        &mut tx,
        commitment_number,
        local_payment_basepoint,
        remote_payment_basepoint,
        true,
    );

    tx
}

/// Exercise 31: Build commitment transaction from ChannelKeys (deriving keys)
///
/// PRODUCTION PATH: This is the typical production workflow.
///
/// Flow:
/// 1. Start with ChannelKeys containing base keys (from Exercise 5)
/// 2. Derive commitment-specific keys (Exercise 13)
/// 3. Build transaction with those keys (Exercise 30)
pub fn build_commitment_from_channel_keys(
    funding_outpoint: OutPoint,
    local_channel_keys: &ChannelKeys,
    remote_payment_basepoint: &PublicKey,
    remote_revocation_basepoint: &PublicKey,
    remote_htlc_basepoint: &PublicKey,
    local_htlc_basepoint: &PublicKey,
    to_local_value_msat: u64,
    to_remote_value_msat: u64,
    offered_htlcs: Vec<(u64, [u8; 32])>,
    received_htlcs: Vec<(u64, [u8; 32], u32)>,
    commitment_number: u64,
    to_self_delay: u16,
    dust_limit_satoshis: u64,
    feerate_per_kw: u64,
) -> Transaction {
    // STEP 1: Derive all commitment keys from basepoints
    let commitment_keys = local_channel_keys.get_commitment_keys(
        commitment_number,
        remote_revocation_basepoint,
        remote_htlc_basepoint,
        local_htlc_basepoint,
    );

    // STEP 2: Build transaction with derived keys
    let local_payment_basepoint = PublicKey::from_secret_key(
        &local_channel_keys.secp_ctx,
        &local_channel_keys.payment_base_key,
    );

    build_complete_commitment_transaction(
        funding_outpoint,
        &commitment_keys,
        remote_payment_basepoint,
        &local_payment_basepoint,
        to_local_value_msat,
        to_remote_value_msat,
        offered_htlcs,
        received_htlcs,
        commitment_number,
        to_self_delay,
        dust_limit_satoshis,
        feerate_per_kw,
    )
}

// ============================================================================
// SECTION 11: BOLT 3 TEST VECTOR WORKFLOWS
// ============================================================================
// These exercises show how to use BOLT 3 test vectors to verify your
// implementation matches the Lightning specification.

/// Exercise 32: Build simple commitment transaction from BOLT 3 test vector
///
/// TESTING PATH: Use exact keys from BOLT 3 specification.
///
/// This demonstrates using CommitmentKeys.from_keys() to inject exact keys
/// from test vectors instead of deriving them, allowing you to verify that
/// your transaction construction produces the exact same output as the spec.
pub fn build_bolt3_simple_commitment(test_vector: &Bolt3TestVector) -> Transaction {
    let secp = Secp256k1::new();

    // Build ChannelKeys for key derivation
    let channel_keys = ChannelKeys {
        funding_key: test_vector.local_funding_privkey.clone(),
        revocation_base_key: test_vector.local_revocation_basepoint_secret.clone(),
        payment_base_key: test_vector.local_payment_basepoint_secret.clone(),
        delayed_payment_base_key: test_vector.local_delayed_payment_basepoint_secret.clone(),
        htlc_base_key: test_vector.local_htlc_basepoint_secret.clone(),
        commitment_seed: test_vector.commitment_seed,
        secp_ctx: secp.clone(),
    };

    let funding_outpoint = OutPoint {
        txid: bitcoin::Txid::from_slice(&test_vector.funding_txid).unwrap(),
        vout: test_vector.funding_output_index,
    };

    // OPTION 2: Use exact keys from test vector (testing path)
    // For BOLT 3 test vectors, we use exact keys they provide
    let per_commitment_point =
        channel_keys.derive_per_commitment_point(test_vector.commitment_number);

    let commitment_keys = CommitmentKeys::from_keys(
        per_commitment_point,
        PublicKey::from_slice(
            &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19")
                .unwrap(),
        )
        .unwrap(),
        PublicKey::from_slice(
            &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c")
                .unwrap(),
        )
        .unwrap(),
        PublicKey::from_slice(
            &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7")
                .unwrap(),
        )
        .unwrap(),
        PublicKey::from_slice(
            &hex::decode("0394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b")
                .unwrap(),
        )
        .unwrap(),
    );

    build_complete_commitment_transaction(
        funding_outpoint,
        &commitment_keys,
        &test_vector.remote_payment_basepoint,
        &test_vector.local_payment_basepoint,
        test_vector.to_local_msat,
        test_vector.to_remote_msat,
        vec![], // No offered HTLCs
        vec![], // No received HTLCs
        test_vector.commitment_number,
        test_vector.local_delay,
        test_vector.local_dust_limit_satoshi,
        test_vector.feerate_per_kw,
    )
}

/// Exercise 33: Build commitment transaction with HTLCs from BOLT 3 test vector
/// Similar to Exercise 32 but includes HTLC outputs for more complex testing
pub fn build_bolt3_commitment_with_htlcs(
    test_vector: &Bolt3TestVector,
    htlcs: Vec<Bolt3Htlc>,
) -> Transaction {
    let secp = Secp256k1::new();

    let channel_keys = ChannelKeys {
        funding_key: test_vector.local_funding_privkey.clone(),
        revocation_base_key: test_vector.local_revocation_basepoint_secret.clone(),
        payment_base_key: test_vector.local_payment_basepoint_secret.clone(),
        delayed_payment_base_key: test_vector.local_delayed_payment_basepoint_secret.clone(),
        htlc_base_key: test_vector.local_htlc_basepoint_secret.clone(),
        commitment_seed: test_vector.commitment_seed,
        secp_ctx: secp,
    };

    let funding_outpoint = OutPoint {
        txid: bitcoin::Txid::from_slice(&test_vector.funding_txid).unwrap(),
        vout: test_vector.funding_output_index,
    };

    // Derive commitment keys
    let commitment_keys = CommitmentKeys::from_keys(
        channel_keys.derive_per_commitment_point(test_vector.commitment_number),
        test_vector.local_revocation_pubkey,
        test_vector.local_delayedpubkey,
        test_vector.local_htlcpubkey,
        test_vector.remote_htlcpubkey,
    );

    // DEBUG: Print what keys we're actually using
    println!("\n=== HTLC Keys Debug ===");
    println!(
        "Expected local_htlcpubkey:  {}",
        hex::encode(test_vector.local_htlcpubkey.serialize())
    );
    println!(
        "Actual local_htlc_key:      {}",
        hex::encode(commitment_keys.local_htlc_key.serialize())
    );
    println!(
        "Expected remote_htlcpubkey: {}",
        hex::encode(test_vector.remote_htlcpubkey.serialize())
    );
    println!(
        "Actual remote_htlc_key:     {}",
        hex::encode(commitment_keys.remote_htlc_key.serialize())
    );

    // Separate HTLCs by direction
    let mut offered = Vec::new();
    let mut received = Vec::new();

    for htlc in htlcs {
        match htlc.direction {
            HtlcDirection::Offered => {
                offered.push((htlc.amount_msat / 1000, htlc.payment_hash));
            }
            HtlcDirection::Received => {
                received.push((htlc.amount_msat / 1000, htlc.payment_hash, htlc.cltv_expiry));
            }
        }
    }

    build_complete_commitment_transaction(
        funding_outpoint,
        &commitment_keys,
        &test_vector.remote_payment_basepoint,
        &test_vector.local_payment_basepoint,
        test_vector.to_local_msat,
        test_vector.to_remote_msat,
        offered,
        received,
        test_vector.commitment_number,
        test_vector.local_delay,
        test_vector.local_dust_limit_satoshi,
        test_vector.feerate_per_kw,
    )
}

/// Exercise 34: Verify a transaction matches expected TXID
/// Helper function to check if your built transaction matches the expected TXID
pub fn verify_bolt3_txid(tx: &Transaction, expected_txid: &str) -> bool {
    let actual_txid = tx.compute_txid().to_string();
    actual_txid == expected_txid
}
