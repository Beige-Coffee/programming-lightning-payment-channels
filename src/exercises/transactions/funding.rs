use bitcoin::{Transaction, TxIn, TxOut, OutPoint, Sequence, Witness, Amount};
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::PublicKey;
use bitcoin::transaction::Version;
use bitcoin::locktime::absolute::LockTime;
use bitcoin::Txid;
use bitcoin::PublicKey as BitcoinPublicKey;

use crate::scripts::funding::create_funding_script;

/// Exercise 6: Create funding transaction
pub fn create_funding_transaction(
    input_txid: Txid,
    input_vout: u32,
    funding_amount_sat: u64,
    local_funding_pubkey: &BitcoinPublicKey,
    remote_funding_pubkey: &BitcoinPublicKey,
) -> Transaction {
    // Create the 2-of-2 multisig script
    let funding_script = create_funding_script(local_funding_pubkey, remote_funding_pubkey);

    // Convert to P2WSH output
    let funding_script_pubkey = funding_script.to_p2wsh();

    // Create input (TxIn) spending from previous transaction
    let tx_input = TxIn {
            previous_output: OutPoint {
                txid: input_txid,
                vout: input_vout,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        };

    // Create output (TxOut) with funding amount locked to multisig script
    let output = TxOut {
            value: Amount::from_sat(funding_amount_sat),
            script_pubkey: funding_script_pubkey,
        };

    // Assemble & Return the transaction
    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![tx_input],
        output: vec![output],
    }
}

