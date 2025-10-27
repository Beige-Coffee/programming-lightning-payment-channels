use bitcoin::{Transaction, TxIn, TxOut, OutPoint, Sequence, Witness, Amount};
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{Secp256k1, PublicKey, SecretKey};
use bitcoin::transaction::Version;
use bitcoin::locktime::absolute::LockTime;
use bitcoin::consensus::encode::serialize_hex;
use crate::internal::helper::{get_unspent_output, sign_raw_transaction};
use crate::internal::bitcoind_client::{BitcoindClient, get_bitcoind_client};
use crate::scripts::funding::create_funding_script;
use crate::keys::derivation::new_keys_manager;
use crate::transactions::funding::create_funding_transaction;
use std::time::Duration;
use tokio::time::sleep;
use bitcoin::Network;
use crate::types::{KeyFamily};
use bitcoin::PublicKey as BitcoinPublicKey;

pub async fn build_funding_tx(
    bitcoind: BitcoindClient,
    tx_input: TxIn,
    funding_amount_sat: u64,
) { 
    let our_seed = [0x01; 32];
    let remote_seed = [0x02; 32];
    let bitcoin_network = Network::Bitcoin;
    let channel_index = 0;
    let secp_ctx = Secp256k1::new();
    
    let our_keys_manager = new_keys_manager(our_seed, bitcoin_network);
    let local_funding_privkey = our_keys_manager.derive_key(KeyFamily::MultiSig, channel_index);
    let local_funding_pubkey = BitcoinPublicKey::new(
            PublicKey::from_secret_key(&secp_ctx, &local_funding_privkey));
    
    let remote_keys_manager = new_keys_manager(remote_seed, bitcoin_network);
    let remote_funding_privkey = remote_keys_manager.derive_key(KeyFamily::MultiSig, channel_index);
    let remote_funding_pubkey = BitcoinPublicKey::new(
        PublicKey::from_secret_key(&secp_ctx, &remote_funding_privkey));
    
    let input_txid = tx_input.previous_output.txid;
    let input_vout = tx_input.previous_output.vout;
    
    let tx = create_funding_transaction(
        input_txid,
        input_vout,
        funding_amount_sat,
        &local_funding_pubkey,
        &remote_funding_pubkey,
    );
    
    let signed_tx = sign_raw_transaction(bitcoind.clone(), tx).await;
    
    println!("\n✓ Funding Transaction Created\n");
    println!("Tx ID: {}", signed_tx.compute_txid());
    println!("\nTx Hex: {}", serialize_hex(&signed_tx));
    println!();
}

/// Interactive CLI function to create a funding transaction
/// This fetches a UTXO automatically and creates the funding transaction
pub async fn run() {
    
    // Connect to bitcoind
    let bitcoind = get_bitcoind_client().await;
    
    // get an unspent output for funding transaction
    let tx_input = get_unspent_output(bitcoind.clone()).await;

    let tx_in_amount = 5_000_000;
    
    build_funding_tx(bitcoind, tx_input, tx_in_amount).await;

    // Add a delay to allow the spawned task to complete
    sleep(Duration::from_secs(2)).await;
}
