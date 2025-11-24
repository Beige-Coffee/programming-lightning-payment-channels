use crate::transactions::create_funding_transaction;
use crate::types::{ChannelKeyManager, KeyFamily, KeysManager};
use crate::*;
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::consensus::encode::serialize_hex;
use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::{sha256, Hash, HashEngine};
use bitcoin::locktime::absolute::LockTime;
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{PublicKey, Scalar, Secp256k1, SecretKey};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::transaction::Version;
use bitcoin::Network;
use bitcoin::PublicKey as BitcoinPublicKey;
use bitcoin::Txid;
use bitcoin::{Amount, OutPoint, Sequence, Transaction, TxIn, TxOut, Witness};
use serial_test::serial;
use std::str::FromStr;

#[test]
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

    assert_eq!(
        signature,
hex::decode("3044022060fbcd83321e2e409566aeb8032ceee9ac968906151238068f7b0cf9e10b4bd702201f73255bd8bfb895ec3e04fda22500e262d17377923a8783c191a290beac984701").unwrap(),
        "Signature should match expected value"
    );
}

#[test]
fn test_08_derive_revocation_public_key() {
    let secp_ctx = Secp256k1::new();

    // Test vector from BOLT 3
    let revocation_basepoint = PublicKey::from_slice(
        &hex::decode("036d6caac248af96f6afa7f904f550253a0f3ef3f5aa2fe6838a95b216691468e2").unwrap(),
    )
    .unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap(),
    )
    .unwrap();

    // Derive the revocation public key
    let revocation_pubkey =
        derive_revocation_public_key(&revocation_basepoint, &per_commitment_point, &secp_ctx);

    // Expected revocation public key from BOLT 3 test vectors
    let expected_revocation_pubkey = PublicKey::from_slice(
        &hex::decode("02916e326636d19c33f13e8c0c3a03dd157f332f3e99c317c141dd865eb01f8ff0").unwrap(),
    )
    .unwrap();

    assert_eq!(
        revocation_pubkey, expected_revocation_pubkey,
        "Derived revocation public key should match BOLT 3 test vector"
    );
}

#[test]
fn test_09_derive_revocation_private_key() {
    let secp_ctx = Secp256k1::new();

    // Test vector secrets from BOLT 3
    let revocation_basepoint_secret = SecretKey::from_slice(
        &hex::decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f").unwrap(),
    )
    .unwrap();

    let per_commitment_secret = SecretKey::from_slice(
        &hex::decode("1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100").unwrap(),
    )
    .unwrap();

    let expected_private_key = SecretKey::from_slice(
        &hex::decode("d09ffff62ddb2297ab000cc85bcb4283fdeb6aa052affbc9dddcf33b61078110").unwrap(),
    )
    .unwrap();

    // Derive the revocation private key
    let revocation_privkey = derive_revocation_private_key(
        &revocation_basepoint_secret,
        &per_commitment_secret,
        &secp_ctx,
    );

    // The public key derived from the private key should match the directly derived public key
    assert_eq!(
        expected_private_key, revocation_privkey,
        "Public key from private key should match directly derived public key"
    );
}

#[test]
fn test_10_build_commitment_secret() {
    let secp_ctx = Secp256k1::new();

    // BOLT 3 test vector: seed of all zeros, I=281474976710655
    let seed = [0x00; 32];
    let channel_keys = ChannelKeyManager {
        funding_key: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        revocation_base_key: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        payment_base_key: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        delayed_payment_base_key: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        htlc_base_key: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        commitment_seed: seed,
        secp_ctx,
    };

    let secret = channel_keys.build_commitment_secret(281474976710655);
    let expected =
        hex::decode("02a40c85b6f28da08dfdbe0926c53fab2de6d28c10301f8f7c4073d5e42e3148").unwrap();

    assert_eq!(
        secret.as_slice(),
        expected.as_slice(),
        "Commitment secret should match BOLT 3 test vector"
    );
}

