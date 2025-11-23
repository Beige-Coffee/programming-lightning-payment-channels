use bitcoin::{Transaction, TxIn, TxOut, OutPoint, Sequence, Witness, Amount};
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::PublicKey;
use bitcoin::transaction::Version;
use bitcoin::locktime::absolute::LockTime;
use bitcoin::Txid;
use bitcoin::PublicKey as BitcoinPublicKey;

use crate::scripts::funding::create_funding_script;

/// Exercise 6
pub fn create_funding_transaction(
    input_txid: Txid,
    input_vout: u32,
    funding_amount_sat: u64,
    local_funding_pubkey: &BitcoinPublicKey,
    remote_funding_pubkey: &BitcoinPublicKey,
) -> Transaction {
    // Create the funding script (2-of-2 multisig)
    let funding_script = create_funding_script(local_funding_pubkey, remote_funding_pubkey);
    
    // Convert to P2WSH (pay-to-witness-script-hash)
    let funding_script_pubkey = funding_script.to_p2wsh();

    let tx_input = TxIn {
            previous_output: OutPoint {
                txid: input_txid,
                vout: input_vout,
            },
            script_sig: ScriptBuf::new(), // Empty for SegWit
            sequence: Sequence::MAX,      // 0xffffffff (RBF disabled)
            witness: Witness::new(),      // Witness will be added when signing
        };

    let output = TxOut {
            value: Amount::from_sat(funding_amount_sat),
            script_pubkey: funding_script_pubkey,
        };

    
    // Create the transaction
    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![tx_input],
        output: vec![output],
    }
}

