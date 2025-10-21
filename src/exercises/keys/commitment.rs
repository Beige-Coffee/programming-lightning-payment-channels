use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey, All, Scalar};
use bitcoin::hashes::{Hash, sha256};
use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::HashEngine;

use crate::types::ChannelKeys;

// ============================================================================
// SECTION 2: PER-COMMITMENT KEY DERIVATION
// ============================================================================
// These exercises teach how to derive keys specific to each commitment
// transaction using per-commitment points and base keys.

/// Exercise 6: Build commitment secret from commitment number
impl ChannelKeys {
    pub fn build_commitment_secret(&self, commitment_number: u64) -> [u8; 32] {
        let mut res: [u8; 32] = self.commitment_seed.clone();
        for i in 0..48 {
            let bitpos = 47 - i;
            if commitment_number & (1 << bitpos) == (1 << bitpos) {
                res[bitpos / 8] ^= 1 << (bitpos & 7);
                res = Sha256::hash(&res).to_byte_array();
            }
        }
        res
    }
}

/// Exercise 7: Derive per-commitment point from commitment number
impl ChannelKeys {
    pub fn derive_per_commitment_point(&self, commitment_number: u64) -> PublicKey {
        let secret = self.build_commitment_secret(commitment_number);
        let secret_key = SecretKey::from_slice(&secret).expect("Valid secret");
        PublicKey::from_secret_key(&self.secp_ctx, &secret_key)
    }
}

/// Exercise 8: Derive public key from basepoint and per-commitment point
pub fn derive_public_key(
    basepoint: &PublicKey,
    per_commitment_point: &PublicKey,
    secp_ctx: &Secp256k1<All>,
) -> PublicKey {
    // pubkey = basepoint + SHA256(per_commitment_point || basepoint)
    let mut engine = Sha256::engine();
    engine.input(&per_commitment_point.serialize());
    engine.input(&basepoint.serialize());
    let res = Sha256::from_engine(engine);
    
	let hashkey = PublicKey::from_secret_key(
		&secp_ctx,
		&SecretKey::from_slice(res.as_byte_array())
			.expect("Hashes should always be valid keys unless SHA-256 is broken"),
	);

    basepoint.combine(&hashkey).expect("Addition only fails if the tweak is the inverse of the key. This is not possible when the tweak contains the hash of the key.")
}

/// Exercise 9: Derive private key from base secret and per-commitment point
pub fn derive_private_key(
    base_secret: &SecretKey,
    per_commitment_point: &PublicKey,
    secp_ctx: &Secp256k1<All>,
) -> SecretKey {
    // privkey = base_secret + SHA256(per_commitment_point || basepoint)
    let basepoint = PublicKey::from_secret_key(secp_ctx, base_secret);
    
    let mut engine = Sha256::engine();
    engine.input(&per_commitment_point.serialize());
    engine.input(&basepoint.serialize());
	let res = Sha256::from_engine(engine).to_byte_array();

	base_secret.clone().add_tweak(&Scalar::from_be_bytes(res).unwrap())
		.expect("Addition only fails if the tweak is the inverse of the key. This is not possible when the tweak contains the hash of the key.")
}

// ============================================================================
// REVOCATION KEY DERIVATION (Special Case)
// ============================================================================
// Revocation keys use a different derivation formula to allow the counterparty
// to punish us if we broadcast an old state.

/// Exercise 11: Derive revocation public key
pub fn derive_revocation_public_key(
    revocation_basepoint: &PublicKey,
    per_commitment_point: &PublicKey,
    secp_ctx: &Secp256k1<All>,
) -> PublicKey {
    // revocationpubkey = revocation_basepoint * SHA256(revocation_basepoint || per_commitment_point) +
    //                    per_commitment_point * SHA256(per_commitment_point || revocation_basepoint)
    
    // First component
    let mut engine1 = Sha256::engine();
    engine1.input(&revocation_basepoint.serialize());
    engine1.input(&per_commitment_point.serialize());
    let hash1 = Sha256::from_engine(engine1).to_byte_array();
    let scalar1 = Scalar::from_be_bytes(hash1).expect("Valid scalar");
    let component1 = revocation_basepoint.mul_tweak(secp_ctx, &scalar1).expect("Valid tweak");
    
    // Second component
    let mut engine2 = Sha256::engine();
    engine2.input(&per_commitment_point.serialize());
    engine2.input(&revocation_basepoint.serialize());
    let hash2 = Sha256::from_engine(engine2).to_byte_array();
    let scalar2 = Scalar::from_be_bytes(hash2).expect("Valid scalar");
    let component2 = per_commitment_point.mul_tweak(secp_ctx, &scalar2).expect("Valid tweak");
    
    // Combine
    component1.combine(&component2).expect("Valid combination")
}

