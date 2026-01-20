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
use crate::internal::helper::{get_unspent_output, get_outpoint, sign_raw_transaction};
use crate::internal::bitcoind_client::{BitcoindClient, get_bitcoind_client};
use crate::scripts::funding::create_funding_script;
use crate::keys::derivation::new_keys_manager;
use bitcoin::sighash::SighashCache;
use bitcoin::secp256k1;
use crate::transactions::funding::create_funding_transaction;
use bitcoin::sighash::EcdsaSighashType;
use bitcoin::Network;
use crate::types::{KeyFamily};
use bitcoin::PublicKey as BitcoinPublicKey;
use bitcoin::secp256k1::ecdsa::Signature;
use bitcoin::secp256k1::Message;

pub fn build_simple_htlc_spend_tx(
    bitcoind: BitcoindClient,
    txid: String,
    htlc_amount_sat: u64,
) { 
    let alice_seed = [0x01; 32];
    let bob_seed = [0x02; 32];
    let bitcoin_network = Network::Bitcoin;
    let channel_index = 0;
    let txid_index = 0;
    let secp_ctx = Secp256k1::new();

    let alice_keys_manager = new_keys_manager(alice_seed, bitcoin_network);
    let alice_privkey = alice_keys_manager.derive_key(KeyFamily::MultiSig, channel_index);
    let alice_pubkey = BitcoinPublicKey::new(
            PublicKey::from_secret_key(&secp_ctx, &alice_privkey));

    let bob_keys_manager = new_keys_manager(bob_seed, bitcoin_network);
    let bob_privkey = bob_keys_manager.derive_key(KeyFamily::MultiSig, channel_index);
    let bob_pubkey = BitcoinPublicKey::new(
        PublicKey::from_secret_key(&secp_ctx, &bob_privkey));


    let simple_htlc_outpoint = get_outpoint(txid.to_string(), txid_index);

    let output_script = ScriptBuf::new_p2wpkh(&bob_pubkey.wpubkey_hash().unwrap());

    let output = TxOut {
            value: Amount::from_sat(404_000),
            script_pubkey: output_script,
        };

    let version = Version::TWO;
    let locktime = LockTime::ZERO;

    let input_txid = simple_htlc_outpoint.txid;
    let input_vout = simple_htlc_outpoint.vout;

    let tx_input = TxIn {
            previous_output: OutPoint {
                txid: input_txid,
                vout: input_vout,
            },
            script_sig: ScriptBuf::new(), // Empty for SegWit
            sequence: Sequence::MAX,      // 0xffffffff (RBF disabled)
            witness: Witness::new(),      // Witness will be added when signing
        };

    let tx = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![tx_input],
            output: vec![output],
        };

    

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

    let signed_tx = sign_transaction(
            tx,
            bob_pubkey,
            alice_pubkey,
            bob_privkey,
            );

    println!("\n✅ Simple HTLC Transaction Created\n");
    println!("Tx ID: {}", signed_tx.compute_txid());
    println!("\nTx Hex: {}", serialize_hex(&signed_tx));
    println!();
}

/// Interactive CLI function to create a Funding Transaction
/// This fetches a UTXO automatically and creates the Funding Transaction
pub fn run(simple_htlc_txid: String) {
    // Connect to bitcoind
    let bitcoind = get_bitcoind_client();

    let htlc_amount_sat = 405_000;

    build_simple_htlc_spend_tx(bitcoind, simple_htlc_txid.clone(), htlc_amount_sat);
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

pub fn sign_transaction(
    tx: Transaction,
    bob_pubkey: BitcoinPublicKey,
    alice_pubkey: BitcoinPublicKey,
    bob_privkey: SecretKey,
    )-> Transaction {

    let funding_amount = 405_000;
    let txid_index = 0;

    let secret = "ProgrammingLightning".to_string();
    let secret_bytes = secret.as_bytes();
    let payment_hash = Sha256::hash(secret_bytes).to_byte_array();
    let payment_hash160 = Ripemd160::hash(&payment_hash).to_byte_array();

    let redeem_script = build_hash_locked_script(
        &alice_pubkey,
        &bob_pubkey,
        &payment_hash160
    );

    let mut signed_tx = tx.clone();

    let signature = generate_p2wsh_signature(
         tx.clone(), 
         txid_index,
         &redeem_script,
         funding_amount,
         EcdsaSighashType::All,
        bob_privkey);

    // Convert signature to DER and append SigHashType
    let mut signature_der = signature.serialize_der().to_vec();
    signature_der.push(EcdsaSighashType::All as u8);

    signed_tx.input[0].witness.push(signature_der);

    signed_tx.input[0].witness.push(secret_bytes);

    signed_tx.input[0].witness.push(vec!(1));

    // push witness
    signed_tx.input[0]
        .witness
        .push(redeem_script.clone().into_bytes());

    signed_tx
}

pub fn generate_p2wsh_signature(
    transaction: Transaction,
    input_idx: usize,
    witness_script: &ScriptBuf,
    value: u64,
    sighash_type: EcdsaSighashType,
    private_key: secp256k1::SecretKey,
) -> Signature {
    let secp = Secp256k1::new();

    let message =
        generate_p2wsh_message(transaction, input_idx, witness_script, value, sighash_type);
    let signature = secp.sign_ecdsa(&message, &private_key);

    signature
}

fn generate_p2wsh_message(
    transaction: Transaction,
    input_idx: usize,
    witness_script: &ScriptBuf,
    value: u64,
    sighash_type: EcdsaSighashType,
) -> Message {
    let secp = Secp256k1::new();

    let mut cache = SighashCache::new(&transaction);

    let amount = Amount::from_sat(value);

    let sighash = cache
        .p2wsh_signature_hash(input_idx, &witness_script, amount, sighash_type)
        .unwrap();

    let message = Message::from_digest_slice(&sighash[..]).unwrap();

    message
}