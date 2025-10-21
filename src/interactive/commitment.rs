use bitcoin::{Transaction, TxIn, TxOut, OutPoint, Sequence, Witness, Amount};
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{Secp256k1, PublicKey, SecretKey};
use bitcoin::transaction::Version;
use bitcoin::locktime::absolute::LockTime;
use bitcoin::consensus::encode::serialize_hex;
use crate::internal::helper::{get_funding_input};
use crate::internal::bitcoind_client::{BitcoindClient, get_bitcoind_client};
use crate::scripts::funding::create_funding_script;
use crate::keys::derivation::new_keys_manager;
use crate::transactions::funding::create_simple_funding_transaction;
use std::time::Duration;
use tokio::time::sleep;
use bitcoin::Network;
use crate::types::{KeyFamily};

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

    // Get our keys
    let our_keys_manager = new_keys_manager(our_seed, bitcoin_network);
    let channel_keys = our_keys_manager.derive_channel_keys(channel_index);
    let first_commitment_point = channel_keys.derive_per_commitment_point(commitment_number=1);
    let commitment_keys = CommitmentKeys::from_channel_keys(
            first_commitment_point,
            channel_keys);
    

    // Get our Counterparty Pubkey
    let remote_keys_manager = new_keys_manager(remote_seed, bitcoin_network);
    let remote_channel_keys = remote_keys_manager.derive_channel_keys(channel_index);
    let remote_first_commitment_point = remote_channel_keys.derive_per_commitment_point(commitment_number=1);
    let remote_commitment_keys = CommitmentKeys::from_channel_keys(
        remote_first_commitment_point,
        remote_channel_keys);

    let remote_keys_manager = new_keys_manager(remote_seed, bitcoin_network);
    let remote_payment_privkey = remote_keys_manager.derive_key(KeyFamily::PaymentBase, channel_index);
    let remote_payment_pubkey = PublicKey::from_secret_key(&secp_ctx, &remote_payment_privkey);

    let txid_index = 0;
    let funding_outpoint = get_funding_input(txid.to_string(), txid_index).previous_output;

    let funding_amount = 5_000_000;
    let to_local_value = 3_998_500;
    let to_remote_value = 1_000_500;
    let to_self_delay = 144;
    let feerate_per_kw = 15000;
    let offered_htlcs: Vec<(u64, [u8; 32])> = Vec::new();
    let received_htlcs: Vec<(u64, [u8; 32], u32)> = Vec::new();

    let tx = create_commitment_transaction(
        funding_outpoint,
        to_local_value,
        to_remote_value,
        commitment_keys,
        remote_payment_pubkey,
        to_self_delay,
        feerate_per_kw,
        offered_htlcs,
        received_htlcs,
    );

    println!("\n✓ Commitment Transaction Created\n");
    println!("Tx ID: {}", tx.compute_txid());
    println!("\nTx Hex: {}", tx(&signed_tx));
    println!();
}
