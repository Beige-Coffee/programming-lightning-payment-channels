use bitcoin::{Transaction, Amount, Witness};
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey, Message, All};
use bitcoin::sighash::{SighashCache, EcdsaSighashType};
use bitcoin::hashes::{Hash, sha256};

use crate::types::InMemorySigner;

// ============================================================================
// INMEMORY SIGNER IMPLEMENTATION
// ============================================================================
//
// This follows LDK's architecture where the InMemorySigner struct holds
// all private keys for a channel and implements signing methods directly
// on the struct itself, rather than as standalone functions.
//
// Benefits of this approach:
// - Encapsulation: Keys and signing logic are together
// - Object-oriented: More intuitive API (signer.sign_tx() vs sign_tx(&signer, ...))
// - Production-ready: Matches how real Lightning implementations work
// - Extensible: Easy to add new signing methods or implement traits

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
    // BASIC TRANSACTION SIGNING
    // ========================================================================

    /// Exercise 30: Sign a transaction input
    /// 
    /// This is the fundamental signing operation used by all other methods.
    /// It computes the sighash for a given input and creates an ECDSA signature.
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

    /// Exercise 31: Create witness for commitment transaction
    /// 
    /// In a real Lightning implementation:
    /// 1. You create the unsigned commitment transaction
    /// 2. You send it to your counterparty to get their signature
    /// 3. You sign it with your local funding key (using this method)
    /// 4. You combine both signatures to create the witness
    /// 
    /// This method signs with the signer's funding key and expects the remote signature
    /// as a parameter (which you would have received from your counterparty).
    pub fn create_commitment_witness(
        &self,
        tx: &Transaction,
        funding_script: &ScriptBuf,
        funding_amount: u64,
        remote_funding_signature: Vec<u8>,
    ) -> Witness {
        // Sign with our funding key
        let local_sig = self.sign_transaction_input(
            tx, 
            0, 
            funding_script, 
            funding_amount, 
            &self.funding_key,
        );
        
        // Build witness stack: [0, sig1, sig2, witnessScript]
        Witness::from_slice(&[
            &[][..],                      // OP_0 for CHECKMULTISIG bug
            &local_sig[..],
            &remote_funding_signature[..],
            funding_script.as_bytes(),
        ])
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

    // ========================================================================
    // HTLC TRANSACTION SIGNING
    // ========================================================================

    /// Sign an HTLC-success transaction with local key and create its witness
    /// 
    /// This function signs an HTLC-success transaction with the local HTLC key and
    /// combines it with the remote HTLC signature (received from counterparty) to
    /// construct the complete witness stack with the payment preimage.
    /// 
    /// In a real Lightning implementation, you would:
    /// 1. Create the unsigned HTLC transaction
    /// 2. Send it to your counterparty to get their signature
    /// 3. Sign it yourself with your local key
    /// 4. Combine both signatures to create the witness
    /// 
    /// Witness stack: [0, remote_htlc_sig, local_htlc_sig, payment_preimage, htlc_script]
    pub fn sign_htlc_success_transaction(
        &self,
        tx: &Transaction,
        htlc_script: &ScriptBuf,
        htlc_amount: u64,
        local_htlc_key: &SecretKey,
        remote_htlc_signature: Vec<u8>,
        payment_preimage: [u8; 32],
    ) -> Witness {
        // Sign with our local HTLC key
        let local_htlc_signature = self.sign_transaction_input(
            tx,
            0,
            htlc_script,
            htlc_amount,
            local_htlc_key,
        );

        // Create witness with both signatures and payment preimage
        self.create_htlc_success_witness(
            remote_htlc_signature,
            local_htlc_signature,
            payment_preimage,
            htlc_script,
        )
    }

    /// Sign an HTLC-timeout transaction with local key and create its witness
    /// 
    /// This function signs an HTLC-timeout transaction with the local HTLC key and
    /// combines it with the remote HTLC signature (received from counterparty) to
    /// construct the complete witness stack for the timeout path.
    /// 
    /// In a real Lightning implementation, you would:
    /// 1. Create the unsigned HTLC transaction
    /// 2. Send it to your counterparty to get their signature
    /// 3. Sign it yourself with your local key
    /// 4. Combine both signatures to create the witness
    /// 
    /// Witness stack: [0, remote_htlc_sig, local_htlc_sig, 0 (false), htlc_script]
    pub fn sign_htlc_timeout_transaction(
        &self,
        tx: &Transaction,
        htlc_script: &ScriptBuf,
        htlc_amount: u64,
        local_htlc_key: &SecretKey,
        remote_htlc_signature: Vec<u8>,
    ) -> Witness {
        // Sign with our local HTLC key
        let local_htlc_signature = self.sign_transaction_input(
            tx,
            0,
            htlc_script,
            htlc_amount,
            local_htlc_key,
        );

        // Create witness with both signatures (no preimage for timeout)
        self.create_htlc_timeout_witness(
            remote_htlc_signature,
            local_htlc_signature,
            htlc_script,
        )
    }

    // ========================================================================
    // HTLC WITNESS HELPERS
    // ========================================================================

    /// Create witness for HTLC-success transaction
    /// Witness stack: [0, remote_htlc_sig, local_htlc_sig, payment_preimage, htlc_script]
    pub fn create_htlc_success_witness(
        &self,
        remote_htlc_signature: Vec<u8>,
        local_htlc_signature: Vec<u8>,
        payment_preimage: [u8; 32],
        htlc_script: &ScriptBuf,
    ) -> Witness {
        Witness::from_slice(&[
            &[][..],                        // OP_0 for CHECKMULTISIG bug
            &remote_htlc_signature[..],
            &local_htlc_signature[..],
            &payment_preimage[..],
            htlc_script.as_bytes(),
        ])
    }

    /// Create witness for HTLC-timeout transaction
    /// Witness stack: [0, remote_htlc_sig, local_htlc_sig, 0 (false), htlc_script]
    pub fn create_htlc_timeout_witness(
        &self,
        remote_htlc_signature: Vec<u8>,
        local_htlc_signature: Vec<u8>,
        htlc_script: &ScriptBuf,
    ) -> Witness {
        Witness::from_slice(&[
            &[][..],                        // OP_0 for CHECKMULTISIG bug
            &remote_htlc_signature[..],
            &local_htlc_signature[..],
            &[][..],                        // OP_FALSE for timeout path
            htlc_script.as_bytes(),
        ])
    }
}

