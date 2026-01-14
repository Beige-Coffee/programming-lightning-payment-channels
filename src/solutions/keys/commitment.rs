use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::HashEngine;
use bitcoin::hashes::{sha256, Hash};
use bitcoin::secp256k1::{All, PublicKey, Scalar, Secp256k1, SecretKey};

use crate::types::CommitmentKeys;

/// Exercise 8: Derive revocation public key
pub fn derive_revocation_public_key(
    revocation_basepoint: &PublicKey,
    per_commitment_point: &PublicKey,
    secp_ctx: &Secp256k1<All>,
) -> PublicKey {

    // First component: revocation_basepoint * SHA256(revocation_basepoint || per_commitment_point)
    let mut engine1 = Sha256::engine();
    engine1.input(&revocation_basepoint.serialize());
    engine1.input(&per_commitment_point.serialize());
    let hash1 = Sha256::from_engine(engine1).to_byte_array();
    let scalar1 = Scalar::from_be_bytes(hash1).expect("Valid scalar");
    let component1 = revocation_basepoint
        .mul_tweak(secp_ctx, &scalar1)
        .expect("Valid tweak");

    // Second component: per_commitment_point * SHA256(per_commitment_point || revocation_basepoint)
    let mut engine2 = Sha256::engine();
    engine2.input(&per_commitment_point.serialize());
    engine2.input(&revocation_basepoint.serialize());
    let hash2 = Sha256::from_engine(engine2).to_byte_array();
    let scalar2 = Scalar::from_be_bytes(hash2).expect("Valid scalar");
    let component2 = per_commitment_point
        .mul_tweak(secp_ctx, &scalar2)
        .expect("Valid tweak");

    // Add the two components together
    component1.combine(&component2).expect("Valid combination")
}

/// Exercise 9: Derive revocation private key
pub fn derive_revocation_private_key(
    revocation_basepoint_secret: &SecretKey,
    per_commitment_secret: &SecretKey,
    secp_ctx: &Secp256k1<All>,
) -> SecretKey {

    // Convert private keys to public keys for hashing
    let revocation_basepoint = PublicKey::from_secret_key(secp_ctx, revocation_basepoint_secret);
    let per_commitment_point = PublicKey::from_secret_key(secp_ctx, per_commitment_secret);

    // First component: revocation_basepoint_secret * SHA256(revocation_basepoint || per_commitment_point)
    let mut engine1 = Sha256::engine();
    engine1.input(&revocation_basepoint.serialize());
    engine1.input(&per_commitment_point.serialize());
    let hash1 = Sha256::from_engine(engine1).to_byte_array();
    let scalar1 = Scalar::from_be_bytes(hash1).expect("Valid scalar");
    let key1 = revocation_basepoint_secret
        .mul_tweak(&scalar1)
        .expect("Valid tweak");

    // Second component: per_commitment_secret * SHA256(per_commitment_point || revocation_basepoint)
    let mut engine2 = Sha256::engine();
    engine2.input(&per_commitment_point.serialize());
    engine2.input(&revocation_basepoint.serialize());
    let hash2 = Sha256::from_engine(engine2).to_byte_array();
    let scalar2 = Scalar::from_be_bytes(hash2).expect("Valid scalar");
    let key2 = per_commitment_secret
        .mul_tweak(&scalar2)
        .expect("Valid tweak");

    // Add the two components together
    let scalar_key2 = Scalar::from_be_bytes(key2.secret_bytes()).expect("Valid scalar");
    key1.add_tweak(&scalar_key2).expect("Valid addition")
}

/// Exercise 12: Derive public key from basepoint and per-commitment point
pub fn derive_public_key(
    basepoint: &PublicKey,
    per_commitment_point: &PublicKey,
    secp_ctx: &Secp256k1<All>,
) -> PublicKey {

    // Create a SHA256 hash engine
    let mut engine = Sha256::engine();

    // Hash per_commitment_point and basepoint together
    engine.input(&per_commitment_point.serialize());
    engine.input(&basepoint.serialize());
    let res = Sha256::from_engine(engine);

    // Convert hash to a public key
    let hashkey = PublicKey::from_secret_key(
        &secp_ctx,
        &SecretKey::from_slice(res.as_byte_array())
            .expect("Hashes should always be valid keys unless SHA-256 is broken"),
    );

    // Add basepoint and hashkey: basepoint + SHA256(per_commitment_point || basepoint) * G
    basepoint.combine(&hashkey).expect("Addition only fails if the tweak is the inverse of the key. This is not possible when the tweak contains the hash of the key.")
}

/// Exercise 13: Derive private key from basepoint secret and per-commitment point
pub fn derive_private_key(
    basepoint_secret: &SecretKey,
    per_commitment_point: &PublicKey,
    secp_ctx: &Secp256k1<All>,
) -> SecretKey {

    // Convert basepoint secret to public key for hashing
    let basepoint = PublicKey::from_secret_key(secp_ctx, basepoint_secret);

    // Hash per_commitment_point and basepoint together
    let mut engine = Sha256::engine();
    engine.input(&per_commitment_point.serialize());
    engine.input(&basepoint.serialize());
    let res = Sha256::from_engine(engine).to_byte_array();

    // Add tweak to basepoint_secret: basepoint_secret + SHA256(per_commitment_point || basepoint)
    basepoint_secret.clone().add_tweak(&Scalar::from_be_bytes(res).unwrap())
		.expect("Addition only fails if the tweak is the inverse of the key. This is not possible when the tweak contains the hash of the key.")
}






/// helper (Not an exercise)
impl CommitmentKeys {
    pub fn from_basepoints(
        per_commitment_point: &PublicKey,
        local_delayed_payment_basepoint: &PublicKey,
        local_htlc_basepoint: &PublicKey,
        remote_revocation_basepoint: &PublicKey,
        remote_htlc_basepoint: &PublicKey,
        secp_ctx: &Secp256k1<All>,
    ) -> Self {
        // Derive revocation key (remote can revoke our commitment)
        let revocation_key = derive_revocation_public_key(
            remote_revocation_basepoint,
            per_commitment_point,
            secp_ctx,
        );

        // Derive local delayed payment key
        let local_delayed_payment_key = derive_public_key(
            local_delayed_payment_basepoint,
            per_commitment_point,
            secp_ctx,
        );

        // Derive local HTLC key
        let local_htlc_key =
            derive_public_key(local_htlc_basepoint, per_commitment_point, secp_ctx);

        // Derive remote HTLC key
        let remote_htlc_key =
            derive_public_key(remote_htlc_basepoint, per_commitment_point, secp_ctx);

        Self {
            per_commitment_point: *per_commitment_point,
            revocation_key,
            local_htlc_key,
            remote_htlc_key,
            local_delayed_payment_key,
        }
    }

    pub fn from_keys(
        per_commitment_point: PublicKey,
        revocation_key: PublicKey,
        local_delayed_payment_key: PublicKey,
        local_htlc_key: PublicKey,
        remote_htlc_key: PublicKey,
    ) -> Self {
        Self {
            per_commitment_point,
            revocation_key,
            local_delayed_payment_key,
            local_htlc_key,
            remote_htlc_key,
        }
    }
}
