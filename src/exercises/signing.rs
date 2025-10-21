use bitcoin::{Transaction, Amount, Witness};
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey, Message, All};
use bitcoin::sighash::{SighashCache, EcdsaSighashType};
use bitcoin::hashes::{Hash, sha256};
// ============================================================================
// TRANSACTION SIGNING & VERIFICATION
// ============================================================================

/// Exercise 30: Sign a transaction input
pub fn sign_transaction_input(
    tx: &Transaction,
    input_index: usize,
    script: &ScriptBuf,
    amount: u64,
    secret_key: &SecretKey,
    secp_ctx: &Secp256k1<All>,
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
    let sig = secp_ctx.sign_ecdsa(&msg, secret_key);
    
    let mut sig_bytes = sig.serialize_der().to_vec();
    sig_bytes.push(EcdsaSighashType::All as u8);
    sig_bytes
}

/// Exercise 31: Create witness for commitment transaction
/// 
/// In a real Lightning implementation:
/// 1. You create the unsigned commitment transaction
/// 2. You send it to your counterparty to get their signature
/// 3. You sign it with your local funding key
/// 4. You combine both signatures to create the witness
/// 
/// This function only signs with your local key and expects the remote signature
/// as a parameter (which you would have received from your counterparty).
pub fn create_commitment_witness(
    tx: &Transaction,
    funding_script: &ScriptBuf,
    funding_amount: u64,
    local_funding_key: &SecretKey,
    remote_funding_signature: Vec<u8>,
    secp_ctx: &Secp256k1<All>,
) -> Witness {
    // Sign with our local key
    let local_sig = sign_transaction_input(
        tx, 
        0, 
        funding_script, 
        funding_amount, 
        local_funding_key, 
        secp_ctx
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
pub fn verify_signature(
    tx: &Transaction,
    input_index: usize,
    script: &ScriptBuf,
    amount: u64,
    signature: &[u8],
    pubkey: &PublicKey,
    secp_ctx: &Secp256k1<All>,
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
    
    secp_ctx.verify_ecdsa(&msg, &sig, pubkey).is_ok()
}

// ============================================================================
// HTLC TRANSACTION SIGNING
// ============================================================================

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
    tx: &Transaction,
    htlc_script: &ScriptBuf,
    htlc_amount: u64,
    local_htlc_key: &SecretKey,
    remote_htlc_signature: Vec<u8>,
    payment_preimage: [u8; 32],
    secp_ctx: &Secp256k1<All>,
) -> Witness {
    // Sign with our local HTLC key
    let local_htlc_signature = sign_transaction_input(
        tx,
        0,
        htlc_script,
        htlc_amount,
        local_htlc_key,
        secp_ctx,
    );

    // Create witness with both signatures and payment preimage
    create_htlc_success_witness(
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
    tx: &Transaction,
    htlc_script: &ScriptBuf,
    htlc_amount: u64,
    local_htlc_key: &SecretKey,
    remote_htlc_signature: Vec<u8>,
    secp_ctx: &Secp256k1<All>,
) -> Witness {
    // Sign with our local HTLC key
    let local_htlc_signature = sign_transaction_input(
        tx,
        0,
        htlc_script,
        htlc_amount,
        local_htlc_key,
        secp_ctx,
    );

    // Create witness with both signatures (no preimage for timeout)
    create_htlc_timeout_witness(
        remote_htlc_signature,
        local_htlc_signature,
        htlc_script,
    )
}

// ============================================================================
// HTLC WITNESS HELPERS
// ============================================================================

/// Create witness for HTLC-success transaction
/// Witness stack: [0, remote_htlc_sig, local_htlc_sig, payment_preimage, htlc_script]
pub fn create_htlc_success_witness(
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
