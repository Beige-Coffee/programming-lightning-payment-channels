use bitcoin::{Transaction, TxIn, TxOut, OutPoint, Sequence, Witness, Amount};
use bitcoin::script::ScriptBuf;
use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::blockdata::opcodes::all as opcodes;
use bitcoin::secp256k1::{Secp256k1, PublicKey, SecretKey};
use bitcoin::hashes::ripemd160::Hash as Ripemd160;
use bitcoin::hashes::Hash;
use bitcoin::script::{Builder};
use bitcoin::transaction::Version;
use bitcoin::locktime::absolute::LockTime;
use bitcoin::consensus::encode::serialize_hex;
use crate::internal::helper::{get_unspent_output, sign_raw_transaction};
use crate::internal::bitcoind_client::{BitcoindClient, get_bitcoind_client};
use crate::scripts::funding::create_funding_script;
use crate::keys::derivation::new_keys_manager;
use crate::transactions::funding::create_funding_transaction;
use bitcoin::Network;
use crate::types::{KeyFamily};
use bitcoin::PublicKey as BitcoinPublicKey;

pub fn build_simple_htlc_tx(
    bitcoind: BitcoindClient,
    tx_input: TxIn,
    htlc_amount_sat: u64,
) { 
    let alice_seed = [0x01; 32];
    let bob_seed = [0x02; 32];
    let bitcoin_network = Network::Bitcoin;
    let channel_index = 0;
    let secp_ctx = Secp256k1::new();

    let alice_keys_manager = new_keys_manager(alice_seed, bitcoin_network);
    let alice_privkey = alice_keys_manager.derive_key(KeyFamily::MultiSig, channel_index);
    let alice_pubkey = BitcoinPublicKey::new(
            PublicKey::from_secret_key(&secp_ctx, &alice_privkey));

    let alice_pubkey_hex = alice_pubkey.to_string();
    println!("Alice's Public Key (Hex): {}", alice_pubkey_hex);

    let bob_keys_manager = new_keys_manager(bob_seed, bitcoin_network);
    let bob_privkey = bob_keys_manager.derive_key(KeyFamily::MultiSig, channel_index);
    let bob_pubkey = BitcoinPublicKey::new(
        PublicKey::from_secret_key(&secp_ctx, &bob_privkey));

    let bob_pubkey_hex = bob_pubkey.to_string();
    println!("Bob's Public Key (Hex): {}", bob_pubkey_hex);

    let input_txid = tx_input.previous_output.txid;
    let input_vout = tx_input.previous_output.vout;

    // preimage
    let secret = "ProgrammingLightning".to_string();
    let secret_bytes = secret.as_bytes();
    let payment_hash = Sha256::hash(secret_bytes).to_byte_array();
    let payment_hash160 = Ripemd160::hash(&payment_hash).to_byte_array();

    let htlc_script = build_hash_locked_script(
        &alice_pubkey,
        &bob_pubkey,
        &payment_hash160
    );

    // Convert to P2WSH (pay-to-witness-script-hash)
    let output_script = htlc_script.to_p2wsh();

    let tx_input = TxIn {
            previous_output: OutPoint {
                txid: input_txid,
                vout: input_vout,
            },
            script_sig: ScriptBuf::new(), // Empty for SegWit
            sequence: Sequence::MAX,      // 0xffffffff (RBF disabled)
            witness: Witness::new(),      // Witness will be added when signing
        };

    let output = TxOut {
            value: Amount::from_sat(htlc_amount_sat),
            script_pubkey: output_script,
        };


    // Create the transaction
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![tx_input],
        output: vec![output],
    };

    let signed_tx = sign_raw_transaction(bitcoind.clone(), tx);

    println!("\n✅ Simple HTLC Transaction Created\n");
    println!("Tx ID: {}", signed_tx.compute_txid());
    println!("\nTx Hex: {}", serialize_hex(&signed_tx));
    println!();
}

/// Interactive CLI function to create a Funding Transaction
/// This fetches a UTXO automatically and creates the Funding Transaction
pub fn run() {
    // Connect to bitcoind
    let bitcoind = get_bitcoind_client();

    // get an unspent output for Funding Transaction
    let tx_input = get_unspent_output(bitcoind.clone());

    let htlc_amount_sat = 405_000;

    build_simple_htlc_tx(bitcoind, tx_input, htlc_amount_sat);
}


fn build_hash_locked_script(
    alice_pubkey: &BitcoinPublicKey,
    bob_pubkey: &BitcoinPublicKey,
    payment_hash160: &[u8; 20]) -> ScriptBuf {
    
    Builder::new()
        .push_opcode(opcodes::OP_IF)
        .push_opcode(opcodes::OP_HASH160)
        .push_slice(payment_hash160)
        .push_opcode(opcodes::OP_EQUALVERIFY)
        .push_key(bob_pubkey)
        .push_opcode(opcodes::OP_CHECKSIG)
        .push_opcode(opcodes::OP_ELSE)
        .push_int(200)
        .push_opcode(opcodes::OP_CLTV)
        .push_opcode(opcodes::OP_DROP)
        .push_key(alice_pubkey)
        .push_opcode(opcodes::OP_CHECKSIG)
        .push_opcode(opcodes::OP_ENDIF)
    .into_script()
}