#[test]
fn test_11_derive_per_commitment_point() {
    let secp_ctx = Secp256k1::new();

    let seed = [0x00; 32];
    let channel_keys = ChannelKeyManager {
        funding_key: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        revocation_base_key: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        payment_base_key: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        delayed_payment_base_key: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        htlc_base_key: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        commitment_seed: seed,
        secp_ctx: secp_ctx.clone(),
    };

    // Derive per-commitment point
    let per_commitment_point = channel_keys.derive_per_commitment_point(281474976710655);

    // Manually verify it matches the public key of the secret
    let secret = channel_keys.build_commitment_secret(281474976710655);
    let secret_key = SecretKey::from_slice(&secret).unwrap();
    let expected_point = PublicKey::from_secret_key(&secp_ctx, &secret_key);

    assert_eq!(
        per_commitment_point, expected_point,
        "Per-commitment point should be the public key of the commitment secret"
    );
}

#[test]
fn test_12_derive_public_key() {
    let secp_ctx = Secp256k1::new();

    // BOLT 3 test vector
    let basepoint = PublicKey::from_slice(
        &hex::decode("036d6caac248af96f6afa7f904f550253a0f3ef3f5aa2fe6838a95b216691468e2").unwrap(),
    )
    .unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap(),
    )
    .unwrap();

    // Derive the public key
    let derived_pubkey = derive_public_key(&basepoint, &per_commitment_point, &secp_ctx);

    // Expected derived public key from BOLT 3 test vectors
    let expected_pubkey = PublicKey::from_slice(
        &hex::decode("0235f2dbfaa89b57ec7b055afe29849ef7ddfeb1cefdb9ebdc43f5494984db29e5").unwrap(),
    )
    .unwrap();

    assert_eq!(
        derived_pubkey, expected_pubkey,
        "Derived public key should match BOLT 3 test vector"
    );
}

#[test]
fn test_13_derive_private_key() {
    let secp_ctx = Secp256k1::new();

    // BOLT 3 test vector
    let base_secret = SecretKey::from_slice(
        &hex::decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f").unwrap(),
    )
    .unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap(),
    )
    .unwrap();

    let expected_privkey = SecretKey::from_slice(
        &hex::decode("cbced912d3b21bf196a766651e436aff192362621ce317704ea2f75d87e7be0f").unwrap(),
    )
    .unwrap();

    // Derive the private key
    let derived_privkey = derive_private_key(&base_secret, &per_commitment_point, &secp_ctx);

    // Both methods should produce the same public key
    assert_eq!(
        expected_privkey, derived_privkey,
        "Derived private key should match BOLT 3 test vector"
    );
}

#[test]
fn test_14_create_to_remote_script() {
    // BOLT 3 test vector - remote public key
    let remote_pubkey = PublicKey::from_slice(
        &hex::decode("032c0b7cf95324a07d05398b240174dc0c2be444d96b159aa6c7f7b1e668680991").unwrap(),
    )
    .unwrap();

    // Create to_remote script
    let to_remote_script = create_to_remote_script(&remote_pubkey);

    // Expected P2WPKH script from BOLT 3 test vectors
    // Format: OP_0 <20-byte-pubkey-hash>
    let expected_script = hex::decode("0014cc1b07838e387deacd0e5232e1e8b49f4c29e484").unwrap();

    assert_eq!(
        to_remote_script.as_bytes(),
        expected_script.as_slice(),
        "to_remote script should match BOLT 3 test vector"
    );
}

#[test]
fn test_15_create_to_local_script() {
    // BOLT 3 test vectors
    let revocation_pubkey = PublicKey::from_slice(
        &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19").unwrap(),
    )
    .unwrap();

    let local_delayedpubkey = PublicKey::from_slice(
        &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c").unwrap(),
    )
    .unwrap();

    let to_self_delay = 144;

    // Create to_local script
    let to_local_script =
        create_to_local_script(&revocation_pubkey, &local_delayedpubkey, to_self_delay);

    // Expected to_local script from BOLT 3 test vectors
    let expected_script = hex::decode(
        "63210212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b1967029000b2752103fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c68ac"
    ).unwrap();

    assert_eq!(
        to_local_script.as_bytes(),
        expected_script.as_slice(),
        "to_local script should match BOLT 3 test vector"
    );
}

