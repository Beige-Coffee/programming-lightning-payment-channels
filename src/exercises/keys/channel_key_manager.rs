use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::HashEngine;
use bitcoin::hashes::{sha256, Hash};
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{All, Message, PublicKey, Secp256k1, SecretKey};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::{Amount, Transaction};

use crate::keys::commitment::{
    derive_private_key, derive_public_key, derive_revocation_private_key,
    derive_revocation_public_key,
};
use crate::types::{ChannelKeyManager, ChannelPublicKeys, CommitmentKeys};

// ============================================================================
// CHANNEL KEY MANAGER - UNIFIED IMPLEMENTATION
// ============================================================================
//
// This file consolidates all ChannelKeyManager methods into a single location.
// Previously, these methods were scattered across multiple files:
// - Signing operations were in keys/sign.rs
// - Key derivation would logically be in keys/commitment.rs
// - References were in keys/derivation.rs
//
// Now everything is in one place for better organization and maintainability.
//
// The ChannelKeyManager provides three main categories of functionality:
// 1. Construction and conversion (new, to_public_keys)
// 2. Per-commitment derivation (build_commitment_secret, derive_per_commitment_point, get_commitment_keys)
// 3. Transaction signing (sign_transaction_input, verify_signature)

impl ChannelKeyManager {
    pub fn to_public_keys(&self) -> ChannelPublicKeys {
        ChannelPublicKeys {
            funding_pubkey: PublicKey::from_secret_key(&self.secp_ctx, &self.funding_key),
            revocation_basepoint: PublicKey::from_secret_key(
                &self.secp_ctx,
                &self.revocation_base_key,
            ),
            payment_basepoint: PublicKey::from_secret_key(&self.secp_ctx, &self.payment_base_key),
            delayed_payment_basepoint: PublicKey::from_secret_key(
                &self.secp_ctx,
                &self.delayed_payment_base_key,
            ),
            htlc_basepoint: PublicKey::from_secret_key(&self.secp_ctx, &self.htlc_base_key),
        }
    }

    // ========================================================================
    // PER-COMMITMENT KEY DERIVATION
    // ========================================================================

    /// Exercise 7: Generate per-commitment secret from commitment seed
    ///
    /// Uses SHA256 to derive a unique secret for each commitment number.
    /// The commitment_seed is combined with the index to produce deterministic
    /// but unpredictable secrets for each state.
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

    /// Exercise 8: Derive per-commitment point from commitment secret
    ///
    /// The per-commitment point is the public key corresponding to the
    /// per-commitment secret. This is shared with the counterparty and
    /// used to derive all the commitment-specific keys.
    pub fn derive_per_commitment_point(&self, commitment_number: u64) -> PublicKey {
        let secret = self.build_commitment_secret(commitment_number);
        let secret_key = SecretKey::from_slice(&secret).expect("Valid secret");
        PublicKey::from_secret_key(&self.secp_ctx, &secret_key)
    }

    /// Exercise 13: Derive all commitment keys from channel base keys
    ///
    /// PRODUCTION PATH: This is the typical workflow in production.
    ///
    /// Takes the basepoints for both parties and derives all the keys needed
    /// for a specific commitment transaction. This is the function you'd use
    /// in production to build commitment transactions.
    ///
    /// Returns a CommitmentKeys struct containing:
    /// - per_commitment_point: The point used for derivation
    /// - revocation_key: Used by remote to punish old state broadcasts
    /// - local_delayed_payment_key: Used for to_local output
    /// - local_htlc_key: Used for local HTLC operations
    /// - remote_htlc_key: Used for remote HTLC operations
    pub fn get_commitment_keys(
        &self,
        commitment_number: u64,
        remote_revocation_basepoint: &PublicKey,
        remote_htlc_basepoint: &PublicKey,
        local_htlc_basepoint: &PublicKey,
    ) -> CommitmentKeys {
        // Derive the per-commitment point for this state
        let per_commitment_point = self.derive_per_commitment_point(commitment_number);

        // Derive the revocation key (remote can use this to punish us)
        let revocation_key = derive_revocation_public_key(
            remote_revocation_basepoint,
            &per_commitment_point,
            &self.secp_ctx,
        );

        // Derive local delayed payment key (our to_local output)
        let local_delayed_payment_basepoint =
            PublicKey::from_secret_key(&self.secp_ctx, &self.delayed_payment_base_key);
        let local_delayed_payment_key = derive_public_key(
            &local_delayed_payment_basepoint,
            &per_commitment_point,
            &self.secp_ctx,
        );

        // Derive local HTLC key
        let local_htlc_key =
            derive_public_key(local_htlc_basepoint, &per_commitment_point, &self.secp_ctx);

        // Derive remote HTLC key
        let remote_htlc_key =
            derive_public_key(remote_htlc_basepoint, &per_commitment_point, &self.secp_ctx);

        CommitmentKeys {
            per_commitment_point,
            revocation_key,
            local_htlc_key,
            remote_htlc_key,
            local_delayed_payment_key,
        }
    }

    // ========================================================================
    // TRANSACTION SIGNING OPERATIONS
    // ========================================================================

    /// Exercise 30: Sign a transaction input
    ///
    /// This is the fundamental signing operation used by all witness construction.
    /// It computes the sighash for a given input and creates an ECDSA signature.
    ///
    /// Returns: DER-encoded signature with SIGHASH_ALL appended
    pub fn sign_transaction_input(
        &self,
        tx: &Transaction,
        input_index: usize,
        script: &ScriptBuf,
        amount: u64,
        secret_key: &SecretKey,
    ) -> Vec<u8> {
        let mut sighash_cache = SighashCache::new(tx);

        let sighash = sighash_cache
            .p2wsh_signature_hash(
                input_index,
                script,
                Amount::from_sat(amount),
                EcdsaSighashType::All,
            )
            .expect("Valid sighash");

        let msg = Message::from_digest(sighash.to_byte_array());
        let sig = self.secp_ctx.sign_ecdsa(&msg, secret_key);

        let mut sig_bytes = sig.serialize_der().to_vec();
        sig_bytes.push(EcdsaSighashType::All as u8);
        sig_bytes
    }

    /// Exercise 32: Verify a signature
    ///
    /// Verifies that a signature is valid for a given transaction input.
    /// This is useful for validating signatures received from counterparties.
    pub fn verify_signature(
        &self,
        tx: &Transaction,
        input_index: usize,
        script: &ScriptBuf,
        amount: u64,
        signature: &[u8],
        pubkey: &PublicKey,
    ) -> bool {
        let mut sighash_cache = SighashCache::new(tx);

        let sighash = sighash_cache
            .p2wsh_signature_hash(
                input_index,
                script,
                Amount::from_sat(amount),
                EcdsaSighashType::All,
            )
            .expect("Valid sighash");

        let msg = Message::from_digest(sighash.to_byte_array());

        // Remove sighash type byte
        let sig_slice = &signature[..signature.len() - 1];
        let sig = bitcoin::secp256k1::ecdsa::Signature::from_der(sig_slice)
            .expect("Valid signature");

        self.secp_ctx.verify_ecdsa(&msg, &sig, pubkey).is_ok()
    }
}
