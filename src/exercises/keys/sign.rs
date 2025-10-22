use bitcoin::{Transaction, Amount};
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey, Message};
use bitcoin::sighash::{SighashCache, EcdsaSighashType};

use crate::types::InMemorySigner;

// ============================================================================
// INMEMORY SIGNER IMPLEMENTATION
// ============================================================================
//
// This implementation provides pure cryptographic signing operations.
// The InMemorySigner holds all private keys for a channel and provides
// methods to sign transaction inputs.
//
// Witness construction (combining signatures with scripts and other data)
// is handled in the respective transaction modules:
// - transactions/commitment.rs for commitment transaction witnesses
// - transactions/htlc.rs for HTLC transaction witnesses
//
// This separation follows LDK's architecture where:
// - Signing is a security boundary (handled here)
// - Witness construction is business logic (handled in transaction code)

impl InMemorySigner {
    /// Create a new InMemorySigner with the given keys
    pub fn new(
        funding_key: SecretKey,
        revocation_base_key: SecretKey,
        payment_base_key: SecretKey,
        delayed_payment_base_key: SecretKey,
        htlc_base_key: SecretKey,
        commitment_seed: [u8; 32],
    ) -> Self {
        Self {
            funding_key,
            revocation_base_key,
            payment_base_key,
            delayed_payment_base_key,
            htlc_base_key,
            commitment_seed,
            secp_ctx: Secp256k1::new(),
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