#[test]
fn test_16_get_commitment_transaction_number_obscure_factor() {
    // BOLT 3 test vectors - payment basepoints
    let initiator_payment_basepoint = PublicKey::from_slice(
        &hex::decode("034f355bdcb7cc0af728ef3cceb9615d90684bb5b2ca5f859ab0f0b704075871aa").unwrap(),
    )
    .unwrap();

    let receiver_payment_basepoint = PublicKey::from_slice(
        &hex::decode("032c0b7cf95324a07d05398b240174dc0c2be444d96b159aa6c7f7b1e668680991").unwrap(),
    )
    .unwrap();

    let expected_obscured_factor = 0x2bb038521914;

    // Calculate obscure factor (local is opener/initiator in BOLT 3 test vectors)
    let actual_obscure_factor = get_commitment_transaction_number_obscure_factor(
        &initiator_payment_basepoint,
        &receiver_payment_basepoint,
    );

    assert_eq!(
        actual_obscure_factor, expected_obscured_factor,
        "Obscure number should match BOLT 3 test vector calculation"
    );
}

#[test]
fn test_17_set_obscured_commitment_number() {
    // BOLT 3 test vectors - payment basepoints
    let initiator_payment_basepoint = PublicKey::from_slice(
        &hex::decode("034f355bdcb7cc0af728ef3cceb9615d90684bb5b2ca5f859ab0f0b704075871aa").unwrap(),
    )
    .unwrap();

    let receiver_payment_basepoint = PublicKey::from_slice(
        &hex::decode("032c0b7cf95324a07d05398b240174dc0c2be444d96b159aa6c7f7b1e668680991").unwrap(),
    )
    .unwrap();

    let commitment_number = 42;

    let commitment_transaction_number_obscure_factor =
        get_commitment_transaction_number_obscure_factor(
            &initiator_payment_basepoint,
            &receiver_payment_basepoint,
        );

    let obscured_commitment_transaction_number = commitment_transaction_number_obscure_factor
        ^ (INITIAL_COMMITMENT_NUMBER - commitment_number);

    // Upper 24 bits in locktime
    let locktime_value =
        ((0x20 as u32) << 8 * 3) | ((obscured_commitment_transaction_number & 0xffffff) as u32);

    // Lower 24 bits in sequence
    let sequence_value = Sequence(
        ((0x80 as u32) << 8 * 3) | ((obscured_commitment_transaction_number >> 3 * 8) as u32),
    );

    // Create a simple transaction with one input
    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: Txid::all_zeros(),
                vout: 0,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ZERO,
            witness: Witness::new(),
        }],
        output: vec![],
    };

    // Set obscured commitment number
    set_obscured_commitment_number(
        &mut tx,
        commitment_number,
        &initiator_payment_basepoint,
        &receiver_payment_basepoint,
    );

    // Extract values from transaction
    let actual_locktime_value = tx.lock_time.to_consensus_u32();
    let expected_sequence_value = tx.input[0].sequence;

    assert_eq!(
        actual_locktime_value, locktime_value,
        "Obscured locktime number is incorrect"
    );

    assert_eq!(
        expected_sequence_value, sequence_value,
        "Obscured sequence number is incorrect"
    );
}

