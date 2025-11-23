use crate::transactions::create_funding_transaction;
use crate::types::{ChannelKeyManager, KeyFamily, KeysManager};
use crate::*;
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::hashes::{sha256, Hash, HashEngine};
use bitcoin::secp256k1::{PublicKey, Scalar, Secp256k1, SecretKey};
use bitcoin::Network;
use bitcoin::PublicKey as BitcoinPublicKey;
use serial_test::serial;
use std::str::FromStr;
use bitcoin::sighash::{EcdsaSighashType, SighashCache};

#[test]
#[serial]
fn test_01_new_keys_manager() {
    let seed = [0x01; 32];
    let bitcoin_network = Network::Bitcoin;

    let keys_manager = new_keys_manager(seed, bitcoin_network);

    assert_eq!(
        keys_manager.network, bitcoin_network,
        "Network should match the input network"
    );

    let expected_master_key = Xpriv::new_master(bitcoin_network, &seed).unwrap();
    assert_eq!(
        keys_manager.master_key.to_string(),
        expected_master_key.to_string(),
        "Master key should be derived correctly from seed"
    );
}

#[test]
#[serial]
fn test_02_derive_key() {
    let seed = [0x01; 32];
    let bitcoin_network = Network::Bitcoin;
    let multisig_index = 0;
    let channel_index = 0;
    let keys_manager = new_keys_manager(seed, bitcoin_network);

    // Manually derive the expected key using the same path
    let path_str = format!("m/1017'/0'/{}'/0/{}", multisig_index, channel_index);
    let path = DerivationPath::from_str(&path_str).unwrap();
    let expected_derived = keys_manager
        .master_key
        .derive_priv(&keys_manager.secp_ctx, &path)
        .unwrap();
    let expected_funding_key = expected_derived.private_key;

    // Use the derive_key method
    let actual_funding_key = keys_manager.derive_key(KeyFamily::MultiSig, channel_index);

    assert_eq!(
        expected_funding_key.secret_bytes(),
        actual_funding_key.secret_bytes(),
        "Derived funding key should match expected key"
    );
}

#[test]
#[serial]
fn test_03_derive_channel_keys() {
    let seed = [0x01; 32];
    let bitcoin_network = Network::Bitcoin;
    let channel_index = 0;
    let keys_manager = new_keys_manager(seed, bitcoin_network);

    // Derive all channel keys at once
    let channel_keys = keys_manager.derive_channel_keys(channel_index);

    // Manually derive each key to verify
    let expected_funding_key = keys_manager.derive_key(KeyFamily::MultiSig, channel_index);
    let expected_revocation_key = keys_manager.derive_key(KeyFamily::RevocationBase, channel_index);
    let expected_payment_key = keys_manager.derive_key(KeyFamily::PaymentBase, channel_index);
    let expected_delayed_key = keys_manager.derive_key(KeyFamily::DelayBase, channel_index);
    let expected_htlc_key = keys_manager.derive_key(KeyFamily::HtlcBase, channel_index);
    let expected_commitment_seed = keys_manager
        .derive_key(KeyFamily::CommitmentSeed, channel_index)
        .secret_bytes();

    // Verify all keys match
    assert_eq!(
        channel_keys.funding_key.secret_bytes(),
        expected_funding_key.secret_bytes(),
        "Funding key should match"
    );
    assert_eq!(
        channel_keys.revocation_base_key.secret_bytes(),
        expected_revocation_key.secret_bytes(),
        "Revocation base key should match"
    );
    assert_eq!(
        channel_keys.payment_base_key.secret_bytes(),
        expected_payment_key.secret_bytes(),
        "Payment base key should match"
    );
    assert_eq!(
        channel_keys.delayed_payment_base_key.secret_bytes(),
        expected_delayed_key.secret_bytes(),
        "Delayed payment base key should match"
    );
    assert_eq!(
        channel_keys.htlc_base_key.secret_bytes(),
        expected_htlc_key.secret_bytes(),
        "HTLC base key should match"
    );
    assert_eq!(
        channel_keys.commitment_seed, expected_commitment_seed,
        "Commitment seed should match"
    );
}

#[test]
#[serial]
fn test_04_to_public_keys() {
    let seed = [0x01; 32];
    let bitcoin_network = Network::Bitcoin;
    let channel_index = 0;
    let keys_manager = new_keys_manager(seed, bitcoin_network);

    // Derive channel keys
    let channel_keys = keys_manager.derive_channel_keys(channel_index);

    // Convert to public keys
    let public_keys = channel_keys.to_public_keys();

    // Manually derive each public key to verify
    let expected_funding_pubkey =
        PublicKey::from_secret_key(&channel_keys.secp_ctx, &channel_keys.funding_key);
    let expected_revocation_basepoint =
        PublicKey::from_secret_key(&channel_keys.secp_ctx, &channel_keys.revocation_base_key);
    let expected_payment_basepoint =
        PublicKey::from_secret_key(&channel_keys.secp_ctx, &channel_keys.payment_base_key);
    let expected_delayed_payment_basepoint = PublicKey::from_secret_key(
        &channel_keys.secp_ctx,
        &channel_keys.delayed_payment_base_key,
    );
    let expected_htlc_basepoint =
        PublicKey::from_secret_key(&channel_keys.secp_ctx, &channel_keys.htlc_base_key);

    // Verify all public keys match
    assert_eq!(
        public_keys.funding_pubkey, expected_funding_pubkey,
        "Funding public key should match"
    );
    assert_eq!(
        public_keys.revocation_basepoint, expected_revocation_basepoint,
        "Revocation basepoint should match"
    );
    assert_eq!(
        public_keys.payment_basepoint, expected_payment_basepoint,
        "Payment basepoint should match"
    );
    assert_eq!(
        public_keys.delayed_payment_basepoint, expected_delayed_payment_basepoint,
        "Delayed payment basepoint should match"
    );
    assert_eq!(
        public_keys.htlc_basepoint, expected_htlc_basepoint,
        "HTLC basepoint should match"
    );
}

