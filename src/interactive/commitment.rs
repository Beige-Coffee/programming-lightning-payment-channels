use crate::internal::bitcoind_client::{get_bitcoind_client, BitcoindClient};
use crate::internal::helper::get_funding_input;
use crate::keys::derivation::new_keys_manager;
use crate::scripts::funding::create_funding_script;
use crate::signing::create_commitment_witness;
use crate::transactions::commitment::create_commitment_transaction;
use crate::types::{CommitmentKeys, KeyFamily};
use bitcoin::consensus::encode::serialize_hex;
use bitcoin::locktime::absolute::LockTime;
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use bitcoin::transaction::Version;
use bitcoin::Network;
use bitcoin::{Amount, OutPoint, Sequence, Transaction, TxIn, TxOut, Witness};
use std::time::Duration;
use tokio::time::sleep;

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
    let commitment_number = 1;

    // Get our keys
    let our_keys_manager = new_keys_manager(our_seed, bitcoin_network);
    let our_channel_keys = our_keys_manager.derive_channel_keys(channel_index);
    let local_funding_privkey = our_keys_manager.derive_key(KeyFamily::MultiSig, channel_index);
    let local_funding_pubkey = PublicKey::from_secret_key(&secp_ctx, &local_funding_privkey);
    let first_commitment_point = our_channel_keys.derive_per_commitment_point(commitment_number);

    // Get our Counterparty Pubkey
    let remote_keys_manager = new_keys_manager(remote_seed, bitcoin_network);
    let remote_channel_keys = our_keys_manager.derive_channel_keys(channel_index);
    let remote_payment_privkey =
        remote_keys_manager.derive_key(KeyFamily::PaymentBase, channel_index);
    let remote_payment_pubkey = PublicKey::from_secret_key(&secp_ctx, &remote_payment_privkey);
    let remote_funding_privkey = remote_keys_manager.derive_key(KeyFamily::MultiSig, channel_index);
    let remote_funding_pubkey = PublicKey::from_secret_key(&secp_ctx, &remote_funding_privkey);

    // Get our keys
    // we need the remote basepoints for revocation and htlc,
    //     so we create this after creating their keys
    let commitment_keys = CommitmentKeys::from_basepoints(
        &first_commitment_point,
        our_channel_keys.delayed_payment_base_key,
        our_channel_keys.htlc_base_key,
        remote_channel_keys.revocation_base_key,
        remote_channel_keys.htlc_base_key,
        &secp_ctx,
    );

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

    let funding_script = create_funding_script(local_funding_pubkey, remote_funding_pubkey);

    let signed_tx = create_commitment_witness(
        tx,
        funding_script,
        funding_amount,
        local_funding_privkey,
        remote_funding_privkey,
        secp_ctx,
    );

    println!("\n✓ Commitment Transaction Created\n");
    println!("Tx ID: {}", tx.compute_txid());
    println!("\nTx Hex: {}", tx(&signed_tx));
    println!();
}
