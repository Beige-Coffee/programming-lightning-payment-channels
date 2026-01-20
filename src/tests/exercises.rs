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
    let channel_index = 0;
    let keys_manager = new_keys_manager(seed, bitcoin_network);

    // Test all key families
    let key_families = vec![
        KeyFamily::MultiSig,
        KeyFamily::RevocationBase,
        KeyFamily::HtlcBase,
        KeyFamily::PaymentBase,
        KeyFamily::DelayBase,
        KeyFamily::CommitmentSeed,
    ];

    for key_family in key_families {
        // Manually derive the expected key using the same path
        let path_str = format!("m/1017'/0'/{}'/0/{}", key_family as u32, channel_index);
        let path = DerivationPath::from_str(&path_str).unwrap();
        let expected_derived = keys_manager
            .master_key
            .derive_priv(&keys_manager.secp_ctx, &path)
            .unwrap();
        let expected_key = expected_derived.private_key;

        // Use the derive_key method
        let actual_key = keys_manager.derive_key(key_family, channel_index);

        assert_eq!(
            expected_key.secret_bytes(),
            actual_key.secret_bytes(),
            "Derived key for {:?} should match expected key",
            key_family
        );
    }
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
        channel_keys.revocation_basepoint_secret.secret_bytes(),
        expected_revocation_key.secret_bytes(),
        "Revocation base key should match"
    );
    assert_eq!(
        channel_keys.payment_basepoint_secret.secret_bytes(),
        expected_payment_key.secret_bytes(),
        "Payment base key should match"
    );
    assert_eq!(
        channel_keys.delayed_payment_basepoint_secret.secret_bytes(),
        expected_delayed_key.secret_bytes(),
        "Delayed payment base key should match"
    );
    assert_eq!(
        channel_keys.htlc_basepoint_secret.secret_bytes(),
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
    let expected_revocation_basepoint = PublicKey::from_secret_key(
        &channel_keys.secp_ctx,
        &channel_keys.revocation_basepoint_secret,
    );
    let expected_payment_basepoint = PublicKey::from_secret_key(
        &channel_keys.secp_ctx,
        &channel_keys.payment_basepoint_secret,
    );
    let expected_delayed_payment_basepoint = PublicKey::from_secret_key(
        &channel_keys.secp_ctx,
        &channel_keys.delayed_payment_basepoint_secret,
    );
    let expected_htlc_basepoint =
        PublicKey::from_secret_key(&channel_keys.secp_ctx, &channel_keys.htlc_basepoint_secret);

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
    txid_bytes.reverse();

    let input_txid = bitcoin::Txid::from_slice(&txid_bytes).unwrap();

    let input_vout = 0;
    let funding_amount_sat = 500000;

    let funding_tx_hex = "0200000001bef67e4e2fb9ddeeb3461973cd4c62abb35050b1add772995b820b584a4884890000000000ffffffff0120a1070000000000220020313220af947477a37bcbbf3bb5def854df44e93f8aaad1831ea13a7db215406a00000000";

    let tx = create_funding_transaction(
        input_txid,
        input_vout,
        funding_amount_sat,
        &local_funding_pubkey,
        &remote_funding_pubkey,
    );

    let tx_hex = hex::encode(bitcoin::consensus::serialize(&tx));

    assert_eq!(
        funding_tx_hex, tx_hex,
        "Funding transaction should match provided solution"
    );
}

