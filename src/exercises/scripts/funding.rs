use bitcoin::PublicKey as BitcoinPublicKey;
use bitcoin::secp256k1::PublicKey;
use bitcoin::script::{Builder, ScriptBuf};
use bitcoin::blockdata::opcodes::all as opcodes;

// ============================================================================
// SECTION 3: FUNDING SCRIPTS
// ============================================================================
// Funding scripts are used to create the on-chain transaction that opens
// the Lightning channel.

/// Exercise 13: Create a 2-of-2 multisig funding script
/// Both parties must sign to spend from this output
pub fn create_funding_script(pubkey1: &BitcoinPublicKey, pubkey2: &BitcoinPublicKey) -> ScriptBuf {
    // Sort pubkeys for determinism (BOLT 3 requirement)
    let (pubkey_lesser, pubkey_larger) = if pubkey1.inner.serialize() < pubkey2.inner.serialize() {
        (pubkey1, pubkey2)
    } else {
        (pubkey2, pubkey1)
    };
    Builder::new()
        .push_int(2)
        .push_key(pubkey_lesser)
        .push_key(pubkey_larger)
        .push_int(2)
        .push_opcode(opcodes::OP_CHECKMULTISIG)
        .into_script()
}