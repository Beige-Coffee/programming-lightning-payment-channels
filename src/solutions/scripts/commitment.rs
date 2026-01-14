use bitcoin::PublicKey as BitcoinPublicKey;
use bitcoin::secp256k1::PublicKey;
use bitcoin::script::{Builder, ScriptBuf};
use bitcoin::blockdata::opcodes::all as opcodes;
use bitcoin::hashes::{Hash, hash160};
use bitcoin::hashes::hash160::Hash as Hash160;


/// Exercise 14: Create to_remote script (P2WPKH)
pub fn create_to_remote_script(remote_pubkey: &PublicKey) -> ScriptBuf {

    // Hash the public key using HASH160
    let pubkey_hash = Hash160::hash(&remote_pubkey.serialize());

    // Build P2WPKH script: OP_0 <20-byte-pubkey-hash>
    Builder::new()
        .push_int(0)
        .push_slice(pubkey_hash.as_byte_array())
        .into_script()
}

/// Exercise 15: Create to_local script (revocable with delay)
pub fn create_to_local_script(
    revocation_pubkey: &PublicKey,
    local_delayedpubkey: &PublicKey,
    to_self_delay: u16,
) -> ScriptBuf {
    // Build script with two paths:
    // OP_IF
    //     <revocationpubkey>
    // OP_ELSE
    //     <to_self_delay>
    //     OP_CHECKSEQUENCEVERIFY
    //     OP_DROP
    //     <local_delayedpubkey>
    // OP_ENDIF
    // OP_CHECKSIG
    Builder::new()
        .push_opcode(opcodes::OP_IF)
        .push_slice(revocation_pubkey.serialize())
        .push_opcode(opcodes::OP_ELSE)
        .push_int(to_self_delay as i64)
        .push_opcode(opcodes::OP_CSV)
        .push_opcode(opcodes::OP_DROP)
        .push_slice(local_delayedpubkey.serialize())
        .push_opcode(opcodes::OP_ENDIF)
        .push_opcode(opcodes::OP_CHECKSIG)
        .into_script()
}