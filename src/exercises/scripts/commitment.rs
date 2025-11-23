use bitcoin::PublicKey as BitcoinPublicKey;
use bitcoin::secp256k1::PublicKey;
use bitcoin::script::{Builder, ScriptBuf};
use bitcoin::blockdata::opcodes::all as opcodes;
use bitcoin::hashes::{Hash, hash160};
use bitcoin::hashes::hash160::Hash as Hash160;


/// Exercise 14: Create to_remote script (P2WPKH)
/// This output goes to the counterparty and is immediately spendable by them
pub fn create_to_remote_script(remote_pubkey: &PublicKey) -> ScriptBuf {
    // P2WPKH format: OP_0 <20-byte-pubkey-hash>
    let pubkey_hash = Hash160::hash(&remote_pubkey.serialize());
    Builder::new()
        .push_int(0)
        .push_slice(pubkey_hash.as_byte_array())
        .into_script()
}

/// Exercise 15: Create to_local script (revocable with delay)
/// This output goes to us but has a time delay and can be revoked by counterparty
pub fn create_to_local_script(
    revocation_pubkey: &PublicKey,
    local_delayedpubkey: &PublicKey,
    to_self_delay: u16,
) -> ScriptBuf {
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