// ============================================================================
// BACKWARD COMPATIBILITY - Standalone Functions
// ============================================================================
//
// These functions are provided for backward compatibility with existing code
// that uses the standalone function API. They simply delegate to the
// InMemorySigner methods.
//
// New code should use the InMemorySigner methods directly.

/// Exercise 30: Sign a transaction input (standalone function)
/// 
/// Deprecated: Use `signer.sign_transaction_input()` instead
#[deprecated(since = "0.1.0", note = "Use InMemorySigner::sign_transaction_input() instead")]
pub fn sign_transaction_input(
    tx: &Transaction,
    input_index: usize,
    script: &ScriptBuf,
    amount: u64,
    secret_key: &SecretKey,
    secp_ctx: &Secp256k1<All>,
) -> Vec<u8> {
    let signer = InMemorySigner {
        funding_key: *secret_key,
        revocation_base_key: *secret_key,
        payment_base_key: *secret_key,
        delayed_payment_base_key: *secret_key,
        htlc_base_key: *secret_key,
        commitment_seed: [0; 32],
        secp_ctx: secp_ctx.clone(),
    };
    signer.sign_transaction_input(tx, input_index, script, amount, secret_key)
}

/// Exercise 31: Create witness for commitment transaction (standalone function)
/// 
/// Deprecated: Use `signer.create_commitment_witness()` instead
#[deprecated(since = "0.1.0", note = "Use InMemorySigner::create_commitment_witness() instead")]
pub fn create_commitment_witness(
    tx: &Transaction,
    funding_script: &ScriptBuf,
    funding_amount: u64,
    local_funding_key: &SecretKey,
    remote_funding_signature: Vec<u8>,
    secp_ctx: &Secp256k1<All>,
) -> Witness {
    let signer = InMemorySigner {
        funding_key: *local_funding_key,
        revocation_base_key: *local_funding_key,
        payment_base_key: *local_funding_key,
        delayed_payment_base_key: *local_funding_key,
        htlc_base_key: *local_funding_key,
        commitment_seed: [0; 32],
        secp_ctx: secp_ctx.clone(),
    };
    signer.create_commitment_witness(tx, funding_script, funding_amount, remote_funding_signature)
}

/// Exercise 32: Verify a signature (standalone function)
/// 
/// Deprecated: Use `signer.verify_signature()` instead
#[deprecated(since = "0.1.0", note = "Use InMemorySigner::verify_signature() instead")]
pub fn verify_signature(
    tx: &Transaction,
    input_index: usize,
    script: &ScriptBuf,
    amount: u64,
    signature: &[u8],
    pubkey: &PublicKey,
    secp_ctx: &Secp256k1<All>,
) -> bool {
    let signer = InMemorySigner {
        funding_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        revocation_base_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        payment_base_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        delayed_payment_base_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        htlc_base_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        commitment_seed: [0; 32],
        secp_ctx: secp_ctx.clone(),
    };
    signer.verify_signature(tx, input_index, script, amount, signature, pubkey)
}

