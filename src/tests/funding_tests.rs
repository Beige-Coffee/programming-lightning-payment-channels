use crate::*;
use bitcoin::secp256k1::PublicKey;
use bitcoin::{Transaction, TxIn, TxOut, OutPoint, Sequence, Witness, Amount};
use bitcoin::script::ScriptBuf;
use bitcoin::transaction::Version;
use bitcoin::locktime::absolute::LockTime;
use hex;
use bitcoin::hashes::{Hash, sha256};
use bitcoin::PublicKey as BitcoinPublicKey;

#[test]
fn test_bolt3_funding_script() {
    println!("\n=== Testing BOLT 3 Funding Script ===\n");
    
    // Test vector pubkeys
    let local_pubkey_hex = "023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb";
    let remote_pubkey_hex = "030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c1";
    
    let local_pubkey = BitcoinPublicKey::new(PublicKey::from_slice(
        &hex::decode(local_pubkey_hex).unwrap()
    ).unwrap());
    
    let remote_pubkey = BitcoinPublicKey::new(PublicKey::from_slice(
        &hex::decode(remote_pubkey_hex).unwrap()
    ).unwrap());
    
    // Expected funding witness script from BOLT 3
    let expected_script_hex = "5221023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb21030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c152ae";
    let expected_script = hex::decode(expected_script_hex).unwrap();
    
    // Create funding script using our function
    let funding_script = create_funding_script(&local_pubkey, &remote_pubkey);
    let actual_script = funding_script.as_bytes();
    
    println!("Local pubkey:  {}", local_pubkey_hex);
    println!("Remote pubkey: {}", remote_pubkey_hex);
    println!("\nExpected script: {}", expected_script_hex);
    println!("Actual script:   {}\n", hex::encode(actual_script));
    
    assert_eq!(
        actual_script,
        expected_script.as_slice(),
        "Funding script does not match BOLT 3 test vector"
    );
    
    println!("✓ Funding script matches BOLT 3 test vector!\n");
}

#[test]
fn test_bolt3_funding_transaction() {
    println!("\n=== Testing BOLT 3 Funding Transaction ===\n");
    
    // Test vector values
    let input_txid_hex = "fd2105607605d2302994ffea703b09f66b6351816ee737a93e42a841ea20bbad";
    let input_vout = 0u32;
    let input_amount = 5_000_000_000u64;
    let funding_amount = 10_000_000u64;
    let change_amount = 4_989_986_080u64;
    
    // Pubkeys
    let local_pubkey = BitcoinPublicKey::new(PublicKey::from_slice(
        &hex::decode("023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb").unwrap()
    ).unwrap());
    
    let remote_pubkey = BitcoinPublicKey::new(PublicKey::from_slice(
        &hex::decode("030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c1").unwrap()
    ).unwrap());
    
    // Expected values
    let expected_txid = "8984484a580b825b9972d7adb15050b3ab624ccd731946b3eeddb92f4e7ef6be";
    let expected_funding_output_script = "0020c015c4a6be010e21657068fc2e6a9d02b27ebe4d490a25846f7237f104d1a3cd";
    
    // Create funding script
    let funding_script = create_funding_script(&local_pubkey, &remote_pubkey);
    let funding_script_p2wsh = funding_script.to_p2wsh();
    
    println!("Input TXID: {}", input_txid_hex);
    println!("Input vout: {}", input_vout);
    println!("Input amount: {} sats", input_amount);
    println!("Funding amount: {} sats", funding_amount);
    println!("Change amount: {} sats", change_amount);
    
    println!("\nFunding script (witness): {}", hex::encode(funding_script.as_bytes()));
    println!("Funding script (P2WSH):   {}", hex::encode(funding_script_p2wsh.as_bytes()));
    println!("Expected P2WSH:           {}", expected_funding_output_script);
    
    // Check if P2WSH script matches
    let actual_p2wsh_hex = hex::encode(funding_script_p2wsh.as_bytes());
    assert_eq!(
        actual_p2wsh_hex,
        expected_funding_output_script,
        "Funding output script (P2WSH) does not match"
    );
    
    println!("\n✓ Funding output script matches!\n");
    
    // Create the Funding Transaction structure
    let mut txid_bytes = hex::decode(input_txid_hex).unwrap();
    txid_bytes.reverse(); // Bitcoin uses little-endian for txids
    
    let input_txid = bitcoin::Txid::from_slice(&txid_bytes).unwrap();
    
    let funding_tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![
            TxIn {
                previous_output: OutPoint {
                    txid: input_txid,
                    vout: input_vout,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            }
        ],
        output: vec![
            TxOut {
                value: Amount::from_sat(funding_amount),
                script_pubkey: funding_script_p2wsh.clone(),
            },
            TxOut {
                value: Amount::from_sat(change_amount),
                script_pubkey: ScriptBuf::new(),
            }
        ],
    };
    
    println!("Transaction structure:");
    println!("  Version: {:?}", funding_tx.version);
    println!("  Inputs: {}", funding_tx.input.len());
    println!("  Outputs: {}", funding_tx.output.len());
    println!("\n  Output 0:");
    println!("    Amount: {} sats", funding_tx.output[0].value.to_sat());
    println!("    Script: {}", hex::encode(funding_tx.output[0].script_pubkey.as_bytes()));
    println!("\n  Output 1:");
    println!("    Amount: {} sats", funding_tx.output[1].value.to_sat());
    
    println!("\n✓ Transaction structure created successfully!");
    println!("\nNote: Full transaction verification requires the private key for signing.");
    println!("The funding script and P2WSH output match the BOLT 3 test vector. ✓\n");
}

#[test]
fn test_funding_script_ordering() {
    println!("\n=== Testing Funding Script Pubkey Ordering ===\n");
    
    // Test that pubkeys are correctly sorted (lexicographically)
    let pubkey1 = BitcoinPublicKey::new(PublicKey::from_slice(
        &hex::decode("023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb").unwrap()
    ).unwrap());
    
    let pubkey2 = BitcoinPublicKey::new(PublicKey::from_slice(
        &hex::decode("030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c1").unwrap()
    ).unwrap());
    
    // Test both orderings should produce the same result
    let script1 = create_funding_script(&pubkey1, &pubkey2);
    let script2 = create_funding_script(&pubkey2, &pubkey1);
    
    println!("Script with pubkey1 first: {}", hex::encode(script1.as_bytes()));
    println!("Script with pubkey2 first: {}", hex::encode(script2.as_bytes()));
    
    assert_eq!(
        script1.as_bytes(),
        script2.as_bytes(),
        "Funding scripts with different input orders should be identical"
    );
    
    println!("\n✓ Pubkey ordering is deterministic!\n");
}