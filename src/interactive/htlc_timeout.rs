use crate::internal::bitcoind_client::{get_bitcoind_client, BitcoindClient};
use crate::internal::helper::get_outpoint;
use crate::keys::derivation::new_keys_manager;
use crate::scripts::funding::create_funding_script;
use crate::scripts::htlc::create_offered_htlc_script;
use crate::keys::commitment::{derive_private_key};
use crate::transactions::htlc::{create_htlc_timeout_transaction, finalize_htlc_timeout};
use crate::types::{CommitmentKeys,ChannelKeyManager, KeyFamily};
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

pub async fn run(commitment_txid: String) {
    // Parse the argument as txid
    let txid = commitment_txid;

    // get bitcoin client
    let bitcoind = get_bitcoind_client();

    let our_seed = [0x01; 32];
    let remote_seed = [0x02; 32];
    let bitcoin_network = Network::Bitcoin;
    let channel_index = 0;
    let secp_ctx = Secp256k1::new();
    let commitment_number = 2;

    // Get our keys
    let our_node_keys_manager = new_keys_manager(our_seed, bitcoin_network);
    let our_channel_keys_manager = our_node_keys_manager.derive_channel_keys(channel_index);
    let our_channel_public_keys = our_channel_keys_manager.to_public_keys();
    let local_funding_privkey = our_channel_keys_manager.funding_key;
    let local_funding_pubkey = our_channel_public_keys.funding_pubkey;
    let second_commitment_point = our_channel_keys_manager.derive_per_commitment_point(commitment_number);
    
    // Derive local HTLC secret key (for signing)
    let local_htlc_basepoint_secret = our_channel_keys_manager.htlc_basepoint_secret;
    let local_htlc_secret = derive_private_key(
                                &local_htlc_basepoint_secret,
                                &second_commitment_point,
                                &secp_ctx,
                                );

    // Get our Counterparty keys
    let remote_node_keys_manager = new_keys_manager(remote_seed, bitcoin_network);
    let remote_channel_keys_manager = remote_node_keys_manager.derive_channel_keys(channel_index);
    let remote_channel_public_keys = remote_channel_keys_manager.to_public_keys();
    let remote_payment_pubkey = remote_channel_public_keys.payment_basepoint;
    let remote_funding_privkey = remote_channel_keys_manager.funding_key;
    let remote_funding_pubkey = remote_channel_public_keys.funding_pubkey;
    
    let remote_htlc_basepoint_secret = remote_channel_keys_manager.htlc_basepoint_secret;
    let remote_htlc_secret = derive_private_key(
                                &remote_htlc_basepoint_secret,
                                &second_commitment_point,
                                &secp_ctx,
                                );

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

    let txid_index = 1;
    let htlc_outpoint = get_outpoint(txid.to_string(), txid_index);

    let htlc_amount = 404_000;
    let cltv_expiry = 200;
    let to_self_delay = 144;
    let feerate_per_kw = 1117;
    let payment_hash = Sha256::hash(&[0u8; 32]).to_byte_array();

    // Create the HTLC script that we're spending from
    let htlc_script = create_offered_htlc_script(
        &commitment_keys.revocation_key,
        &commitment_keys.local_htlc_key,
        &commitment_keys.remote_htlc_key,
        &payment_hash,
    );

    // Step 1: Create the unsigned HTLC timeout transaction
    let tx = create_htlc_timeout_transaction(
        htlc_outpoint,
        htlc_amount,
        cltv_expiry,
        &commitment_keys,
        to_self_delay,
        feerate_per_kw,
    );

    // The input_index is the index of the input in the transaction being signed
    // Since the HTLC timeout transaction has only one input, it's always 0
    let input_index = 0;

    // Step 2: In real Lightning, we would send this transaction to our counterparty
    // and they would send us back their signature. Here we simulate that by
    // creating their signature ourselves (but in reality we wouldn't have their key!)
    let remote_htlc_signature = remote_channel_keys_manager.sign_transaction_input_sighash_all(
        &tx,
        input_index,
        &htlc_script,
        htlc_amount,
        &remote_htlc_secret,
    );

    let signed_tx = finalize_htlc_timeout(
        our_channel_keys_manager,
        tx,
        input_index,
        &htlc_script,
        htlc_amount,
        remote_htlc_signature);


    println!("\n✅ HTLC Timeout Transaction Created\n");
    println!("Tx ID: {}", signed_tx.compute_txid());
    println!("\nTx Hex: {}", serialize_hex(&signed_tx));
    println!();
}
