use bitcoin::{Transaction, TxIn, TxOut, OutPoint, Sequence, Witness, Amount};
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::PublicKey;
use bitcoin::transaction::Version;
use bitcoin::locktime::absolute::LockTime;
use bitcoin::Txid;

use crate::scripts::funding::create_funding_script;

// ============================================================================
// SECTION 2: FUNDING TRANSACTIONS
// ============================================================================
// These exercises teach how to create the on-chain funding transaction that
// opens a Lightning channel. The funding transaction is a standard Bitcoin
// transaction with one special output - a 2-of-2 multisig that will become
// the channel capacity.

/// Exercise 6: Create a basic funding transaction with one input and one output
/// 
/// This creates the simplest possible funding transaction:
/// - One input (spending from an existing UTXO)
/// - One output (the 2-of-2 multisig funding output)
/// 
/// In practice, you'd also need a change output, but we'll start simple.
pub fn create_funding_transaction(
    input_txid: Txid,
    input_vout: u32,
    funding_amount_sat: u64,
    local_funding_pubkey: &PublicKey,
    remote_funding_pubkey: &PublicKey,
) -> Transaction {
    // Create the funding script (2-of-2 multisig)
    let funding_script = create_funding_script(local_funding_pubkey, remote_funding_pubkey);
    
    // Convert to P2WSH (pay-to-witness-script-hash)
    let funding_script_pubkey = funding_script.to_p2wsh();
    
    // Create the transaction
    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![
            TxIn {
                previous_output: OutPoint {
                    txid: input_txid,
                    vout: input_vout,
                },
                script_sig: ScriptBuf::new(), // Empty for SegWit
                sequence: Sequence::MAX,      // 0xffffffff (RBF disabled)
                witness: Witness::new(),      // Witness will be added when signing
            }
        ],
        output: vec![
            TxOut {
                value: Amount::from_sat(funding_amount_sat),
                script_pubkey: funding_script_pubkey,
            }
        ],
    }
}