#[test]
fn test_18_create_commitment_transaction_outputs() {
    let secp_ctx = Secp256k1::new();

    // BOLT 3 test vector keys - create channel public keys
    let local_delayed_basepoint = PublicKey::from_slice(
        &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c").unwrap(),
    )
    .unwrap();

    let local_htlc_basepoint = PublicKey::from_slice(
        &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7").unwrap(),
    )
    .unwrap();

    let remote_revocation_basepoint = PublicKey::from_slice(
        &hex::decode("036d6caac248af96f6afa7f904f550253a0f3ef3f5aa2fe6838a95b216691468e2").unwrap(),
    )
    .unwrap();

    let remote_htlc_basepoint = PublicKey::from_slice(
        &hex::decode("031fa8d91e4dcfe4b5e9f2e6d2fc3c4eca29b993f6b5c8e5d738e2b75e4c18a5e5").unwrap(),
    )
    .unwrap();

    let remote_payment_basepoint = PublicKey::from_slice(
        &hex::decode("032c0b7cf95324a07d05398b240174dc0c2be444d96b159aa6c7f7b1e668680991").unwrap(),
    )
    .unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap(),
    )
    .unwrap();

    // Create commitment keys using from_basepoints
    let commitment_keys = CommitmentKeys::from_basepoints(
        &per_commitment_point,
        &local_delayed_basepoint,
        &local_htlc_basepoint,
        &remote_revocation_basepoint,
        &remote_htlc_basepoint,
        &secp_ctx,
    );

    let to_self_delay = 144;
    let dust_limit_satoshis = 546;

    // Test case 1: Values above dust limit
    let to_local_value = 7_000_000;
    let to_remote_value = 3_000_000;
    let fee = 0;

    let outputs = create_commitment_transaction_outputs(
        to_local_value,
        to_remote_value,
        &commitment_keys,
        &remote_payment_basepoint,
        to_self_delay,
        dust_limit_satoshis,
        fee,
    );

    // Should have 2 outputs (both above dust limit)
    assert_eq!(
        outputs.len(),
        2,
        "Should have 2 outputs when both values are above dust"
    );

    // Verify outputs exist with correct values
    assert!(
        outputs.iter().any(|o| o.value == to_remote_value),
        "Should have to_remote output"
    );
    assert!(
        outputs.iter().any(|o| o.value == to_local_value - fee),
        "Should have to_local output"
    );

    // Verify to_local is P2WSH
    let to_local_output = outputs
        .iter()
        .find(|o| o.value == to_local_value - fee)
        .unwrap();
    assert!(
        to_local_output.script.is_p2wsh(),
        "to_local should be P2WSH"
    );
    assert_eq!(
        to_local_output.cltv_expiry, None,
        "to_local should have no CLTV expiry"
    );

    // Test case 2: Values below dust limit
    let to_local_dust = 500; // Below dust
    let to_remote_dust = 400; // Below dust

    let outputs_dust = create_commitment_transaction_outputs(
        to_local_dust,
        to_remote_dust,
        &commitment_keys,
        &remote_payment_basepoint,
        to_self_delay,
        dust_limit_satoshis,
        fee,
    );

    // Should have 0 outputs (both below dust limit)
    assert_eq!(
        outputs_dust.len(),
        0,
        "Should have no outputs when both values are below dust limit"
    );
}

#[test]
fn test_19_sort_outputs() {
    // Create scripts with different byte values for sorting
    let script_a = ScriptBuf::from_bytes(vec![0x00, 0x14, 0xaa, 0xaa]);
    let script_b = ScriptBuf::from_bytes(vec![0x00, 0x14, 0xbb, 0xbb]);
    let script_c = ScriptBuf::from_bytes(vec![0x00, 0x14, 0xcc, 0xcc]);

    // Create outputs in unsorted order
    let mut outputs = vec![
        OutputWithMetadata {
            value: 3000,
            script: script_c.clone(),
            cltv_expiry: None,
        },
        OutputWithMetadata {
            value: 1000,
            script: script_b.clone(),
            cltv_expiry: None,
        },
        OutputWithMetadata {
            value: 2000,
            script: script_a.clone(),
            cltv_expiry: None,
        },
    ];

    // Sort outputs
    sort_outputs(&mut outputs);

    // Should be sorted by value: 1000, 2000, 3000
    assert_eq!(
        outputs[0].value, 1000,
        "First output should have lowest value"
    );
    assert_eq!(
        outputs[1].value, 2000,
        "Second output should have middle value"
    );
    assert_eq!(
        outputs[2].value, 3000,
        "Third output should have highest value"
    );

    // Test same value, different scripts
    let mut outputs_same_value = vec![
        OutputWithMetadata {
            value: 1000,
            script: script_c.clone(),
            cltv_expiry: None,
        },
        OutputWithMetadata {
            value: 1000,
            script: script_a.clone(),
            cltv_expiry: None,
        },
        OutputWithMetadata {
            value: 1000,
            script: script_b.clone(),
            cltv_expiry: None,
        },
    ];

    sort_outputs(&mut outputs_same_value);

    // Should be sorted by script: aa, bb, cc
    assert_eq!(
        outputs_same_value[0].script, script_a,
        "First should have script_a"
    );
    assert_eq!(
        outputs_same_value[1].script, script_b,
        "Second should have script_b"
    );
    assert_eq!(
        outputs_same_value[2].script, script_c,
        "Third should have script_c"
    );

    // Test same value and script, different CLTV expiry
    let mut outputs_same_script = vec![
        OutputWithMetadata {
            value: 1000,
            script: script_a.clone(),
            cltv_expiry: Some(550),
        },
        OutputWithMetadata {
            value: 1000,
            script: script_a.clone(),
            cltv_expiry: Some(500),
        },
        OutputWithMetadata {
            value: 1000,
            script: script_a.clone(),
            cltv_expiry: Some(525),
        },
    ];

    sort_outputs(&mut outputs_same_script);

    // Should be sorted by CLTV expiry: 500, 525, 550
    assert_eq!(
        outputs_same_script[0].cltv_expiry,
        Some(500),
        "First should have lowest CLTV"
    );
    assert_eq!(
        outputs_same_script[1].cltv_expiry,
        Some(525),
        "Second should have middle CLTV"
    );
    assert_eq!(
        outputs_same_script[2].cltv_expiry,
        Some(550),
        "Third should have highest CLTV"
    );
}

