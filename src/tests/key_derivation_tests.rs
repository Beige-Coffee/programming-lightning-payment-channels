use crate::*;
use bitcoin::hashes::{sha256, Hash, HashEngine};
use bitcoin::secp256k1::{PublicKey, Scalar, Secp256k1, SecretKey};

#[test]
fn test_derivation_of_local_public_key() {

    let basepoint_secret = SecretKey::from_slice(
        &hex::decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f").unwrap()[..32]
    ).unwrap();

    let per_commitment_secret = SecretKey::from_slice(
        &hex::decode("1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100").unwrap()[..32]
    ).unwrap();
    
    let base_point = PublicKey::from_slice(
        &hex::decode("036d6caac248af96f6afa7f904f550253a0f3ef3f5aa2fe6838a95b216691468e2").unwrap()
    ).unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap()
    ).unwrap();

    let expected_localpubkey = PublicKey::from_slice(
        &hex::decode("0235f2dbfaa89b57ec7b055afe29849ef7ddfeb1cefdb9ebdc43f5494984db29e5").unwrap()
    ).unwrap();

    let secp = Secp256k1::new();

    let actual_local_pubkey = derive_public_key(&base_point, &per_commitment_point, &secp);

    println!("Expected Local Public Key: {}", expected_localpubkey);
    println!("Actual Local Public Key: {}", actual_local_pubkey);

    assert_eq!(
        actual_local_pubkey,
        expected_localpubkey,
        "Local public keys do not match"
    );
}

#[test]
fn test_derivation_of_local_private_key() {

    let basepoint_secret = SecretKey::from_slice(
        &hex::decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f").unwrap()[..32]
    ).unwrap();

    let per_commitment_secret = SecretKey::from_slice(
        &hex::decode("1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100").unwrap()[..32]
    ).unwrap();
    
    let base_point = PublicKey::from_slice(
        &hex::decode("036d6caac248af96f6afa7f904f550253a0f3ef3f5aa2fe6838a95b216691468e2").unwrap()
    ).unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap()
    ).unwrap();

    let expected_localprivkey = SecretKey::from_slice(
        &hex::decode("cbced912d3b21bf196a766651e436aff192362621ce317704ea2f75d87e7be0f").unwrap()
    ).unwrap();

    let secp = Secp256k1::new();

    let actual_local_privkey = derive_private_key(&basepoint_secret, &per_commitment_point, &secp);

    println!("Expected Local Public Key: {:?}", expected_localprivkey);
    println!("Actual Local Public Key: {:?}", actual_local_privkey);

    assert_eq!(
        expected_localprivkey,
        actual_local_privkey,
        "Local private keys do not match"
    );
}

#[test]
fn test_derivation_of_revocation_pubkey() {

    let basepoint_secret = SecretKey::from_slice(
        &hex::decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f").unwrap()[..32]
    ).unwrap();

    let per_commitment_secret = SecretKey::from_slice(
        &hex::decode("1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100").unwrap()[..32]
    ).unwrap();
    
    let base_point = PublicKey::from_slice(
        &hex::decode("036d6caac248af96f6afa7f904f550253a0f3ef3f5aa2fe6838a95b216691468e2").unwrap()
    ).unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap()
    ).unwrap();

    let revocation_basepoint = PublicKey::from_slice(
        &hex::decode("036d6caac248af96f6afa7f904f550253a0f3ef3f5aa2fe6838a95b216691468e2").unwrap()
    ).unwrap();

    let expected_revocation_pubkey = PublicKey::from_slice(
        &hex::decode("02916e326636d19c33f13e8c0c3a03dd157f332f3e99c317c141dd865eb01f8ff0").unwrap()
    ).unwrap();

    let secp = Secp256k1::new();

    let actual_revocation_pubkey = derive_revocation_public_key(&revocation_basepoint, &per_commitment_point, &secp);

    println!("Expected Local Public Key: {:?}", expected_revocation_pubkey);
    println!("Actual Local Public Key: {:?}", actual_revocation_pubkey);

    assert_eq!(
        expected_revocation_pubkey,
        actual_revocation_pubkey,
        "Revocation public keys do not match"
    );
}

#[test]
fn test_derivation_of_revocation_privkey() {

    let basepoint_secret = SecretKey::from_slice(
        &hex::decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f").unwrap()[..32]
    ).unwrap();

    let per_commitment_secret = SecretKey::from_slice(
        &hex::decode("1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100").unwrap()[..32]
    ).unwrap();
    
    let base_point = PublicKey::from_slice(
        &hex::decode("036d6caac248af96f6afa7f904f550253a0f3ef3f5aa2fe6838a95b216691468e2").unwrap()
    ).unwrap();

    let per_commitment_point = PublicKey::from_slice(
        &hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486").unwrap()
    ).unwrap();

    let revocation_basepoint = PublicKey::from_slice(
        &hex::decode("036d6caac248af96f6afa7f904f550253a0f3ef3f5aa2fe6838a95b216691468e2").unwrap()
    ).unwrap();

    let revocation_basepoint_secret = SecretKey::from_slice(
        &hex::decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f").unwrap()
    ).unwrap();

    let expected_revocation_privkey = SecretKey::from_slice(
        &hex::decode("d09ffff62ddb2297ab000cc85bcb4283fdeb6aa052affbc9dddcf33b61078110").unwrap()
    ).unwrap();

    let secp = Secp256k1::new();

    let actual_revocation_privkey = derive_revocation_private_key(&revocation_basepoint_secret, &per_commitment_secret, &secp);

    println!("Expected Local Public Key: {:?}", expected_revocation_privkey);
    println!("Actual Local Public Key: {:?}", actual_revocation_privkey);

    assert_eq!(
        expected_revocation_privkey,
        actual_revocation_privkey,
        "Revocation private keys do not match"
    );
}