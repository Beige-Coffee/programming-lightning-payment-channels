#![allow(dead_code, unused_imports, unused_variables, unused_must_use)]
use crate::internal;
use bitcoin::amount::Amount;
use bitcoin::blockdata::opcodes::all as opcodes;
use bitcoin::consensus::encode;
use bitcoin::consensus::encode::serialize_hex;
use bitcoin::hash_types::Txid;
use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::Hash;
use bitcoin::hashes::HashEngine;
use bitcoin::locktime::absolute::LockTime;
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1;
use bitcoin::secp256k1::ecdsa::Signature;
use bitcoin::secp256k1::Message;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::secp256k1::{PublicKey as secp256k1PublicKey, Scalar, SecretKey};
use bitcoin::sighash::EcdsaSighashType;
use bitcoin::sighash::SighashCache;
use bitcoin::transaction::Version;
use bitcoin::Network;
use bitcoin::PubkeyHash;
use bitcoin::PublicKey;
use bitcoin::{OutPoint, Sequence, Transaction, TxIn, TxOut, Witness};
use internal::bitcoind_client::BitcoindClient;
use internal::hex_utils;
use std::env;

pub fn get_outpoint(input_tx_id_str: String, vout: usize) -> OutPoint {

    // Get an unspent output to spend
    let mut tx_id_bytes = hex::decode(input_tx_id_str).expect("Valid hex string");
    tx_id_bytes.reverse();
    let input_txid = Txid::from_byte_array(tx_id_bytes.try_into().expect("Expected 32 bytes"));

    OutPoint {
            txid: input_txid,
            vout: vout as u32,
        }
}

pub async fn get_unspent_output(bitcoind: BitcoindClient) -> TxIn {
    let utxos = bitcoind.list_unspent().await;
    let utxo = utxos
        .0
        .iter()
        .find(|utxo| utxo.amount > 4_999_999 && utxo.amount < 6_000_000)
        .expect("No UTXOs with positive balance found");

    let tx_input = TxIn {
        previous_output: OutPoint {
            txid: utxo.txid,
            vout: utxo.vout,
        },
        sequence: Sequence::MAX,
        script_sig: ScriptBuf::new(),
        witness: Witness::new(),
    };

    tx_input
}

pub async fn sign_raw_transaction(bitcoind: BitcoindClient, tx: Transaction) -> Transaction {
    // we need to serialize the tx before passing it into
    //    `sign_raw_transaction_with_wallet`
    let tx_hex = serialize_hex(&tx);

    // sign the transaction
    let signed_tx = bitcoind.sign_raw_transaction_with_wallet(tx_hex).await;

    // convert signed transaction hex into a Transaction type
    let final_tx: Transaction =
        encode::deserialize(&hex_utils::to_vec(&signed_tx.hex).unwrap()).unwrap();

    final_tx
}