#[test]
fn test_20_create_commitment_transaction() {
    let secp_ctx = Secp256k1::new();

    // BOLT 3 test vector funding outpoint
    let funding_txid =
        Txid::from_str("8984484a580b825b9972d7adb15050b3ab624ccd731946b3eeddb92f4e7ef6be").unwrap();
    let funding_outpoint = OutPoint {
        txid: funding_txid,
        vout: 0,
    };

    // BOLT 3 test vector keys
    let local_delayed_basepoint = PublicKey::from_slice(
        &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c").unwrap(),
    )
    .unwrap();

    let local_htlc_basepoint = PublicKey::from_slice(
        &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7").unwrap(),
    )
    .unwrap();

    let remote_revocation_basepoint = PublicKey::from_slice(
        &hex::decode("036d6caac248af96f6afa7f904f550253a0f3ef3f5aa2fe6838a95b216691468e2").unwrap(),
    )
    .unwrap();

    let remote_htlc_basepoint = PublicKey::from_slice(
        &hex::decode("032c0b7cf95324a07d05398b240174dc0c2be444d96b159aa6c7f7b1e668680991").unwrap(),
    )
    .unwrap();

    let local_payment_basepoint = PublicKey::from_slice(
        &hex::decode("034f355bdcb7cc0af728ef3cceb9615d90684bb5b2ca5f859ab0f0b704075871aa").unwrap(),
    )
    .unwrap();

    let remote_payment_basepoint = PublicKey::from_slice(
        &hex::decode("032c0b7cf95324a07d05398b240174dc0c2be444d96b159aa6c7f7b1e668680991").unwrap(),
    )
    .unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap(),
    )
    .unwrap();

    // Create commitment keys
    let commitment_keys = CommitmentKeys::from_basepoints(
        &per_commitment_point,
        &local_delayed_basepoint,
        &local_htlc_basepoint,
        &remote_revocation_basepoint,
        &remote_htlc_basepoint,
        &secp_ctx,
    );

    // BOLT 3 test vector values
    let to_local_value = 7_000_000;
    let to_remote_value = 3_000_000;
    let commitment_number = 42;
    let to_self_delay = 144;
    let dust_limit_satoshis = 546;
    let feerate_per_kw = 0; // Use 0 fee for simpler verification

    // Test without HTLCs
    let tx = create_commitment_transaction(
        funding_outpoint,
        to_local_value,
        to_remote_value,
        &commitment_keys,
        &local_payment_basepoint,
        &remote_payment_basepoint,
        commitment_number,
        to_self_delay,
        dust_limit_satoshis,
        feerate_per_kw,
        &[], // no offered HTLCs
        &[], // no received HTLCs
    );

    // Verify transaction structure
    assert_eq!(tx.version, Version::TWO, "Version should be 2");
    assert_eq!(tx.input.len(), 1, "Should have 1 input");
    assert_eq!(
        tx.output.len(),
        2,
        "Should have 2 outputs (to_local and to_remote)"
    );

    // Verify input references funding outpoint
    assert_eq!(
        tx.input[0].previous_output.txid, funding_txid,
        "Input should reference funding txid"
    );
    assert_eq!(
        tx.input[0].previous_output.vout, 0,
        "Input should reference vout 0"
    );

    // Verify obscured commitment number is set
    let locktime_value = tx.lock_time.to_consensus_u32();
    let sequence_value = tx.input[0].sequence.0;
    assert_eq!(
        locktime_value >> 24,
        0x20,
        "Locktime upper byte should be 0x20"
    );
    assert_eq!(
        sequence_value >> 24,
        0x80,
        "Sequence upper byte should be 0x80"
    );

    // Verify outputs are sorted by value (BIP69)
    assert!(
        tx.output[0].value.to_sat() <= tx.output[1].value.to_sat(),
        "Outputs should be sorted by value"
    );

    // Test with HTLCs
    let offered_htlcs = vec![HTLCOutput {
        amount_sat: 2_000_000,
        payment_hash: Sha256::hash(&[0x02; 32]).to_byte_array(),
        cltv_expiry: 502,
    }];

    let received_htlcs = vec![HTLCOutput {
        amount_sat: 1_000_000,
        payment_hash: Sha256::hash(&[0x00; 32]).to_byte_array(),
        cltv_expiry: 500,
    }];

    let tx_with_htlcs = create_commitment_transaction(
        funding_outpoint,
        to_local_value,
        to_remote_value,
        &commitment_keys,
        &local_payment_basepoint,
        &remote_payment_basepoint,
        commitment_number,
        to_self_delay,
        dust_limit_satoshis,
        feerate_per_kw,
        &offered_htlcs,
        &received_htlcs,
    );

    // Should have more outputs with HTLCs (2 base + 2 HTLCs = 4)
    assert_eq!(
        tx_with_htlcs.output.len(),
        4,
        "Should have 4 outputs with HTLCs"
    );

    // Verify outputs are still sorted
    for i in 0..tx_with_htlcs.output.len() - 1 {
        assert!(
            tx_with_htlcs.output[i].value <= tx_with_htlcs.output[i + 1].value,
            "Outputs should be sorted by value"
        );
    }
}

