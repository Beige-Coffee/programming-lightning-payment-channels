use bitcoin::{Transaction, TxIn, TxOut, OutPoint, Sequence, Witness, Amount};
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{Secp256k1, PublicKey, SecretKey};
use bitcoin::transaction::Version;
use bitcoin::locktime::absolute::LockTime;
use bitcoin::consensus::encode::serialize_hex;

use crate::internal::bitcoind_client::{BitcoindClient, get_bitcoind_client};
use crate::scripts::funding::create_funding_script;

/// Helper function to create a public key from a private key
fn pubkey_from_private_key(private_key_bytes: &[u8; 32]) -> PublicKey {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(private_key_bytes).expect("Invalid private key");
    PublicKey::from_secret_key(&secp, &secret_key)
}

/// Build a funding transaction with the given inputs
fn build_funding_transaction(
    inputs: Vec<TxIn>,
    our_pubkey: &PublicKey,
    counterparty_pubkey: &PublicKey,
    funding_amount: u64,
) -> Transaction {
    // Create the 2-of-2 multisig funding script
    let funding_script = create_funding_script(our_pubkey, counterparty_pubkey);
    let funding_script_pubkey = funding_script.to_p2wsh();
    
    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: inputs,
        output: vec![
            TxOut {
                value: Amount::from_sat(funding_amount),
                script_pubkey: funding_script_pubkey,
            }
        ],
    }
}

/// Sign a raw transaction using the bitcoind wallet
async fn sign_raw_transaction(bitcoind: BitcoindClient, tx: Transaction) -> Transaction {
    let tx_hex = serialize_hex(&tx);
    let signed_tx = bitcoind.sign_raw_transaction_with_wallet(tx_hex).await;
    
    if !signed_tx.complete {
        panic!("Transaction signing failed or incomplete");
    }
    
    // Decode the signed transaction
    let signed_tx_bytes = hex::decode(&signed_tx.hex).expect("Invalid hex");
    bitcoin::consensus::encode::deserialize(&signed_tx_bytes)
        .expect("Failed to deserialize transaction")
}

/// Main function to build a funding transaction
/// 
/// This function:
/// 1. Takes a TxIn (input UTXO) and its amount
/// 2. Creates public keys for both parties
/// 3. Builds a 2-of-2 multisig funding transaction
/// 4. Signs it with bitcoind
/// 5. Prints the TX ID and hex
pub async fn build_funding_tx(
    bitcoind: BitcoindClient,
    tx_input: TxIn,
    tx_in_amount: u64,
) {
    println!("\n=== Building Funding Transaction ===\n");
    
    // Generate public keys for both parties
    // In a real scenario, you would generate your own and receive the counterparty's
    let our_public_key = pubkey_from_private_key(&[0x01; 32]);
    let counterparty_pubkey = pubkey_from_private_key(&[0x02; 32]);
    
    println!("Our public key: {}", hex::encode(our_public_key.serialize()));
    println!("Counterparty public key: {}", hex::encode(counterparty_pubkey.serialize()));
    
    // Build funding transaction
    let tx = build_funding_transaction(
        vec![tx_input],
        &our_public_key,
        &counterparty_pubkey,
        tx_in_amount,
    );
    
    println!("\nSigning transaction...");
    let signed_tx = sign_raw_transaction(bitcoind.clone(), tx).await;
    
    println!("\n✓ Funding Transaction Created\n");
    println!("Tx ID: {}", signed_tx.compute_txid());
    println!("\nTx Hex: {}", serialize_hex(&signed_tx));
    println!();
}

/// Interactive CLI function to create a funding transaction
/// This fetches a UTXO automatically and creates the funding transaction
pub async fn run() {
    println!("\n=== Lightning Channel Funding Transaction ===\n");
    
    // Connect to bitcoind
    println!("Connecting to bitcoind...");
    let bitcoind = get_bitcoind_client().await;
    println!("✓ Connected\n");
    
    // Fetch available UTXOs
    println!("Fetching available UTXOs...");
    let unspent = bitcoind.list_unspent().await;
    
    if unspent.0.is_empty() {
        println!("❌ No UTXOs available. Please fund your wallet first.");
        println!("   Run: bitcoin-cli -regtest generatetoaddress 101 $(bitcoin-cli -regtest getnewaddress)");
        return;
    }
    
    // Use the first UTXO
    let selected_utxo = &unspent.0[0];
    println!("✓ Using UTXO:");
    println!("  TXID: {}", selected_utxo.txid);
    println!("  Vout: {}", selected_utxo.vout);
    println!("  Amount: {} sats\n", selected_utxo.amount);
    
    // Create TxIn from the UTXO
    let tx_input = TxIn {
        previous_output: OutPoint {
            txid: selected_utxo.txid,
            vout: selected_utxo.vout,
        },
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    };
    
    // Use the full amount (minus a small buffer for fees if needed)
    let funding_amount = selected_utxo.amount;
    
    // Build and display the funding transaction
    build_funding_tx(bitcoind, tx_input, funding_amount).await;
}
