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

/// Exercise 4: Derive all base public keys
impl ChannelKeyManager {
    pub fn to_public_keys(&self) -> ChannelPublicKeys {
        // Convert each private key to its corresponding public key
        // Return ChannelPublicKeys struct
        ChannelPublicKeys {
            funding_pubkey: PublicKey::from_secret_key(&self.secp_ctx, &self.funding_key),
            revocation_basepoint: PublicKey::from_secret_key(
                &self.secp_ctx,
                &self.revocation_basepoint_secret,
            ),
            payment_basepoint: PublicKey::from_secret_key(
                &self.secp_ctx,
                &self.payment_basepoint_secret,
            ),
            delayed_payment_basepoint: PublicKey::from_secret_key(
                &self.secp_ctx,
                &self.delayed_payment_basepoint_secret,
            ),
            htlc_basepoint: PublicKey::from_secret_key(&self.secp_ctx, &self.htlc_basepoint_secret),
        }
    }
}

/// Exercise 7: Sign transaction input with SIGHASH_ALL
impl ChannelKeyManager {
    pub fn sign_transaction_input_sighash_all(
        &self,
        tx: &Transaction,
        input_index: usize,
        script: &ScriptBuf,
        amount: u64,
        secret_key: &SecretKey,
    ) -> Vec<u8> {
        // Compute the sighash for the P2WSH input
        let mut sighash_cache = SighashCache::new(tx);

        // Compute the P2WSH signature hash
        let sighash = sighash_cache
            .p2wsh_signature_hash(
                input_index,
                script,
                Amount::from_sat(amount),
                EcdsaSighashType::All,
            )
            .expect("Valid sighash");

        // Convert sighash to Message
        let msg = Message::from_digest(sighash.to_byte_array());

        // Sign the Message with secret_key
        let sig = self.secp_ctx.sign_ecdsa(&msg, secret_key);

        // Serialize signature and append SIGHASH_ALL flag (EcdsaSighashType::All)
        let mut sig_bytes = sig.serialize_der().to_vec();
        sig_bytes.push(EcdsaSighashType::All as u8);
        sig_bytes
    }
}

impl ChannelKeyManager {
    /// Exercise 10: Build per-commitment secret
    pub fn build_commitment_secret(&self, commitment_number: u64) -> [u8; 32] {
        // Initialize P as clone of commitment seed
        let mut p: [u8; 32] = self.commitment_seed.clone();

        // Apply BOLT 3 derivation algorithm (flip bits and hash)
        for i in 0..48 {
            let bit_position = 47 - i;
            if commitment_number & (1 << bit_position) == (1 << bit_position) {
                p[bit_position / 8] ^= 1 << (bit_position & 7);
                p = Sha256::hash(&p).to_byte_array();
            }
        }
        p
    }

    /// Exercise 11: Derive per-commitment point
    pub fn derive_per_commitment_point(&self, commitment_number: u64) -> PublicKey {
        // Build the commitment secret for this commitment number
        let secret = self.build_commitment_secret(commitment_number);

        // Convert secret to SecretKey and then to PublicKey
        let secret_key = SecretKey::from_slice(&secret).expect("Valid secret");
        PublicKey::from_secret_key(&self.secp_ctx, &secret_key)
    }
}

impl ChannelKeyManager {
    // helper used for tests
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
            PublicKey::from_secret_key(&self.secp_ctx, &self.delayed_payment_basepoint_secret);
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
}
