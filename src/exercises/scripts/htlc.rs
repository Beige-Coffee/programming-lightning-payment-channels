use bitcoin::PublicKey as BitcoinPublicKey;
use bitcoin::secp256k1::PublicKey;
use bitcoin::script::{Builder, ScriptBuf};
use bitcoin::blockdata::opcodes::all as opcodes;
use bitcoin::hashes::{Hash, ripemd160, hash160};
use bitcoin::hashes::ripemd160::Hash as Ripemd160;
use bitcoin::hashes::hash160::Hash as Hash160;
use bitcoin::{PubkeyHash, WPubkeyHash};
use hex;


/// Exercise 21: Create offered HTLC script
pub fn create_offered_htlc_script(
    revocation_pubkey: &PublicKey,
    local_htlcpubkey: &PublicKey,
    remote_htlcpubkey: &PublicKey,
    payment_hash: &[u8; 32],
) -> ScriptBuf {

    unimplemented!();

    // Hash the payment hash with RIPEMD160

    // Hash the revocation public key with PubkeyHash

    // Build script with three paths: revocation, remote with preimage, local with timeout
    // OP_DUP
    // OP_HASH160
    // <revocation_pubkey_hash>
    // OP_EQUAL
    // OP_IF
    //     OP_CHECKSIG
    // OP_ELSE
    //     <remote_htlcpubkey>
    //     OP_SWAP
    //     OP_SIZE
    //     32
    //     OP_EQUAL
    //     OP_NOTIF
    //         OP_DROP
    //         2
    //         OP_SWAP
    //         <local_htlcpubkey>
    //         2
    //         OP_CHECKMULTISIG
    //     OP_ELSE
    //         OP_HASH160
    //         <payment_hash160>
    //         OP_EQUALVERIFY
    //         OP_CHECKSIG
    //     OP_ENDIF
    // OP_ENDIF
}

/// Exercise 24: Create received HTLC script
pub fn create_received_htlc_script(
    revocation_pubkey: &PublicKey,
    local_htlcpubkey: &PublicKey,
    remote_htlcpubkey: &PublicKey,
    payment_hash: &[u8; 32],
    cltv_expiry: u32,
) -> ScriptBuf {

    unimplemented!();

    // Hash the payment hash with RIPEMD160

    // Hash the revocation public key with PubkeyHash

    // Build script with three paths: revocation, local with preimage, remote with timeout
    // OP_DUP
    // OP_HASH160
    // <revocation_pubkey_hash>
    // OP_EQUAL
    // OP_IF
    //     OP_CHECKSIG
    // OP_ELSE
    //     <remote_htlcpubkey>
    //     OP_SWAP
    //     OP_SIZE
    //     32
    //     OP_EQUAL
    //     OP_IF
    //         OP_HASH160
    //         <payment_hash160>
    //         OP_EQUALVERIFY
    //         2
    //         OP_SWAP
    //         <local_htlcpubkey>
    //         2
    //         OP_CHECKMULTISIG
    //     OP_ELSE
    //         OP_DROP
    //         <cltv_expiry>
    //         OP_CLTV
    //         OP_DROP
    //         OP_CHECKSIG
    //     OP_ENDIF
    // OP_ENDIF

}