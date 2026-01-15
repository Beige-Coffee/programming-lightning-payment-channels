use bitcoin::secp256k1::{PublicKey, Secp256k1};
use bitcoin::{OutPoint, Transaction};
use hex;

use crate::transactions::commitment::{
    create_commitment_transaction, set_obscured_commitment_number,
};
use crate::transactions::fees::is_htlc_dust;
use crate::types::{
    Bolt3Htlc, Bolt3TestVector, ChannelKeyManager, CommitmentKeys, HTLCOutput, HtlcDirection,
};
use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::{sha256, Hash};


pub fn build_complete_commitment_transaction(
    funding_outpoint: OutPoint,
    commitment_keys: &CommitmentKeys, 
    remote_payment_basepoint: &PublicKey,
    local_payment_basepoint: &PublicKey,
    to_local_value_msat: u64,
    to_remote_value_msat: u64,
    offered_htlcs: &[HTLCOutput],
    received_htlcs: &[HTLCOutput],
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
        .filter(|htlc| !is_htlc_dust(htlc.amount_sat, dust_limit_satoshis, feerate_per_kw, true))
        .cloned()
        .collect();

    let received_trimmed: Vec<_> = received_htlcs
        .iter()
        .filter(|htlc| !is_htlc_dust(htlc.amount_sat, dust_limit_satoshis, feerate_per_kw, false))
        .cloned()
        .collect();


    // create commitment transaction using exercise students completed
    let tx = create_commitment_transaction(
        funding_outpoint,
        to_local_value,
        to_remote_value,
        commitment_keys,
        local_payment_basepoint,
        remote_payment_basepoint,
        commitment_number,
        to_self_delay,
        dust_limit_satoshis,
        feerate_per_kw,
        &offered_trimmed,
        &received_trimmed,
    );

    tx
}

pub fn build_commitment_from_channel_keys(
    funding_outpoint: OutPoint,
    local_channel_keys: &ChannelKeyManager,
    remote_payment_basepoint: &PublicKey,
    remote_revocation_basepoint: &PublicKey,
    remote_htlc_basepoint: &PublicKey,
    local_htlc_basepoint: &PublicKey,
    to_local_value_msat: u64,
    to_remote_value_msat: u64,
    offered_htlcs: &[HTLCOutput],
    received_htlcs: &[HTLCOutput],
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
        &local_channel_keys.payment_basepoint_secret,
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


pub fn build_bolt3_simple_commitment(test_vector: &Bolt3TestVector) -> Transaction {
    let secp = Secp256k1::new();

    // Build ChannelKeyManager for key derivation
    let channel_keys = ChannelKeyManager {
        funding_key: test_vector.local_funding_privkey.clone(),
        revocation_basepoint_secret: test_vector.local_revocation_basepoint_secret.clone(),
        payment_basepoint_secret: test_vector.local_payment_basepoint_secret.clone(),
        delayed_payment_basepoint_secret: test_vector.local_delayed_payment_basepoint_secret.clone(),
        htlc_basepoint_secret: test_vector.local_htlc_basepoint_secret.clone(),
        commitment_seed: test_vector.commitment_seed,
        secp_ctx: secp.clone(),
    };

    let funding_outpoint = OutPoint {
        txid: bitcoin::Txid::from_slice(&test_vector.funding_txid).unwrap(),
        vout: test_vector.funding_output_index,
    };

    // Use exact keys from test vector (testing path)
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
        &vec![], // No offered HTLCs
        &vec![], // No received HTLCs
        test_vector.commitment_number,
        test_vector.local_delay,
        test_vector.local_dust_limit_satoshi,
        test_vector.feerate_per_kw,
    )
}

pub fn build_bolt3_commitment_with_htlcs(
    test_vector: &Bolt3TestVector,
    htlcs: Vec<Bolt3Htlc>,
) -> Transaction {
    let secp = Secp256k1::new();

    let channel_keys = ChannelKeyManager {
        funding_key: test_vector.local_funding_privkey.clone(),
        revocation_basepoint_secret: test_vector.local_revocation_basepoint_secret.clone(),
        payment_basepoint_secret: test_vector.local_payment_basepoint_secret.clone(),
        delayed_payment_basepoint_secret: test_vector.local_delayed_payment_basepoint_secret.clone(),
        htlc_basepoint_secret: test_vector.local_htlc_basepoint_secret.clone(),
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

    // Separate HTLCs by direction
    let mut offered_htlcs: Vec<HTLCOutput> = Vec::new();
    let mut received_htlcs: Vec<HTLCOutput> = Vec::new();

    for htlc in htlcs {
        match htlc.direction {
            HtlcDirection::Offered => {
                offered_htlcs.push(HTLCOutput {
                    amount_sat: htlc.amount_msat / 1000,
                    payment_hash: htlc.payment_hash,
                    cltv_expiry: htlc.cltv_expiry,
                });
            }
            HtlcDirection::Received => {
                received_htlcs.push(HTLCOutput {
                    amount_sat: htlc.amount_msat / 1000,
                    payment_hash: htlc.payment_hash,
                    cltv_expiry: htlc.cltv_expiry,
                });
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
        &offered_htlcs,
        &received_htlcs,
        test_vector.commitment_number,
        test_vector.local_delay,
        test_vector.local_dust_limit_satoshi,
        test_vector.feerate_per_kw,
    )
}

pub fn verify_bolt3_txid(tx: &Transaction, expected_txid: &str) -> bool {
    let actual_txid = tx.compute_txid().to_string();
    actual_txid == expected_txid
}