#[test]
fn test_21_finalize_holder_commitment() {
    let secp_ctx = Secp256k1::new();

    // BOLT 3 test vector secrets (remove trailing 01 from hex)
    let local_funding_privkey = SecretKey::from_slice(
        &hex::decode("30ff4956bbdd3222d44cc5e8a1261dab1e07957bdac5ae88fe3261ef321f3749").unwrap()
    ).unwrap();

    // BOLT 3 basepoint secrets
    let local_payment_basepoint_secret = SecretKey::from_slice(
        &hex::decode("1111111111111111111111111111111111111111111111111111111111111111").unwrap()
    ).unwrap();

    let local_delayed_payment_basepoint_secret = SecretKey::from_slice(
        &hex::decode("3333333333333333333333333333333333333333333333333333333333333333").unwrap()
    ).unwrap();

    let local_htlc_basepoint_secret = SecretKey::from_slice(
        &hex::decode("1111111111111111111111111111111111111111111111111111111111111111").unwrap()
    ).unwrap();

    let local_revocation_basepoint_secret = SecretKey::from_slice(
        &hex::decode("2222222222222222222222222222222222222222222222222222222222222222").unwrap()
    ).unwrap();

    // BOLT 3 commitment seed (all zeros)
    let commitment_seed = [0x00u8; 32];

    // Build ChannelKeyManager
    let channel_keys = ChannelKeyManager {
        funding_key: local_funding_privkey,
        revocation_base_key: local_revocation_basepoint_secret,
        payment_base_key: local_payment_basepoint_secret,
        delayed_payment_base_key: local_delayed_payment_basepoint_secret,
        htlc_base_key: local_htlc_basepoint_secret,
        commitment_seed,
        secp_ctx: secp_ctx.clone(),
    };

    // BOLT 3 funding pubkeys
    let local_funding_pubkey = BitcoinPublicKey::new(PublicKey::from_slice(
        &hex::decode("023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb").unwrap()
    ).unwrap());

    let remote_funding_pubkey = BitcoinPublicKey::new(PublicKey::from_slice(
        &hex::decode("030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c1").unwrap()
    ).unwrap());

    // BOLT 3 payment basepoints (for obscured commitment number)
    let local_payment_basepoint = PublicKey::from_slice(
        &hex::decode("034f355bdcb7cc0af728ef3cceb9615d90684bb5b2ca5f859ab0f0b704075871aa").unwrap()
    ).unwrap();

    let remote_payment_basepoint = PublicKey::from_slice(
        &hex::decode("032c0b7cf95324a07d05398b240174dc0c2be444d96b159aa6c7f7b1e668680991").unwrap()
    ).unwrap();

    // Funding outpoint from BOLT 3
    let funding_txid = Txid::from_str("8984484a580b825b9972d7adb15050b3ab624ccd731946b3eeddb92f4e7ef6be").unwrap();
    let funding_outpoint = OutPoint {
        txid: funding_txid,
        vout: 0,
    };
    let funding_amount = 10_000_000;

    // Create funding script
    let funding_script = create_funding_script(&local_funding_pubkey, &remote_funding_pubkey);

    // BOLT 3 commitment number is 42
    let bolt3_commitment_number = 42;
    let commitment_number = INITIAL_COMMITMENT_NUMBER - bolt3_commitment_number;

    // Derive per_commitment_point for this commitment
    let per_commitment_point = channel_keys.derive_per_commitment_point(commitment_number);

    // Use exact derived keys from BOLT 3 test vectors
    let commitment_keys = CommitmentKeys::from_keys(
        per_commitment_point,
        PublicKey::from_slice(
            &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19").unwrap()
        ).unwrap(), // revocation_key
        PublicKey::from_slice(
            &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c").unwrap()
        ).unwrap(), // local_delayed_payment_key
        PublicKey::from_slice(
            &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7").unwrap()
        ).unwrap(), // local_htlc_key
        PublicKey::from_slice(
            &hex::decode("0394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b").unwrap()
        ).unwrap(), // remote_htlc_key
    );

    // BOLT 3 test vector values
    let to_local_msat = 7_000_000_000;
    let to_remote_msat = 3_000_000_000;
    let to_local_sat = to_local_msat / 1000;
    let to_remote_sat = to_remote_msat / 1000;
    let to_self_delay = 144;
    let dust_limit_satoshis = 546;
    let feerate_per_kw = 15000;

    // Create unsigned commitment transaction
    let unsigned_tx = create_commitment_transaction(
        funding_outpoint,
        to_local_sat,
        to_remote_sat,
        &commitment_keys,
        &local_payment_basepoint,
        &remote_payment_basepoint,
        commitment_number,
        to_self_delay,
        dust_limit_satoshis,
        feerate_per_kw,
        &[],
        &[],
    );

    // BOLT 3 expected remote signature (with SIGHASH_ALL appended)
    let remote_signature = hex::decode(
        "3045022100c3127b33dcc741dd6b05b1e63cbd1a9a7d816f37af9b6756fa2376b056f032370220408b96279808fe57eb7e463710804cdf4f108388bc5cf722d8c848d2c7f9f3b001"
    ).unwrap();

    // Finalize the holder commitment
    let signed_tx = finalize_holder_commitment(
        channel_keys,
        unsigned_tx,
        0,
        &funding_script,
        funding_amount,
        remote_signature,
    );

    // BOLT 3 expected complete transaction
    let expected_tx_hex = "02000000000101bef67e4e2fb9ddeeb3461973cd4c62abb35050b1add772995b820b584a488489000000000038b02b8002c0c62d0000000000160014cc1b07838e387deacd0e5232e1e8b49f4c29e48454a56a00000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e04004730440220616210b2cc4d3afb601013c373bbd8aac54febd9f15400379a8cb65ce7deca60022034236c010991beb7ff770510561ae8dc885b8d38d1947248c38f2ae05564714201483045022100c3127b33dcc741dd6b05b1e63cbd1a9a7d816f37af9b6756fa2376b056f032370220408b96279808fe57eb7e463710804cdf4f108388bc5cf722d8c848d2c7f9f3b001475221023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb21030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c152ae3e195220";

    // Serialize and compare
    let actual_tx_hex = hex::encode(bitcoin::consensus::serialize(&signed_tx));

    assert_eq!(
        actual_tx_hex,
        expected_tx_hex,
        "Finalized commitment transaction should match BOLT 3 test vector"
    );
}