/// Exercise 12: Derive revocation private key
pub fn derive_revocation_private_key(
    revocation_basepoint_secret: &SecretKey,
    per_commitment_secret: &SecretKey,
    secp_ctx: &Secp256k1<All>,
) -> SecretKey {
    let revocation_basepoint = PublicKey::from_secret_key(secp_ctx, revocation_basepoint_secret);
    let per_commitment_point = PublicKey::from_secret_key(secp_ctx, per_commitment_secret);
    
    // First component
    let mut engine1 = Sha256::engine();
    engine1.input(&revocation_basepoint.serialize());
    engine1.input(&per_commitment_point.serialize());
    let hash1 = Sha256::from_engine(engine1).to_byte_array();
    let scalar1 = Scalar::from_be_bytes(hash1).expect("Valid scalar");
    let key1 = revocation_basepoint_secret.mul_tweak(&scalar1).expect("Valid tweak");
    
    // Second component
    let mut engine2 = Sha256::engine();
    engine2.input(&per_commitment_point.serialize());
    engine2.input(&revocation_basepoint.serialize());
    let hash2 = Sha256::from_engine(engine2).to_byte_array();
    let scalar2 = Scalar::from_be_bytes(hash2).expect("Valid scalar");
    let key2 = per_commitment_secret.mul_tweak(&scalar2).expect("Valid tweak");
    
    // Combine
    let scalar_key2 = Scalar::from_be_bytes(key2.secret_bytes()).expect("Valid scalar");
    key1.add_tweak(&scalar_key2).expect("Valid addition")
}

// ============================================================================
// COMMITMENT KEYS STRUCTURE (Like LDK's TxCreationKeys)
// ============================================================================

/// The set of public keys which are used in the creation of one commitment transaction.
/// These are derived from the channel base keys and per-commitment point.
/// 
/// This structure is similar to LDK's TxCreationKeys and allows us to:
/// 1. Pre-derive all keys before building the transaction
/// 2. Pass exact keys from test vectors for testing
/// 3. Separate key derivation concerns from transaction building
#[derive(Clone, Debug)]
pub struct CommitmentKeys {
    /// The per-commitment point used to derive the other keys
    pub per_commitment_point: PublicKey,
    
    /// The revocation key which allows the broadcaster's counterparty to punish
    /// them if they broadcast an old state
    pub revocation_key: PublicKey,
    
    /// Local party's HTLC key (derived from local_htlc_basepoint)
    pub local_htlc_key: PublicKey,
    
    /// Remote party's HTLC key (derived from remote_htlc_basepoint)
    pub remote_htlc_key: PublicKey,
    
    /// Local party's delayed payment key (for to_local output)
    pub local_delayed_payment_key: PublicKey,
}

impl CommitmentKeys {
    /// Exercise 10: Derive all commitment keys from basepoints and per-commitment point
    /// 
    /// PRODUCTION PATH: Use this when you have basepoints and need to derive keys
    /// for a specific commitment transaction. This is the normal flow in production.
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
            secp_ctx
        );
        
        // Derive local delayed payment key
        let local_delayed_payment_key = derive_public_key(
            local_delayed_payment_basepoint,
            per_commitment_point,
            secp_ctx
        );
        
        // Derive local HTLC key
        let local_htlc_key = derive_public_key(
            local_htlc_basepoint,
            per_commitment_point,
            secp_ctx
        );
        
        // Derive remote HTLC key
        let remote_htlc_key = derive_public_key(
            remote_htlc_basepoint,
            per_commitment_point,
            secp_ctx
        );
        
        Self {
            per_commitment_point: *per_commitment_point,
            revocation_key,
            local_htlc_key,
            remote_htlc_key,
            local_delayed_payment_key,
        }
    }

    /// Create CommitmentKeys from a per-commitment point and a ChannelKeys object
    pub fn from_channel_keys(
        per_commitment_point: PublicKey,
        channel_keys: &ChannelKeys,
    ) -> Self {
        let local_htlc_basepoint = PublicKey::from_secret_key(&channel_keys.secp_ctx, &channel_keys.local_htlc_key);
        let remote_revocation_basepoint = PublicKey::from_secret_key(&channel_keys.secp_ctx, &channel_keys.remote_revocation_base_key);
        let remote_htlc_basepoint = PublicKey::from_secret_key(&channel_keys.secp_ctx, &channel_keys.remote_htlc_base_key);
        let local_delayed_payment_basepoint = PublicKey::from_secret_key(&channel_keys.secp_ctx, &channel_keys.delayed_payment_base_key);

        Self::from_basepoints(
            &per_commitment_point,
            &local_delayed_payment_basepoint,
            &local_htlc_basepoint,
            &remote_revocation_basepoint,
            &remote_htlc_basepoint,
            &channel_keys.secp_ctx,
        )
    }
    /// Create keys directly from provided public keys
    /// 
    /// TESTING PATH: Use this when you have exact keys from BOLT 3 test vectors.
    /// This allows you to inject specific keys without derivation to verify
    /// your transaction construction matches the specification.
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

/// Helper method to get commitment keys for a specific commitment number
/// 
/// This combines per-commitment point derivation (Exercise 7) with commitment key
/// derivation (Exercise 10) to get all keys needed for a commitment transaction.
/// This is a convenience method, not a separate exercise.
impl ChannelKeys {
    pub fn get_commitment_keys(
        &self,
        commitment_number: u64,
        local_revocation_pubkey: &PublicKey,
        remote_htlc_basepoint: &PublicKey,
        local_htlc_basepoint: &PublicKey,
    ) -> CommitmentKeys {
        let per_commitment_point = self.derive_per_commitment_point(commitment_number);
        
        // Convert each base key to public key
        let local_delayed_payment_basepoint = PublicKey::from_secret_key(&self.secp_ctx, &self.delayed_payment_base_key);
        
        CommitmentKeys::from_basepoints(
            &per_commitment_point,
            &local_delayed_payment_basepoint,
            &local_htlc_basepoint,
            local_revocation_pubkey,
            remote_htlc_basepoint,
            &self.secp_ctx,
        )
    }
}