#[test]
fn test_07_sign_transaction_input_sighash_all() {
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
    let signature = channel_keys.sign_transaction_input_sighash_all(
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

    let signature_solution = hex::decode("3044022060fbcd83321e2e409566aeb8032ceee9ac968906151238068f7b0cf9e10b4bd702201f73255bd8bfb895ec3e04fda22500e262d17377923a8783c191a290beac984701").unwrap();

    assert_eq!(
        signature, signature_solution,
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
        revocation_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        delayed_payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        htlc_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(),
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
        revocation_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        delayed_payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        htlc_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(),
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
    let basepoint_secret = SecretKey::from_slice(
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
    let derived_privkey = derive_private_key(&basepoint_secret, &per_commitment_point, &secp_ctx);

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

    let commitment_index = 42;

    let commitment_transaction_number_obscure_factor =
        get_commitment_transaction_number_obscure_factor(
            &initiator_payment_basepoint,
            &receiver_payment_basepoint,
        );

    let obscured_commitment_transaction_number =
        commitment_transaction_number_obscure_factor ^ commitment_index;

    // Upper 24 bits in locktime
    let expected_locktime_value =
        ((0x20 as u32) << 8 * 3) | ((obscured_commitment_transaction_number & 0xffffff) as u32);

    // Lower 24 bits in sequence
    let expected_sequence_value = Sequence(
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
        commitment_index,
        &initiator_payment_basepoint,
        &receiver_payment_basepoint,
    );

    // Extract values from transaction
    let actual_locktime_value = tx.lock_time.to_consensus_u32();
    let actual_sequence_value = tx.input[0].sequence;

    assert_eq!(
        actual_locktime_value, expected_locktime_value,
        "Obscured locktime number is incorrect"
    );

    assert_eq!(
        actual_sequence_value, expected_sequence_value,
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
    let fee = 1_0000;

    let outputs = create_commitment_transaction_outputs(
        to_local_value,
        to_remote_value,
        &commitment_keys,
        &remote_payment_basepoint,
        to_self_delay,
        dust_limit_satoshis,
        fee,
    );

    // Expected to_local script from BOLT 3 test vectors
    let expected_to_local_output_script =
        hex::decode("0020f50bac8895d89a8a4f1de0b87bf52383f4d853e4368db17467fa50e3798d6980")
            .unwrap();

    // Expected P2WPKH script from BOLT 3 test vectors
    // Format: OP_0 <20-byte-pubkey-hash>
    let expected_to_remote_output_script =
        hex::decode("0014cc1b07838e387deacd0e5232e1e8b49f4c29e484").unwrap();

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

    // Verify to_local
    let to_local_output = outputs
        .iter()
        .find(|o| o.value == to_local_value - fee)
        .unwrap();
    assert!(
        to_local_output.script.is_p2wsh(),
        "to_local should be P2WSH"
    );
    assert_eq!(
        hex::encode(to_local_output.script.clone()),
        hex::encode(expected_to_local_output_script.clone()),
        "to_local script should match BOLT 3 test vector"
    );

    // Verify to_remote
    let to_remote_output = outputs.iter().find(|o| o.value == to_remote_value).unwrap();

    assert_eq!(
        hex::encode(to_remote_output.script.clone()),
        hex::encode(expected_to_remote_output_script.clone()),
        "to_local script should match BOLT 3 test vector"
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
    // BOLT 3 commitment number is 42
    let commitment_number = 42;
    let to_self_delay = 144;
    let dust_limit_satoshis = 546;
    let feerate_per_kw = 0;

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

    let expected_tx_hex = "0200000001bef67e4e2fb9ddeeb3461973cd4c62abb35050b1add772995b820b584a488489000000000038b02b8002c0c62d0000000000160014cc1b07838e387deacd0e5232e1e8b49f4c29e484c0cf6a0000000000220020f50bac8895d89a8a4f1de0b87bf52383f4d853e4368db17467fa50e3798d69803e195220";

    assert_eq!(
        hex::encode(bitcoin::consensus::serialize(&tx)),
        expected_tx_hex,
        "TX hex should match provided solution"
    );
}

#[test]
fn test_21_finalize_holder_commitment() {
    let secp_ctx = Secp256k1::new();

    // BOLT 3 test vector secrets (remove trailing 01 from hex)
    let local_funding_privkey = SecretKey::from_slice(
        &hex::decode("30ff4956bbdd3222d44cc5e8a1261dab1e07957bdac5ae88fe3261ef321f3749").unwrap(),
    )
    .unwrap();

    // BOLT 3 basepoint secrets
    let local_payment_basepoint_secret = SecretKey::from_slice(
        &hex::decode("1111111111111111111111111111111111111111111111111111111111111111").unwrap(),
    )
    .unwrap();

    let local_delayed_payment_basepoint_secret = SecretKey::from_slice(
        &hex::decode("3333333333333333333333333333333333333333333333333333333333333333").unwrap(),
    )
    .unwrap();

    let local_htlc_basepoint_secret = SecretKey::from_slice(
        &hex::decode("1111111111111111111111111111111111111111111111111111111111111111").unwrap(),
    )
    .unwrap();

    let local_revocation_basepoint_secret = SecretKey::from_slice(
        &hex::decode("2222222222222222222222222222222222222222222222222222222222222222").unwrap(),
    )
    .unwrap();

    // BOLT 3 commitment seed (all zeros)
    let commitment_seed = [0x00u8; 32];

    // Build ChannelKeyManager
    let channel_keys = ChannelKeyManager {
        funding_key: local_funding_privkey,
        revocation_basepoint_secret: local_revocation_basepoint_secret,
        payment_basepoint_secret: local_payment_basepoint_secret,
        delayed_payment_basepoint_secret: local_delayed_payment_basepoint_secret,
        htlc_basepoint_secret: local_htlc_basepoint_secret,
        commitment_seed,
        secp_ctx: secp_ctx.clone(),
    };

    // BOLT 3 funding pubkeys
    let local_funding_pubkey = BitcoinPublicKey::new(
        PublicKey::from_slice(
            &hex::decode("023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb")
                .unwrap(),
        )
        .unwrap(),
    );

    let remote_funding_pubkey = BitcoinPublicKey::new(
        PublicKey::from_slice(
            &hex::decode("030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c1")
                .unwrap(),
        )
        .unwrap(),
    );

    // BOLT 3 payment basepoints (for obscured commitment number)
    let local_payment_basepoint = PublicKey::from_slice(
        &hex::decode("034f355bdcb7cc0af728ef3cceb9615d90684bb5b2ca5f859ab0f0b704075871aa").unwrap(),
    )
    .unwrap();

    let remote_payment_basepoint = PublicKey::from_slice(
        &hex::decode("032c0b7cf95324a07d05398b240174dc0c2be444d96b159aa6c7f7b1e668680991").unwrap(),
    )
    .unwrap();

    // Funding outpoint from BOLT 3
    let funding_txid =
        Txid::from_str("8984484a580b825b9972d7adb15050b3ab624ccd731946b3eeddb92f4e7ef6be").unwrap();
    let funding_outpoint = OutPoint {
        txid: funding_txid,
        vout: 0,
    };
    let funding_amount = 10_000_000;

    // Create funding script
    let funding_script = create_funding_script(&local_funding_pubkey, &remote_funding_pubkey);

    // BOLT 3 commitment number is 42
    let commitment_number = 42;

    // Derive per_commitment_point for this commitment
    let per_commitment_point = channel_keys.derive_per_commitment_point(commitment_number);

    // Use exact derived keys from BOLT 3 test vectors
    let commitment_keys = CommitmentKeys::from_keys(
        per_commitment_point,
        PublicKey::from_slice(
            &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19")
                .unwrap(),
        )
        .unwrap(), // revocation_key
        PublicKey::from_slice(
            &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c")
                .unwrap(),
        )
        .unwrap(), // local_delayed_payment_key
        PublicKey::from_slice(
            &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7")
                .unwrap(),
        )
        .unwrap(), // local_htlc_key
        PublicKey::from_slice(
            &hex::decode("0394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b")
                .unwrap(),
        )
        .unwrap(), // remote_htlc_key
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

    let local_sig_first = true;

    // Finalize the holder commitment
    let signed_tx = finalize_holder_commitment(
        channel_keys,
        unsigned_tx,
        0,
        &funding_script,
        funding_amount,
        remote_signature,
        local_sig_first,
    );

    // BOLT 3 expected complete transaction
    let expected_tx_hex = "02000000000101bef67e4e2fb9ddeeb3461973cd4c62abb35050b1add772995b820b584a488489000000000038b02b8002c0c62d0000000000160014cc1b07838e387deacd0e5232e1e8b49f4c29e48454a56a00000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e04004730440220616210b2cc4d3afb601013c373bbd8aac54febd9f15400379a8cb65ce7deca60022034236c010991beb7ff770510561ae8dc885b8d38d1947248c38f2ae05564714201483045022100c3127b33dcc741dd6b05b1e63cbd1a9a7d816f37af9b6756fa2376b056f032370220408b96279808fe57eb7e463710804cdf4f108388bc5cf722d8c848d2c7f9f3b001475221023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb21030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c152ae3e195220";

    // Serialize and compare
    let actual_tx_hex = hex::encode(bitcoin::consensus::serialize(&signed_tx));

    assert_eq!(
        actual_tx_hex, expected_tx_hex,
        "Finalized commitment transaction should match BOLT 3 test vector"
    );
}

#[test]
fn test_22_create_offered_htlc_script() {
    // BOLT 3 test vector keys
    let revocation_pubkey = PublicKey::from_slice(
        &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19").unwrap(),
    )
    .unwrap();

    let local_htlcpubkey = PublicKey::from_slice(
        &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7").unwrap(),
    )
    .unwrap();

    let remote_htlcpubkey = PublicKey::from_slice(
        &hex::decode("0394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b").unwrap(),
    )
    .unwrap();

    // BOLT 3 test vector: HTLC #2 uses payment_hash = SHA256(0x0202...02)
    let preimage = [0x02u8; 32];
    let payment_hash = Sha256::hash(&preimage).to_byte_array();

    // Create offered HTLC script
    let script = create_offered_htlc_script(
        &revocation_pubkey,
        &local_htlcpubkey,
        &remote_htlcpubkey,
        &payment_hash,
    );

    // Expected offered HTLC script from BOLT 3 test vectors
    let expected_script_hex = "76a91414011f7254d96b819c76986c277d115efce6f7b58763ac67210394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b7c820120876475527c21030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e752ae67a914b43e1b38138a41b37f7cd9a1d274bc63e3a9b5d188ac6868";

    assert_eq!(
        hex::encode(script.as_bytes()),
        expected_script_hex,
        "Offered HTLC script should match BOLT 3 test vector"
    );
}

#[test]
fn test_23_create_htlc_timeout_transaction() {
    let secp_ctx = Secp256k1::new();

    // BOLT 3 provides the derived local_privkey directly (remove trailing 01)
    let local_htlc_privkey = SecretKey::from_slice(
        &hex::decode("bb13b121cdc357cd2e608b0aea294afca36e2b34cf958e2e6451a2f274694491").unwrap(),
    )
    .unwrap();

    // Build ChannelKeyManager with the BOLT 3 derived HTLC key
    let channel_keys = ChannelKeyManager {
        funding_key: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        revocation_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        delayed_payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        htlc_basepoint_secret: local_htlc_privkey, // Use the BOLT 3 derived key directly
        commitment_seed: [0x01; 32],               // dummy
        secp_ctx: secp_ctx.clone(),
    };

    // BOLT 3 commitment txid (corrected - derived from the commitment tx hex)
    let commitment_txid =
        Txid::from_str("2b887d4c1c59cd605144a1e2f971d168437db453f841f2fefb2c164f28ff84ab").unwrap();

    // HTLC #2 is at output index 1 in the commitment transaction
    let htlc_outpoint = OutPoint {
        txid: commitment_txid,
        vout: 1,
    };

    // BOLT 3 HTLC #2 values
    let htlc_amount = 2000; // satoshis
    let cltv_expiry = 502;

    // BOLT 3 keys
    let revocation_pubkey = PublicKey::from_slice(
        &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19").unwrap(),
    )
    .unwrap();

    let local_delayed_pubkey = PublicKey::from_slice(
        &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c").unwrap(),
    )
    .unwrap();

    let local_htlc_pubkey = PublicKey::from_slice(
        &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7").unwrap(),
    )
    .unwrap();

    let remote_htlc_pubkey = PublicKey::from_slice(
        &hex::decode("0394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b").unwrap(),
    )
    .unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap(),
    )
    .unwrap();

    let commitment_keys = CommitmentKeys::from_keys(
        per_commitment_point,
        revocation_pubkey,
        local_delayed_pubkey,
        local_htlc_pubkey,
        remote_htlc_pubkey,
    );

    // Create the unsigned HTLC timeout transaction
    let to_self_delay = 144;
    let feerate_per_kw = 0;

    let unsigned_htlc_timeout_tx = create_htlc_timeout_transaction(
        htlc_outpoint,
        htlc_amount,
        cltv_expiry,
        &commitment_keys,
        to_self_delay,
        feerate_per_kw,
    );

    // BOLT 3 HTLC #2 offered script (payment_hash = SHA256(0x0202...02))
    let preimage = [0x02u8; 32];
    let payment_hash = Sha256::hash(&preimage).to_byte_array();

    let htlc_script = create_offered_htlc_script(
        &revocation_pubkey,
        &local_htlc_pubkey,
        &remote_htlc_pubkey,
        &payment_hash,
    );

    // BOLT 3 remote HTLC signature (with SIGHASH_ALL appended)
    let remote_htlc_signature = hex::decode(
        "30440220649fe8b20e67e46cbb0d09b4acea87dbec001b39b08dee7bdd0b1f03922a8640022037c462dff79df501cecfdb12ea7f4de91f99230bb544726f6e04527b1f89600401"
    ).unwrap();

    // BOLT 3 local HTLC signature (with SIGHASH_ALL appended)
    let local_htlc_signature = hex::decode(
        "3045022100803159dee7935dba4a1d36a61055ce8fd62caa528573cc221ae288515405a252022029c59e7cffce374fe860100a4a63787e105c3cf5156d40b12dd53ff55ac8cf3f01"
    ).unwrap();

    // Create witness for HTLC timeout: [0, remote_sig, local_sig, 0 (false), htlc_script]
    let htlc_timeout_witness = Witness::from_slice(&[
        &[][..],
        &remote_htlc_signature[..],
        &local_htlc_signature[..],
        &[][..],
        htlc_script.as_bytes(),
    ]);

    // Attach witness to create signed transaction
    let mut signed_htlc_timeout_tx = unsigned_htlc_timeout_tx;
    signed_htlc_timeout_tx.input[0].witness = htlc_timeout_witness;

    // BOLT 3 expected HTLC timeout transaction
    let expected_tx_hex = "02000000000101ab84ff284f162cfbfef241f853b47d4368d171f9e2a1445160cd591c4c7d882b01000000000000000001d0070000000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e05004730440220649fe8b20e67e46cbb0d09b4acea87dbec001b39b08dee7bdd0b1f03922a8640022037c462dff79df501cecfdb12ea7f4de91f99230bb544726f6e04527b1f89600401483045022100803159dee7935dba4a1d36a61055ce8fd62caa528573cc221ae288515405a252022029c59e7cffce374fe860100a4a63787e105c3cf5156d40b12dd53ff55ac8cf3f01008576a91414011f7254d96b819c76986c277d115efce6f7b58763ac67210394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b7c820120876475527c21030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e752ae67a914b43e1b38138a41b37f7cd9a1d274bc63e3a9b5d188ac6868f6010000";

    // Serialize signed transaction and compare
    let actual_tx_hex = hex::encode(bitcoin::consensus::serialize(&signed_htlc_timeout_tx));

    assert_eq!(
        actual_tx_hex, expected_tx_hex,
        "Finalized HTLC timeout transaction should match BOLT 3 test vector"
    );
}

#[test]
fn test_24_finalize_htlc_timeout() {
    let secp_ctx = Secp256k1::new();

    // BOLT 3 provides the derived local_privkey directly (remove trailing 01)
    let local_htlc_privkey = SecretKey::from_slice(
        &hex::decode("bb13b121cdc357cd2e608b0aea294afca36e2b34cf958e2e6451a2f274694491").unwrap(),
    )
    .unwrap();

    // Build ChannelKeyManager with the BOLT 3 derived HTLC key
    let channel_keys = ChannelKeyManager {
        funding_key: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        revocation_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        delayed_payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        htlc_basepoint_secret: local_htlc_privkey, // Use the BOLT 3 derived key directly
        commitment_seed: [0x01; 32],               // dummy
        secp_ctx: secp_ctx.clone(),
    };

    // BOLT 3 commitment txid (corrected - derived from the commitment tx hex)
    let commitment_txid =
        Txid::from_str("2b887d4c1c59cd605144a1e2f971d168437db453f841f2fefb2c164f28ff84ab").unwrap();

    // HTLC #2 is at output index 1 in the commitment transaction
    let htlc_outpoint = OutPoint {
        txid: commitment_txid,
        vout: 1,
    };

    // BOLT 3 HTLC #2 values
    let htlc_amount = 2000; // satoshis
    let cltv_expiry = 502;

    // BOLT 3 keys
    let revocation_pubkey = PublicKey::from_slice(
        &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19").unwrap(),
    )
    .unwrap();

    let local_delayed_pubkey = PublicKey::from_slice(
        &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c").unwrap(),
    )
    .unwrap();

    let local_htlc_pubkey = PublicKey::from_slice(
        &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7").unwrap(),
    )
    .unwrap();

    let remote_htlc_pubkey = PublicKey::from_slice(
        &hex::decode("0394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b").unwrap(),
    )
    .unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap(),
    )
    .unwrap();

    let commitment_keys = CommitmentKeys::from_keys(
        per_commitment_point,
        revocation_pubkey,
        local_delayed_pubkey,
        local_htlc_pubkey,
        remote_htlc_pubkey,
    );

    // Create the unsigned HTLC timeout transaction
    let to_self_delay = 144;
    let feerate_per_kw = 0;

    let unsigned_htlc_timeout_tx = create_htlc_timeout_transaction(
        htlc_outpoint,
        htlc_amount,
        cltv_expiry,
        &commitment_keys,
        to_self_delay,
        feerate_per_kw,
    );

    // BOLT 3 HTLC #2 offered script (payment_hash = SHA256(0x0202...02))
    let preimage = [0x02u8; 32];
    let payment_hash = Sha256::hash(&preimage).to_byte_array();

    let htlc_script = create_offered_htlc_script(
        &revocation_pubkey,
        &local_htlc_pubkey,
        &remote_htlc_pubkey,
        &payment_hash,
    );

    // BOLT 3 remote HTLC signature (with SIGHASH_ALL appended)
    let remote_htlc_signature = hex::decode(
        "30440220649fe8b20e67e46cbb0d09b4acea87dbec001b39b08dee7bdd0b1f03922a8640022037c462dff79df501cecfdb12ea7f4de91f99230bb544726f6e04527b1f89600401"
    ).unwrap();

    // Finalize the HTLC timeout transaction
    let signed_htlc_timeout_tx = finalize_htlc_timeout(
        channel_keys,
        unsigned_htlc_timeout_tx,
        0,
        &htlc_script,
        htlc_amount,
        remote_htlc_signature,
        local_htlc_privkey,
    );

    // BOLT 3 expected HTLC timeout transaction
    let expected_tx_hex = "02000000000101ab84ff284f162cfbfef241f853b47d4368d171f9e2a1445160cd591c4c7d882b01000000000000000001d0070000000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e05004730440220649fe8b20e67e46cbb0d09b4acea87dbec001b39b08dee7bdd0b1f03922a8640022037c462dff79df501cecfdb12ea7f4de91f99230bb544726f6e04527b1f89600401483045022100803159dee7935dba4a1d36a61055ce8fd62caa528573cc221ae288515405a252022029c59e7cffce374fe860100a4a63787e105c3cf5156d40b12dd53ff55ac8cf3f01008576a91414011f7254d96b819c76986c277d115efce6f7b58763ac67210394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b7c820120876475527c21030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e752ae67a914b43e1b38138a41b37f7cd9a1d274bc63e3a9b5d188ac6868f6010000";

    // Serialize and compare
    let actual_tx_hex = hex::encode(bitcoin::consensus::serialize(&signed_htlc_timeout_tx));

    assert_eq!(
        actual_tx_hex, expected_tx_hex,
        "Finalized HTLC timeout transaction should match BOLT 3 test vector"
    );
}

#[test]
fn test_25_create_received_htlc_script() {
    // BOLT 3 test vector keys
    let revocation_pubkey = PublicKey::from_slice(
        &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19").unwrap(),
    )
    .unwrap();

    let local_htlcpubkey = PublicKey::from_slice(
        &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7").unwrap(),
    )
    .unwrap();

    let remote_htlcpubkey = PublicKey::from_slice(
        &hex::decode("0394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b").unwrap(),
    )
    .unwrap();

    // BOLT 3 test vector: HTLC #0 uses preimage = 0x0000...00
    let preimage = [0x00u8; 32];
    let payment_hash = Sha256::hash(&preimage).to_byte_array();
    let cltv_expiry = 500;

    // Create received HTLC script
    let script = create_received_htlc_script(
        &revocation_pubkey,
        &local_htlcpubkey,
        &remote_htlcpubkey,
        &payment_hash,
        cltv_expiry,
    );

    // Expected received HTLC script from BOLT 3 test vectors (HTLC #0)
    let expected_script_hex = "76a91414011f7254d96b819c76986c277d115efce6f7b58763ac67210394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b7c8201208763a914b8bcb07f6344b42ab04250c86a6e8b75d3fdbbc688527c21030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e752ae677502f401b175ac6868";

    assert_eq!(
        hex::encode(script.as_bytes()),
        expected_script_hex,
        "Received HTLC script should match BOLT 3 test vector"
    );
}

#[test]
fn test_26_create_htlc_success_transaction() {
    let secp_ctx = Secp256k1::new();

    // BOLT 3 provides the derived local_privkey directly (remove trailing 01)
    let local_htlc_privkey = SecretKey::from_slice(
        &hex::decode("bb13b121cdc357cd2e608b0aea294afca36e2b34cf958e2e6451a2f274694491").unwrap(),
    )
    .unwrap();

    // Build ChannelKeyManager with the BOLT 3 derived HTLC key
    let channel_keys = ChannelKeyManager {
        funding_key: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        revocation_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        delayed_payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        htlc_basepoint_secret: local_htlc_privkey,
        commitment_seed: [0x01; 32], // dummy
        secp_ctx: secp_ctx.clone(),
    };

    // BOLT 3 commitment txid
    let commitment_txid =
        Txid::from_str("2b887d4c1c59cd605144a1e2f971d168437db453f841f2fefb2c164f28ff84ab").unwrap();

    // HTLC #0 is at output index 0 in the commitment transaction
    let htlc_outpoint = OutPoint {
        txid: commitment_txid,
        vout: 0,
    };

    // BOLT 3 HTLC #0 values (received HTLC)
    let htlc_amount = 1000;
    let cltv_expiry = 500;

    // BOLT 3 keys
    let revocation_pubkey = PublicKey::from_slice(
        &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19").unwrap(),
    )
    .unwrap();

    let local_delayed_pubkey = PublicKey::from_slice(
        &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c").unwrap(),
    )
    .unwrap();

    let local_htlc_pubkey = PublicKey::from_slice(
        &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7").unwrap(),
    )
    .unwrap();

    let remote_htlc_pubkey = PublicKey::from_slice(
        &hex::decode("0394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b").unwrap(),
    )
    .unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap(),
    )
    .unwrap();

    let commitment_keys = CommitmentKeys::from_keys(
        per_commitment_point,
        revocation_pubkey,
        local_delayed_pubkey,
        local_htlc_pubkey,
        remote_htlc_pubkey,
    );

    // Create the unsigned HTLC success transaction
    let to_self_delay = 144;
    let feerate_per_kw = 0;

    let unsigned_htlc_success_tx = create_htlc_success_transaction(
        htlc_outpoint,
        htlc_amount,
        &commitment_keys,
        to_self_delay,
        feerate_per_kw,
    );

    // BOLT 3 HTLC #0 received script (preimage = 0x0000...00, cltv_expiry = 500)
    let payment_preimage = [0x00u8; 32];
    let payment_hash = Sha256::hash(&payment_preimage).to_byte_array();

    let htlc_script = create_received_htlc_script(
        &revocation_pubkey,
        &local_htlc_pubkey,
        &remote_htlc_pubkey,
        &payment_hash,
        cltv_expiry,
    );

    // BOLT 3 remote HTLC signature (with SIGHASH_ALL appended)
    let remote_htlc_signature = hex::decode(
        "3045022100d9e29616b8f3959f1d3d7f7ce893ffedcdc407717d0de8e37d808c91d3a7c50d022078c3033f6d00095c8720a4bc943c1b45727818c082e4e3ddbc6d3116435b624b01"
    ).unwrap();

    // BOLT 3 local HTLC signature (with SIGHASH_ALL appended)
    let local_htlc_signature = hex::decode(
        "30440220636de5682ef0c5b61f124ec74e8aa2461a69777521d6998295dcea36bc3338110220165285594b23c50b28b82df200234566628a27bcd17f7f14404bd865354eb3ce01"
    ).unwrap();

    // Create witness for HTLC success: [0, remote_sig, local_sig, payment_preimage, htlc_script]
    let htlc_success_witness = Witness::from_slice(&[
        &[][..],
        &remote_htlc_signature[..],
        &local_htlc_signature[..],
        &payment_preimage[..],
        htlc_script.as_bytes(),
    ]);

    // Attach witness to create signed transaction
    let mut signed_htlc_success_tx = unsigned_htlc_success_tx;
    signed_htlc_success_tx.input[0].witness = htlc_success_witness;

    // BOLT 3 expected HTLC success transaction
    let expected_tx_hex = "02000000000101ab84ff284f162cfbfef241f853b47d4368d171f9e2a1445160cd591c4c7d882b00000000000000000001e8030000000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e0500483045022100d9e29616b8f3959f1d3d7f7ce893ffedcdc407717d0de8e37d808c91d3a7c50d022078c3033f6d00095c8720a4bc943c1b45727818c082e4e3ddbc6d3116435b624b014730440220636de5682ef0c5b61f124ec74e8aa2461a69777521d6998295dcea36bc3338110220165285594b23c50b28b82df200234566628a27bcd17f7f14404bd865354eb3ce012000000000000000000000000000000000000000000000000000000000000000008a76a91414011f7254d96b819c76986c277d115efce6f7b58763ac67210394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b7c8201208763a914b8bcb07f6344b42ab04250c86a6e8b75d3fdbbc688527c21030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e752ae677502f401b175ac686800000000";

    // Serialize signed transaction and compare
    let actual_tx_hex = hex::encode(bitcoin::consensus::serialize(&signed_htlc_success_tx));

    assert_eq!(
        actual_tx_hex, expected_tx_hex,
        "Finalized HTLC success transaction should match BOLT 3 test vector"
    );
}

#[test]
fn test_27_finalize_htlc_success() {
    let secp_ctx = Secp256k1::new();

    // BOLT 3 provides the derived local_privkey directly (remove trailing 01)
    let local_htlc_privkey = SecretKey::from_slice(
        &hex::decode("bb13b121cdc357cd2e608b0aea294afca36e2b34cf958e2e6451a2f274694491").unwrap(),
    )
    .unwrap();

    // Build ChannelKeyManager with the BOLT 3 derived HTLC key
    let channel_keys = ChannelKeyManager {
        funding_key: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        revocation_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        delayed_payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(), // dummy
        htlc_basepoint_secret: local_htlc_privkey,
        commitment_seed: [0x01; 32], // dummy
        secp_ctx: secp_ctx.clone(),
    };

    // BOLT 3 commitment txid
    let commitment_txid =
        Txid::from_str("2b887d4c1c59cd605144a1e2f971d168437db453f841f2fefb2c164f28ff84ab").unwrap();

    // HTLC #0 is at output index 0 in the commitment transaction
    let htlc_outpoint = OutPoint {
        txid: commitment_txid,
        vout: 0,
    };

    // BOLT 3 HTLC #0 values (received HTLC)
    let htlc_amount = 1000;
    let cltv_expiry = 500;

    // BOLT 3 keys
    let revocation_pubkey = PublicKey::from_slice(
        &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19").unwrap(),
    )
    .unwrap();

    let local_delayed_pubkey = PublicKey::from_slice(
        &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c").unwrap(),
    )
    .unwrap();

    let local_htlc_pubkey = PublicKey::from_slice(
        &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7").unwrap(),
    )
    .unwrap();

    let remote_htlc_pubkey = PublicKey::from_slice(
        &hex::decode("0394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b").unwrap(),
    )
    .unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap(),
    )
    .unwrap();

    let commitment_keys = CommitmentKeys::from_keys(
        per_commitment_point,
        revocation_pubkey,
        local_delayed_pubkey,
        local_htlc_pubkey,
        remote_htlc_pubkey,
    );

    // Create the unsigned HTLC success transaction
    let to_self_delay = 144;
    let feerate_per_kw = 0;

    let unsigned_htlc_success_tx = create_htlc_success_transaction(
        htlc_outpoint,
        htlc_amount,
        &commitment_keys,
        to_self_delay,
        feerate_per_kw,
    );

    // BOLT 3 HTLC #0 received script (preimage = 0x0000...00, cltv_expiry = 500)
    let payment_preimage = [0x00u8; 32];
    let payment_hash = Sha256::hash(&payment_preimage).to_byte_array();

    let htlc_script = create_received_htlc_script(
        &revocation_pubkey,
        &local_htlc_pubkey,
        &remote_htlc_pubkey,
        &payment_hash,
        cltv_expiry,
    );

    // BOLT 3 remote HTLC signature (with SIGHASH_ALL appended)
    let remote_htlc_signature = hex::decode(
        "3045022100d9e29616b8f3959f1d3d7f7ce893ffedcdc407717d0de8e37d808c91d3a7c50d022078c3033f6d00095c8720a4bc943c1b45727818c082e4e3ddbc6d3116435b624b01"
    ).unwrap();

    // Finalize the HTLC success transaction
    let signed_htlc_success_tx = finalize_htlc_success(
        channel_keys,
        unsigned_htlc_success_tx,
        0,
        &htlc_script,
        htlc_amount,
        remote_htlc_signature,
        local_htlc_privkey,
        payment_preimage,
    );

    // BOLT 3 expected HTLC success transaction
    let expected_tx_hex = "02000000000101ab84ff284f162cfbfef241f853b47d4368d171f9e2a1445160cd591c4c7d882b00000000000000000001e8030000000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e0500483045022100d9e29616b8f3959f1d3d7f7ce893ffedcdc407717d0de8e37d808c91d3a7c50d022078c3033f6d00095c8720a4bc943c1b45727818c082e4e3ddbc6d3116435b624b014730440220636de5682ef0c5b61f124ec74e8aa2461a69777521d6998295dcea36bc3338110220165285594b23c50b28b82df200234566628a27bcd17f7f14404bd865354eb3ce012000000000000000000000000000000000000000000000000000000000000000008a76a91414011f7254d96b819c76986c277d115efce6f7b58763ac67210394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b7c8201208763a914b8bcb07f6344b42ab04250c86a6e8b75d3fdbbc688527c21030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e752ae677502f401b175ac686800000000";

    // Serialize and compare
    let actual_tx_hex = hex::encode(bitcoin::consensus::serialize(&signed_htlc_success_tx));

    assert_eq!(
        actual_tx_hex, expected_tx_hex,
        "Finalized HTLC success transaction should match BOLT 3 test vector"
    );
}

#[test]
fn test_28_create_htlc_outputs() {
    let secp_ctx = Secp256k1::new();

    // BOLT 3 test vector keys
    let revocation_pubkey = PublicKey::from_slice(
        &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19").unwrap(),
    )
    .unwrap();

    let local_delayed_pubkey = PublicKey::from_slice(
        &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c").unwrap(),
    )
    .unwrap();

    let local_htlc_pubkey = PublicKey::from_slice(
        &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7").unwrap(),
    )
    .unwrap();

    let remote_htlc_pubkey = PublicKey::from_slice(
        &hex::decode("0394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b").unwrap(),
    )
    .unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap(),
    )
    .unwrap();

    let commitment_keys = CommitmentKeys::from_keys(
        per_commitment_point,
        revocation_pubkey,
        local_delayed_pubkey,
        local_htlc_pubkey,
        remote_htlc_pubkey,
    );

    // BOLT 3 test vector HTLCs
    // Offered HTLCs: #2 (2000 sats, cltv 502) and #3 (3000 sats, cltv 503)
    let offered_htlcs = vec![
        HTLCOutput {
            amount_sat: 2000,
            payment_hash: Sha256::hash(&[0x02u8; 32]).to_byte_array(),
            cltv_expiry: 502,
        },
        HTLCOutput {
            amount_sat: 3000,
            payment_hash: Sha256::hash(&[0x03u8; 32]).to_byte_array(),
            cltv_expiry: 503,
        },
    ];

    // Received HTLCs: #0 (1000 sats, cltv 500), #1 (2000 sats, cltv 501), #4 (4000 sats, cltv 504)
    let received_htlcs = vec![
        HTLCOutput {
            amount_sat: 1000,
            payment_hash: Sha256::hash(&[0x00u8; 32]).to_byte_array(),
            cltv_expiry: 500,
        },
        HTLCOutput {
            amount_sat: 2000,
            payment_hash: Sha256::hash(&[0x01u8; 32]).to_byte_array(),
            cltv_expiry: 501,
        },
        HTLCOutput {
            amount_sat: 4000,
            payment_hash: Sha256::hash(&[0x04u8; 32]).to_byte_array(),
            cltv_expiry: 504,
        },
    ];

    // Create HTLC outputs
    let outputs = create_htlc_outputs(&commitment_keys, &offered_htlcs, &received_htlcs);

    // Should have 5 outputs total (2 offered + 3 received)
    assert_eq!(outputs.len(), 5, "Should have 5 HTLC outputs");

    // Verify offered HTLCs (first 2 outputs, with cltv_expiry for sorting)
    assert_eq!(
        outputs[0].value, 2000,
        "First offered HTLC should be 2000 sats"
    );
    assert_eq!(
        outputs[0].cltv_expiry,
        Some(502),
        "Offered HTLC #2 should have cltv_expiry 502"
    );
    assert!(outputs[0].script.is_p2wsh(), "Output should be P2WSH");

    assert_eq!(
        outputs[1].value, 3000,
        "Second offered HTLC should be 3000 sats"
    );
    assert_eq!(
        outputs[1].cltv_expiry,
        Some(503),
        "Offered HTLC #3 should have cltv_expiry 503"
    );

    // Verify received HTLCs (next 3 outputs, with cltv_expiry set)
    assert_eq!(
        outputs[2].value, 1000,
        "First received HTLC should be 1000 sats"
    );
    assert_eq!(
        outputs[2].cltv_expiry,
        Some(500),
        "Received HTLC should have cltv_expiry 500"
    );
    assert!(outputs[2].script.is_p2wsh(), "Output should be P2WSH");

    assert_eq!(
        outputs[3].value, 2000,
        "Second received HTLC should be 2000 sats"
    );
    assert_eq!(
        outputs[3].cltv_expiry,
        Some(501),
        "Received HTLC should have cltv_expiry 501"
    );

    assert_eq!(
        outputs[4].value, 4000,
        "Third received HTLC should be 4000 sats"
    );
    assert_eq!(
        outputs[4].cltv_expiry,
        Some(504),
        "Received HTLC should have cltv_expiry 504"
    );

    // Verify the P2WSH scripts match BOLT 3 test vectors
    // HTLC #2 offered script P2WSH
    let expected_htlc2_p2wsh =
        hex::decode("0020403d394747cae42e98ff01734ad5c08f82ba123d3d9a620abda88989651e2ab5")
            .unwrap();
    assert_eq!(
        outputs[0].script.as_bytes(),
        expected_htlc2_p2wsh.as_slice(),
        "HTLC #2 P2WSH should match BOLT 3"
    );

    // HTLC #3 offered script P2WSH
    let expected_htlc3_p2wsh =
        hex::decode("0020c20b5d1f8584fd90443e7b7b720136174fa4b9333c261d04dbbd012635c0f419")
            .unwrap();
    assert_eq!(
        outputs[1].script.as_bytes(),
        expected_htlc3_p2wsh.as_slice(),
        "HTLC #3 P2WSH should match BOLT 3"
    );

    // HTLC #0 received script P2WSH
    let expected_htlc0_p2wsh =
        hex::decode("002052bfef0479d7b293c27e0f1eb294bea154c63a3294ef092c19af51409bce0e2a")
            .unwrap();
    assert_eq!(
        outputs[2].script.as_bytes(),
        expected_htlc0_p2wsh.as_slice(),
        "HTLC #0 P2WSH should match BOLT 3"
    );

    // HTLC #1 received script P2WSH
    let expected_htlc1_p2wsh =
        hex::decode("0020748eba944fedc8827f6b06bc44678f93c0f9e6078b35c6331ed31e75f8ce0c2d")
            .unwrap();
    assert_eq!(
        outputs[3].script.as_bytes(),
        expected_htlc1_p2wsh.as_slice(),
        "HTLC #1 P2WSH should match BOLT 3"
    );

    // HTLC #4 received script P2WSH
    let expected_htlc4_p2wsh =
        hex::decode("00208c48d15160397c9731df9bc3b236656efb6665fbfe92b4a6878e88a499f741c4")
            .unwrap();
    assert_eq!(
        outputs[4].script.as_bytes(),
        expected_htlc4_p2wsh.as_slice(),
        "HTLC #4 P2WSH should match BOLT 3"
    );
}

#[test]
fn test_29_create_commitment_transaction_with_htlcs() {
    let secp_ctx = Secp256k1::new();

    // BOLT 3 local funding private key (remove trailing sighash all 01)
    let local_funding_privkey = SecretKey::from_slice(
        &hex::decode("30ff4956bbdd3222d44cc5e8a1261dab1e07957bdac5ae88fe3261ef321f3749").unwrap(),
    )
    .unwrap();

    // Build ChannelKeyManager with BOLT 3 funding key
    let channel_keys = ChannelKeyManager {
        funding_key: local_funding_privkey,
        revocation_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        delayed_payment_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        htlc_basepoint_secret: SecretKey::from_slice(&[0x01; 32]).unwrap(),
        commitment_seed: [0x00; 32],
        secp_ctx: secp_ctx.clone(),
    };

    // BOLT 3 test vector keys
    let revocation_pubkey = PublicKey::from_slice(
        &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19").unwrap(),
    )
    .unwrap();

    let local_delayed_pubkey = PublicKey::from_slice(
        &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c").unwrap(),
    )
    .unwrap();

    let local_htlc_pubkey = PublicKey::from_slice(
        &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7").unwrap(),
    )
    .unwrap();

    let remote_htlc_pubkey = PublicKey::from_slice(
        &hex::decode("0394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b").unwrap(),
    )
    .unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap(),
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

    let commitment_keys = CommitmentKeys::from_keys(
        per_commitment_point,
        revocation_pubkey,
        local_delayed_pubkey,
        local_htlc_pubkey,
        remote_htlc_pubkey,
    );

    // BOLT 3 funding outpoint
    let funding_txid =
        Txid::from_str("8984484a580b825b9972d7adb15050b3ab624ccd731946b3eeddb92f4e7ef6be").unwrap();
    let funding_outpoint = OutPoint {
        txid: funding_txid,
        vout: 0,
    };

    // BOLT 3 test vector: "commitment tx with all five HTLCs untrimmed (minimum feerate)"
    let to_local_sat = 6_988_000;
    let to_remote_sat = 3_000_000;

    // BOLT 3 commitment number is 42
    let commitment_number = 42;

    let to_self_delay = 144;
    let dust_limit_satoshis = 546;
    let feerate_per_kw = 0;

    // BOLT 3 HTLCs
    let offered_htlcs = vec![
        HTLCOutput {
            amount_sat: 2000,
            payment_hash: Sha256::hash(&[0x02u8; 32]).to_byte_array(),
            cltv_expiry: 502,
        },
        HTLCOutput {
            amount_sat: 3000,
            payment_hash: Sha256::hash(&[0x03u8; 32]).to_byte_array(),
            cltv_expiry: 503,
        },
    ];

    let received_htlcs = vec![
        HTLCOutput {
            amount_sat: 1000,
            payment_hash: Sha256::hash(&[0x00u8; 32]).to_byte_array(),
            cltv_expiry: 500,
        },
        HTLCOutput {
            amount_sat: 2000,
            payment_hash: Sha256::hash(&[0x01u8; 32]).to_byte_array(),
            cltv_expiry: 501,
        },
        HTLCOutput {
            amount_sat: 4000,
            payment_hash: Sha256::hash(&[0x04u8; 32]).to_byte_array(),
            cltv_expiry: 504,
        },
    ];

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
        &offered_htlcs,
        &received_htlcs,
    );

    // BOLT 3 funding script
    let funding_script = ScriptBuf::from_hex("5221023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb21030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c152ae").unwrap();
    let funding_amount = 10_000_000;

    // BOLT 3 remote signature (with SIGHASH_ALL appended)
    let remote_sig = hex::decode(
        "3044022009b048187705a8cbc9ad73adbe5af148c3d012e1f067961486c822c7af08158c022006d66f3704cfab3eb2dc49dae24e4aa22a6910fc9b424007583204e3621af2e501"
    ).unwrap();

    let local_sig_first = true;

    // Sign and finalize
    let signed_tx = finalize_holder_commitment(
        channel_keys,
        unsigned_tx,
        0,
        &funding_script,
        funding_amount,
        remote_sig,
        local_sig_first,
    );

    // BOLT 3 expected
    let expected_tx_hex = "02000000000101bef67e4e2fb9ddeeb3461973cd4c62abb35050b1add772995b820b584a488489000000000038b02b8007e80300000000000022002052bfef0479d7b293c27e0f1eb294bea154c63a3294ef092c19af51409bce0e2ad007000000000000220020403d394747cae42e98ff01734ad5c08f82ba123d3d9a620abda88989651e2ab5d007000000000000220020748eba944fedc8827f6b06bc44678f93c0f9e6078b35c6331ed31e75f8ce0c2db80b000000000000220020c20b5d1f8584fd90443e7b7b720136174fa4b9333c261d04dbbd012635c0f419a00f0000000000002200208c48d15160397c9731df9bc3b236656efb6665fbfe92b4a6878e88a499f741c4c0c62d0000000000160014cc1b07838e387deacd0e5232e1e8b49f4c29e484e0a06a00000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e040047304402206fc2d1f10ea59951eefac0b4b7c396a3c3d87b71ff0b019796ef4535beaf36f902201765b0181e514d04f4c8ad75659d7037be26cdb3f8bb6f78fe61decef484c3ea01473044022009b048187705a8cbc9ad73adbe5af148c3d012e1f067961486c822c7af08158c022006d66f3704cfab3eb2dc49dae24e4aa22a6910fc9b424007583204e3621af2e501475221023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb21030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c152ae3e195220";

    let actual_tx_hex = hex::encode(bitcoin::consensus::serialize(&signed_tx));

    assert_eq!(
        actual_tx_hex, expected_tx_hex,
        "Signed commitment transaction with 5 HTLCs should match BOLT 3"
    );
}