#[test]
fn test_05_create_funding_script() {
    // Test vector pubkeys
    let local_pubkey_hex = "023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb";
    let remote_pubkey_hex = "030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c1";

    let local_pubkey = BitcoinPublicKey::new(
        PublicKey::from_slice(&hex::decode(local_pubkey_hex).unwrap()).unwrap(),
    );

    let remote_pubkey = BitcoinPublicKey::new(
        PublicKey::from_slice(&hex::decode(remote_pubkey_hex).unwrap()).unwrap(),
    );

    // Expected funding witness script from BOLT 3
    let expected_script_hex = "5221023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb21030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c152ae";
    let expected_script = hex::decode(expected_script_hex).unwrap();

    // Create funding script using our function
    let funding_script = create_funding_script(&local_pubkey, &remote_pubkey);
    let actual_script = funding_script.as_bytes();

    assert_eq!(
        actual_script,
        expected_script.as_slice(),
        "Funding script does not match BOLT 3 test vector"
    );
}

#[test]
fn test_06_create_funding_transaction() {
    let our_seed = [0x01; 32];
    let remote_seed = [0x02; 32];
    let bitcoin_network = Network::Bitcoin;
    let channel_index = 0;
    let secp_ctx = Secp256k1::new();

    let our_keys_manager = new_keys_manager(our_seed, bitcoin_network);
    let local_funding_privkey = our_keys_manager.derive_key(KeyFamily::MultiSig, channel_index);
    let local_funding_pubkey = BitcoinPublicKey::new(PublicKey::from_secret_key(
        &secp_ctx,
        &local_funding_privkey,
    ));

    let remote_keys_manager = new_keys_manager(remote_seed, bitcoin_network);
    let remote_funding_privkey = remote_keys_manager.derive_key(KeyFamily::MultiSig, channel_index);
    let remote_funding_pubkey = BitcoinPublicKey::new(PublicKey::from_secret_key(
        &secp_ctx,
        &remote_funding_privkey,
    ));

    // Funding transaction details
    let txid_hex = "8984484a580b825b9972d7adb15050b3ab624ccd731946b3eeddb92f4e7ef6be";
    let mut txid_bytes = hex::decode(txid_hex).unwrap();
    txid_bytes.reverse(); // Bitcoin uses little-endian for txids

    let input_txid = bitcoin::Txid::from_slice(&txid_bytes).unwrap();

    let input_vout = 0;
    let funding_amount_sat = 500000;

    let tx = create_funding_transaction(
        input_txid,
        input_vout,
        funding_amount_sat,
        &local_funding_pubkey,
        &remote_funding_pubkey,
    );
}



#[test]
fn test_07_sign_transaction_input() {
    let our_seed = [0x01; 32];
    let remote_seed = [0x02; 32];
    let bitcoin_network = Network::Bitcoin;
    let channel_index = 0;
    let secp_ctx = Secp256k1::new();

    let our_keys_manager = new_keys_manager(our_seed, bitcoin_network);
    let channel_keys = our_keys_manager.derive_channel_keys(channel_index);
    let local_funding_privkey = our_keys_manager.derive_key(KeyFamily::MultiSig, channel_index);
    let local_funding_pubkey = BitcoinPublicKey::new(PublicKey::from_secret_key(
        &secp_ctx,
        &local_funding_privkey,
    ));

    let remote_keys_manager = new_keys_manager(remote_seed, bitcoin_network);
    let remote_funding_privkey = remote_keys_manager.derive_key(KeyFamily::MultiSig, channel_index);
    let remote_funding_pubkey = BitcoinPublicKey::new(PublicKey::from_secret_key(
        &secp_ctx,
        &remote_funding_privkey,
    ));

    // Funding transaction details
    let txid_hex = "8984484a580b825b9972d7adb15050b3ab624ccd731946b3eeddb92f4e7ef6be";
    let mut txid_bytes = hex::decode(txid_hex).unwrap();
    txid_bytes.reverse(); // Bitcoin uses little-endian for txids

    let input_txid = bitcoin::Txid::from_slice(&txid_bytes).unwrap();

    let input_vout = 0;
    let funding_amount_sat = 500000;

    let funding_script = create_funding_script(&local_funding_pubkey, &remote_funding_pubkey);

    let tx = create_funding_transaction(
        input_txid,
        input_vout,
        funding_amount_sat,
        &local_funding_pubkey,
        &remote_funding_pubkey,
    );

    // Sign the transaction input
    let signature = channel_keys.sign_transaction_input(
        &tx,
        0,
        &funding_script,
        funding_amount_sat,
        &channel_keys.funding_key,
    );

    assert_eq!(
        *signature.last().unwrap(),
        EcdsaSighashType::All as u8,
        "Signature should end with SIGHASH_ALL (0x01)"
    );

    // Print the transaction hex after it was signed
    let tx_hex = hex::encode(&tx.serialize());
    println!("Transaction hex after signing: {}", tx_hex);
}