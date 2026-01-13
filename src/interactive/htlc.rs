use crate::internal::bitcoind_client::{get_bitcoind_client, BitcoindClient};
use crate::internal::helper::get_outpoint;
use crate::keys::derivation::new_keys_manager;
use crate::scripts::funding::create_funding_script;
use crate::transactions::commitment::{finalize_holder_commitment};
use crate::transactions::commitment::create_commitment_transaction;
use crate::types::{CommitmentKeys, ChannelKeyManager, KeyFamily, HTLCOutput};
use bitcoin::consensus::encode::serialize_hex;
use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::{sha256, Hash};
use bitcoin::locktime::absolute::LockTime;
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use bitcoin::transaction::Version;
use bitcoin::Network;
use bitcoin::{Amount, OutPoint, Sequence, Transaction, TxIn, TxOut, Witness};
use std::time::Duration;
use tokio::time::sleep;
use bitcoin::PublicKey as BitcoinPublicKey;

pub async fn run(funding_txid: String) {
    // Parse the argument as txid
    let txid = funding_txid;

    // get bitcoin client
    let bitcoind = get_bitcoind_client().await;

    let our_seed = [0x01; 32];
    let remote_seed = [0x02; 32];
    let bitcoin_network = Network::Bitcoin;
    let channel_index = 0;
    let secp_ctx = Secp256k1::new();
    let commitment_number = 2;
    let dust_limit_sats = 355;

    // Get our keys
    let our_node_keys_manager = new_keys_manager(our_seed, bitcoin_network);
    let our_channel_keys_manager = our_node_keys_manager.derive_channel_keys(channel_index);
    let our_channel_public_keys = our_channel_keys_manager.to_public_keys();
    let local_funding_privkey = our_channel_keys_manager.funding_key;
    let local_funding_pubkey = BitcoinPublicKey::new(our_channel_public_keys.funding_pubkey);
    let local_payment_basepoint = our_channel_public_keys.payment_basepoint;
    let second_commitment_point = our_channel_keys_manager.derive_per_commitment_point(commitment_number);

    // Get our Counterparty keys
    let remote_node_keys_manager = new_keys_manager(remote_seed, bitcoin_network);
    let remote_channel_keys_manager = remote_node_keys_manager.derive_channel_keys(channel_index);
    let remote_channel_public_keys = remote_channel_keys_manager.to_public_keys();
    let remote_payment_basepoint = remote_channel_public_keys.payment_basepoint;
    let remote_funding_privkey = remote_channel_keys_manager.funding_key;
    let remote_funding_pubkey = BitcoinPublicKey::new(remote_channel_public_keys.funding_pubkey);

    // Get our commitment keys
    // we need the remote basepoints for revocation and htlc,
    //     so we create this after creating their keys
    let commitment_keys = CommitmentKeys::from_basepoints(
        &second_commitment_point,
        &our_channel_public_keys.delayed_payment_basepoint,
        &our_channel_public_keys.htlc_basepoint,
        &remote_channel_public_keys.revocation_basepoint,
        &remote_channel_public_keys.htlc_basepoint,
        &secp_ctx,
    );

    let txid_index = 0;
    let funding_outpoint = get_outpoint(txid.to_string(), txid_index);

    let funding_amount = 5_000_000;
    let to_local_value = 4_594_500;
    let to_remote_value = 500;
    let to_self_delay = 144;
    let feerate_per_kw = 1117;
    let payment_hash = Sha256::hash(&[0u8; 32]).to_byte_array();
    let mut offered_htlcs: Vec<HTLCOutput> = Vec::new();
    offered_htlcs.push(HTLCOutput {
        amount_sat: 405_000,
        payment_hash: payment_hash,
        cltv_expiry: 200,
    });

    let received_htlcs: Vec<HTLCOutput> = Vec::new();

    // Step 1: Create the unsigned commitment transaction
    let tx = create_commitment_transaction(
        funding_outpoint,
        to_local_value,
        to_remote_value,
        &commitment_keys, // Pre-derived keys!
        &local_payment_basepoint,
        &remote_payment_basepoint,
        commitment_number,
        to_self_delay,
        dust_limit_sats,
        feerate_per_kw,
        &offered_htlcs,  // HTLCs included from the start
        &received_htlcs, // HTLCs included from the start
    );

    let funding_script = create_funding_script(&local_funding_pubkey, &remote_funding_pubkey);

    // Step 2: In real Lightning, we would send this transaction to our counterparty
    // and they would send us back their signature. Here we simulate that by
    // creating their signature ourselves (but in reality we wouldn't have their key!)
    let remote_funding_signature = remote_channel_keys_manager.sign_transaction_input_sighash_all(
        &tx,
        0,
        &funding_script,
        funding_amount,
        &remote_funding_privkey,
    );

    let local_sig_first = true;

    let signed_tx = finalize_holder_commitment(
        our_channel_keys_manager,
        tx,
        0,
        &funding_script,
        funding_amount,
        remote_funding_signature,
        local_sig_first);

    println!("\n✅ Commitment Transaction Created\n");
    println!("Tx ID: {}", signed_tx.compute_txid());
    println!("\nTx Hex: {}", serialize_hex(&signed_tx));
    println!();
}
