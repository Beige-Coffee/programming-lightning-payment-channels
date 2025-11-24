use crate::transactions::create_funding_transaction;
use crate::types::{ChannelKeyManager, KeyFamily, KeysManager};
use crate::*;
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::consensus::encode::serialize_hex;
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
        ((0x20 as u32) << 8 * 3) | ((obscured_commitment_transaction_number & 0xffffffu64) as u32);

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