/// Sign an HTLC-success transaction with local key and create its witness (standalone function)
/// 
/// Deprecated: Use `signer.sign_htlc_success_transaction()` instead
#[deprecated(since = "0.1.0", note = "Use InMemorySigner::sign_htlc_success_transaction() instead")]
pub fn sign_htlc_success_transaction(
    tx: &Transaction,
    htlc_script: &ScriptBuf,
    htlc_amount: u64,
    local_htlc_key: &SecretKey,
    remote_htlc_signature: Vec<u8>,
    payment_preimage: [u8; 32],
    secp_ctx: &Secp256k1<All>,
) -> Witness {
    let signer = InMemorySigner {
        funding_key: *local_htlc_key,
        revocation_base_key: *local_htlc_key,
        payment_base_key: *local_htlc_key,
        delayed_payment_base_key: *local_htlc_key,
        htlc_base_key: *local_htlc_key,
        commitment_seed: [0; 32],
        secp_ctx: secp_ctx.clone(),
    };
    signer.sign_htlc_success_transaction(tx, htlc_script, htlc_amount, local_htlc_key, remote_htlc_signature, payment_preimage)
}

/// Sign an HTLC-timeout transaction with local key and create its witness (standalone function)
/// 
/// Deprecated: Use `signer.sign_htlc_timeout_transaction()` instead
#[deprecated(since = "0.1.0", note = "Use InMemorySigner::sign_htlc_timeout_transaction() instead")]
pub fn sign_htlc_timeout_transaction(
    tx: &Transaction,
    htlc_script: &ScriptBuf,
    htlc_amount: u64,
    local_htlc_key: &SecretKey,
    remote_htlc_signature: Vec<u8>,
    secp_ctx: &Secp256k1<All>,
) -> Witness {
    let signer = InMemorySigner {
        funding_key: *local_htlc_key,
        revocation_base_key: *local_htlc_key,
        payment_base_key: *local_htlc_key,
        delayed_payment_base_key: *local_htlc_key,
        htlc_base_key: *local_htlc_key,
        commitment_seed: [0; 32],
        secp_ctx: secp_ctx.clone(),
    };
    signer.sign_htlc_timeout_transaction(tx, htlc_script, htlc_amount, local_htlc_key, remote_htlc_signature)
}

/// Create witness for HTLC-success transaction (standalone function)
/// Witness stack: [0, remote_htlc_sig, local_htlc_sig, payment_preimage, htlc_script]
/// 
/// Deprecated: Use `signer.create_htlc_success_witness()` instead
#[deprecated(since = "0.1.0", note = "Use InMemorySigner::create_htlc_success_witness() instead")]
pub fn create_htlc_success_witness(
    remote_htlc_signature: Vec<u8>,
    local_htlc_signature: Vec<u8>,
    payment_preimage: [u8; 32],
    htlc_script: &ScriptBuf,
) -> Witness {
    let signer = InMemorySigner {
        funding_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        revocation_base_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        payment_base_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        delayed_payment_base_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        htlc_base_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        commitment_seed: [0; 32],
        secp_ctx: Secp256k1::new(),
    };
    signer.create_htlc_success_witness(remote_htlc_signature, local_htlc_signature, payment_preimage, htlc_script)
}

/// Create witness for HTLC-timeout transaction (standalone function)
/// Witness stack: [0, remote_htlc_sig, local_htlc_sig, 0 (false), htlc_script]
/// 
/// Deprecated: Use `signer.create_htlc_timeout_witness()` instead
#[deprecated(since = "0.1.0", note = "Use InMemorySigner::create_htlc_timeout_witness() instead")]
pub fn create_htlc_timeout_witness(
    remote_htlc_signature: Vec<u8>,
    local_htlc_signature: Vec<u8>,
    htlc_script: &ScriptBuf,
) -> Witness {
    let signer = InMemorySigner {
        funding_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        revocation_base_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        payment_base_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        delayed_payment_base_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        htlc_base_key: SecretKey::from_slice(&[1; 32]).unwrap(),
        commitment_seed: [0; 32],
        secp_ctx: Secp256k1::new(),
    };
    signer.create_htlc_timeout_witness(remote_htlc_signature, local_htlc_signature, htlc_script)
}
