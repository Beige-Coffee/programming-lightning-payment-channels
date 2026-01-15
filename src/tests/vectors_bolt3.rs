use crate::types::{Bolt3Htlc, Bolt3TestVector, ChannelKeyManager, CommitmentKeys, HtlcDirection};
use crate::*;
use bitcoin::consensus::encode;
use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::{sha256, Hash};
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use bitcoin::{OutPoint, Transaction, Witness};
use hex;


// These helper functions are used only in tests to construct witnesses
// for verifying student implementations of finalize_htlc_success and
// finalize_htlc_timeout.

/// Create witness for HTLC-success transaction (test-only)
/// 
/// Witness stack: [0, remote_htlc_sig, local_htlc_sig, payment_preimage, htlc_script]
fn create_htlc_success_witness(
    remote_htlc_signature: Vec<u8>,
    local_htlc_signature: Vec<u8>,
    payment_preimage: [u8; 32],
    htlc_script: &ScriptBuf,
) -> Witness {
    Witness::from_slice(&[
        &[][..],
        &remote_htlc_signature[..],
        &local_htlc_signature[..],
        &payment_preimage[..],
        htlc_script.as_bytes(),
    ])
}

/// Create witness for HTLC-timeout transaction (test-only)
/// 
/// Witness stack: [0, remote_htlc_sig, local_htlc_sig, 0 (false), htlc_script]
fn create_htlc_timeout_witness(
    remote_htlc_signature: Vec<u8>,
    local_htlc_signature: Vec<u8>,
    htlc_script: &ScriptBuf,
) -> Witness {
    Witness::from_slice(&[
        &[][..],
        &remote_htlc_signature[..],
        &local_htlc_signature[..],
        &[][..],
        htlc_script.as_bytes(),
    ])
}

// Helper function to create the common test vector base
// Based on BOLT3 Appendix C: Commitment and HTLC Transaction Test Vectors
//
// Common parameters used across all BOLT3 test vectors:
// https://github.com/lightning/bolts/blob/master/03-transactions.md#appendix-b-funding-transaction-test-vectors
//
//   funding_tx_id: 8984484a580b825b9972d7adb15050b3ab624ccd731946b3eeddb92f4e7ef6be
//   funding_output_index: 0
//   funding_amount_satoshi: 10000000
//   commitment_number: 42
//   local_delay: 144 blocks
//   local_dust_limit_satoshi: 546
fn create_base_test_vector() -> Bolt3TestVector {
    let secp = Secp256k1::new();

    // Funding transaction details
    let mut funding_txid = [0u8; 32];
    hex::decode_to_slice(
        "8984484a580b825b9972d7adb15050b3ab624ccd731946b3eeddb92f4e7ef6be",
        &mut funding_txid,
    )
    .unwrap();
    funding_txid.reverse(); // Convert to little-endian

    // Keys from test vector
    let local_funding_privkey = SecretKey::from_slice(
        &hex::decode("30ff4956bbdd3222d44cc5e8a1261dab1e07957bdac5ae88fe3261ef321f374901").unwrap()
            [..32],
    )
    .unwrap();

    let remote_funding_pubkey = PublicKey::from_slice(
        &hex::decode("030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c1").unwrap(),
    )
    .unwrap();

    // this is not listed in BOLT3 test vectors, so a placeholder is used
    let local_revocation_basepoint_secret = SecretKey::from_slice(
        &hex::decode("bb13b121cdc357cd2e608b0aea294afca36e2b34cf958e2e6451a2f274694491").unwrap()
            [..32],
    )
    .unwrap();

    let local_payment_basepoint_secret = SecretKey::from_slice(
        &hex::decode("bb13b121cdc357cd2e608b0aea294afca36e2b34cf958e2e6451a2f27469449101").unwrap()
            [..32],
    )
    .unwrap();

    // only local_delayedpubkey is provided in BOLT3 test vectors, so a placeholder is used
    let local_delayed_payment_basepoint_secret = SecretKey::from_slice(
        &hex::decode("034f355bdcb7cc0af728ef3cceb9615d90684bb5b2ca5f859ab0f0b704075871aa").unwrap()
            [..32],
    )
    .unwrap();

    let local_htlc_basepoint = PublicKey::from_slice(
        &hex::decode("034f355bdcb7cc0af728ef3cceb9615d90684bb5b2ca5f859ab0f0b704075871aa").unwrap(),
    )
    .unwrap();

    // only local_htlc_basepoint is provided in BOLT3 test vectors, so a placeholder is used
    let local_htlc_basepoint_secret = SecretKey::from_slice(
        &hex::decode("034f355bdcb7cc0af728ef3cceb9615d90684bb5b2ca5f859ab0f0b704075871aa").unwrap()
            [..32],
    )
    .unwrap();

    let local_revocation_pubkey = PublicKey::from_slice(
        &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19").unwrap(),
    )
    .unwrap();

    let remote_payment_basepoint = PublicKey::from_slice(
        &hex::decode("032c0b7cf95324a07d05398b240174dc0c2be444d96b159aa6c7f7b1e668680991").unwrap(),
    )
    .unwrap();

    let local_payment_basepoint = PublicKey::from_slice(
        &hex::decode("034f355bdcb7cc0af728ef3cceb9615d90684bb5b2ca5f859ab0f0b704075871aa").unwrap(),
    )
    .unwrap();

    let local_delayedpubkey = PublicKey::from_slice(
        &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c").unwrap(),
    )
    .unwrap();

    // not provided in BOLT3 test vectors, so a placeholder is used
    let remote_delayed_payment_basepoint = PublicKey::from_slice(
        &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c").unwrap(),
    )
    .unwrap();

    let remote_htlc_basepoint = PublicKey::from_slice(
        &hex::decode("032c0b7cf95324a07d05398b240174dc0c2be444d96b159aa6c7f7b1e668680991").unwrap(),
    )
    .unwrap();

    let local_htlcpubkey = PublicKey::from_slice(
        &hex::decode("030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e7").unwrap(),
    )
    .unwrap();

    let remote_htlcpubkey = PublicKey::from_slice(
        &hex::decode("0394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b").unwrap(),
    )
    .unwrap();

    // Per-commitment secret for commitment number 42
    let per_commitment_seed =
        hex::decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f").unwrap();

    let mut per_commitment_seed_arr = [0u8; 32];
    per_commitment_seed_arr.copy_from_slice(&per_commitment_seed);

    let remote_funding_output_signature = hex::decode(
        "3045022100c3127b33dcc741dd6b05b1e63cbd1a9a7d816f37af9b6756fa2376b056f032370220408b96279808fe57eb7e463710804cdf4f108388bc5cf722d8c848d2c7f9f3b001"
    ).unwrap();

    let local_funding_output_signature = hex::decode(
        "30440220616210b2cc4d3afb601013c373bbd8aac54febd9f15400379a8cb65ce7deca60022034236c010991beb7ff770510561ae8dc885b8d38d1947248c38f2ae05564714201"
    ).unwrap();

    let funding_witness_script = hex::decode(
        "5221023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb21030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c152ae"
    ).unwrap();

    Bolt3TestVector {
        funding_txid,
        funding_output_index: 0,
        funding_amount_satoshi: 10_000_000,
        funding_witness_script,
        commitment_number: 42,
        local_delay: 144,
        local_dust_limit_satoshi: 546,
        feerate_per_kw: 15000,
        to_local_msat: 7_000_000_000,
        to_remote_msat: 3_000_000_000,
        local_funding_output_signature,
        remote_funding_output_signature,
        local_funding_privkey,
        remote_funding_pubkey,
        local_revocation_basepoint_secret,
        local_payment_basepoint_secret,
        local_delayed_payment_basepoint_secret,
        local_delayedpubkey,
        local_htlcpubkey,
        remote_htlcpubkey,
        local_htlc_basepoint,
        local_htlc_basepoint_secret,
        local_revocation_pubkey,
        remote_payment_basepoint,
        local_payment_basepoint,
        remote_delayed_payment_basepoint,
        remote_htlc_basepoint,
        commitment_seed: per_commitment_seed_arr,
    }
}

#[test]
fn test_bolt3_simple_commitment_no_htlcs() {
    println!("\n=== Testing: simple commitment tx with no HTLCs ===\n");

    let test_vector = create_base_test_vector();

    // builds an unsigned commitment transaction
    let commitment_tx = build_bolt3_simple_commitment(&test_vector);

    // get funding output signatures from test vector
    let local_funding_output_signature = test_vector.local_funding_output_signature.clone();
    let remote_funding_output_signature = test_vector.remote_funding_output_signature.clone();

    // Build witness stack
    let commitment_witness = Witness::from_slice(&[
        &[][..],
        &local_funding_output_signature,
        &remote_funding_output_signature,
        &test_vector.funding_witness_script,
    ]);

    let mut signed_commitment_tx = commitment_tx.clone();
    signed_commitment_tx.input[0].witness = commitment_witness;

    let expected_tx = "02000000000101bef67e4e2fb9ddeeb3461973cd4c62abb35050b1add772995b820b584a488489000000000038b02b8002c0c62d0000000000160014cc1b07838e387deacd0e5232e1e8b49f4c29e48454a56a00000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e04004730440220616210b2cc4d3afb601013c373bbd8aac54febd9f15400379a8cb65ce7deca60022034236c010991beb7ff770510561ae8dc885b8d38d1947248c38f2ae05564714201483045022100c3127b33dcc741dd6b05b1e63cbd1a9a7d816f37af9b6756fa2376b056f032370220408b96279808fe57eb7e463710804cdf4f108388bc5cf722d8c848d2c7f9f3b001475221023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb21030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c152ae3e195220";
    let expected_num_outputs = 2;

    let actual_tx = encode::serialize_hex(&signed_commitment_tx);

    println!("Expected TX: {}", expected_tx);
    println!("Actual TX:   {}", actual_tx);

    assert_eq!(expected_tx, actual_tx, "TX Should be equal");

    println!("\n✓ Basic commitment transaction structure verified");

}

#[test]
fn test_bolt3_commitment_with_htlcs_minimum_feerate() {
    // Commit Tx Parameters are the same as simple commitment tx with no HTLCs
    //
    // HTLC Parameters:
    //   HTLC 0: remote->local, 1000000 msat, expiry 500
    //           preimage: 0000000000000000000000000000000000000000000000000000000000000000
    //   HTLC 1: remote->local, 2000000 msat, expiry 501
    //           preimage: 0101010101010101010101010101010101010101010101010101010101010101
    //   HTLC 2: local->remote, 2000000 msat, expiry 502
    //           preimage: 0202020202020202020202020202020202020202020202020202020202020202
    //   HTLC 3: local->remote, 3000000 msat, expiry 503
    //           preimage: 0303030303030303030303030303030303030303030303030303030303030303
    //   HTLC 4: remote->local, 4000000 msat, expiry 504
    //           preimage: 0404040404040404040404040404040404040404040404040404040404040404
    //

    println!("\n=== Testing: commitment tx with all five HTLCs untrimmed (minimum feerate) ===\n");

    let mut test_vector = create_base_test_vector();
    test_vector.feerate_per_kw = 0;
    test_vector.to_local_msat = 6_988_000_000;
    test_vector.to_remote_msat = 3_000_000_000;

    let mut local_funding_output_signature = hex::decode(
        "304402206fc2d1f10ea59951eefac0b4b7c396a3c3d87b71ff0b019796ef4535beaf36f902201765b0181e514d04f4c8ad75659d7037be26cdb3f8bb6f78fe61decef484c3ea"
    ).unwrap();
    local_funding_output_signature.push(0x01);

    let mut remote_funding_output_signature = hex::decode(
        "3044022009b048187705a8cbc9ad73adbe5af148c3d012e1f067961486c822c7af08158c022006d66f3704cfab3eb2dc49dae24e4aa22a6910fc9b424007583204e3621af2e5"
    ).unwrap();
    remote_funding_output_signature.push(0x01);

    // HTLCs ordered by their index in the test vector
    let htlcs = vec![
        // HTLC #0 - remote->local (Received) 1000 msat, expiry 500
        Bolt3Htlc {
            direction: HtlcDirection::Received,
            amount_msat: 1_000_000,
            payment_hash: Sha256::hash(&[0u8; 32]).to_byte_array(),
            cltv_expiry: 500,
        },
        // HTLC #1 - remote->local (Received) 2000 msat, expiry 501
        Bolt3Htlc {
            direction: HtlcDirection::Received,
            amount_msat: 2_000_000,
            payment_hash: Sha256::hash(&[0x01; 32]).to_byte_array(),
            cltv_expiry: 501,
        },
        // HTLC #2 - local->remote (Offered) 2000 msat, expiry 502
        Bolt3Htlc {
            direction: HtlcDirection::Offered,
            amount_msat: 2_000_000,
            payment_hash: Sha256::hash(&[0x02; 32]).to_byte_array(),
            cltv_expiry: 502,
        },
        // HTLC #3 - local->remote (Offered) 3000 msat, expiry 503
        Bolt3Htlc {
            direction: HtlcDirection::Offered,
            amount_msat: 3_000_000,
            payment_hash: Sha256::hash(&[0x03; 32]).to_byte_array(),
            cltv_expiry: 503,
        },
        // HTLC #4 - remote->local (Received) 4000 msat, expiry 504
        Bolt3Htlc {
            direction: HtlcDirection::Received,
            amount_msat: 4_000_000,
            payment_hash: Sha256::hash(&[0x04; 32]).to_byte_array(),
            cltv_expiry: 504,
        },
    ];

    let commitment_tx = build_bolt3_commitment_with_htlcs(&test_vector, htlcs);

    // Expected values from BOLT3 test vectors
    let expected_num_outputs = 7;
    let expected_output_values = vec![
        1000,    // Output 0: HTLC #0 (received 1000)
        2000,    // Output 1: HTLC #2 (offered 2000)
        2000,    // Output 2: HTLC #1 (received 2000)
        3000,    // Output 3: HTLC #3 (offered 3000)
        4000,    // Output 4: HTLC #4 (received 4000)
        3000000, // Output 5: to_remote
        6988000, // Output 6: to_local
    ];

    // Build witness stack for the commitment transaction
    let commitment_witness = Witness::from_slice(&[
        &[][..],
        &local_funding_output_signature,
        &remote_funding_output_signature,
        &test_vector.funding_witness_script,
    ]);

    let mut signed_commitment_tx = commitment_tx.clone();
    signed_commitment_tx.input[0].witness = commitment_witness;

    // Expected transaction hex from BOLT3 vectors
    let expected_tx_hex = "02000000000101bef67e4e2fb9ddeeb3461973cd4c62abb35050b1add772995b820b584a488489000000000038b02b8007e80300000000000022002052bfef0479d7b293c27e0f1eb294bea154c63a3294ef092c19af51409bce0e2ad007000000000000220020403d394747cae42e98ff01734ad5c08f82ba123d3d9a620abda88989651e2ab5d007000000000000220020748eba944fedc8827f6b06bc44678f93c0f9e6078b35c6331ed31e75f8ce0c2db80b000000000000220020c20b5d1f8584fd90443e7b7b720136174fa4b9333c261d04dbbd012635c0f419a00f0000000000002200208c48d15160397c9731df9bc3b236656efb6665fbfe92b4a6878e88a499f741c4c0c62d0000000000160014cc1b07838e387deacd0e5232e1e8b49f4c29e484e0a06a00000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e040047304402206fc2d1f10ea59951eefac0b4b7c396a3c3d87b71ff0b019796ef4535beaf36f902201765b0181e514d04f4c8ad75659d7037be26cdb3f8bb6f78fe61decef484c3ea01473044022009b048187705a8cbc9ad73adbe5af148c3d012e1f067961486c822c7af08158c022006d66f3704cfab3eb2dc49dae24e4aa22a6910fc9b424007583204e3621af2e501475221023da092f6980e58d2c037173180e9a465476026ee50f96695963e8efe436f54eb21030e9f7b623d2ccc7c9bd44d66d5ce21ce504c0acf6385a132cec6d3c39fa711c152ae3e195220";

    let actual_tx_hex = encode::serialize_hex(&signed_commitment_tx);

    println!("\nTransaction hex comparison:");
    println!("Expected: {}", expected_tx_hex);
    println!("Actual:   {}", actual_tx_hex);

    // Verify output values
    for (i, (output, expected_value)) in signed_commitment_tx
        .output
        .iter()
        .zip(expected_output_values.iter())
        .enumerate()
    {
        assert_eq!(
            output.value.to_sat(),
            *expected_value,
            "Output {} value mismatch",
            i
        );
    }

    // Verify complete transaction hex
    assert_eq!(
        actual_tx_hex, expected_tx_hex,
        "Complete transaction hex should match BOLT3 test vectors"
    );

    // Now test the HTLC transactions that spend from the commitment transaction
    println!("\n=== Testing HTLC Transactions ===\n");

    // Expected HTLC transaction hexes from BOLT3 vectors
    let expected_htlc_txs = vec![
        // Output #0: HTLC-success for HTLC #0 (received 1000)
        (
            "htlc-success #0",
            "02000000000101ab84ff284f162cfbfef241f853b47d4368d171f9e2a1445160cd591c4c7d882b00000000000000000001e8030000000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e0500483045022100d9e29616b8f3959f1d3d7f7ce893ffedcdc407717d0de8e37d808c91d3a7c50d022078c3033f6d00095c8720a4bc943c1b45727818c082e4e3ddbc6d3116435b624b014730440220636de5682ef0c5b61f124ec74e8aa2461a69777521d6998295dcea36bc3338110220165285594b23c50b28b82df200234566628a27bcd17f7f14404bd865354eb3ce012000000000000000000000000000000000000000000000000000000000000000008a76a91414011f7254d96b819c76986c277d115efce6f7b58763ac67210394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b7c8201208763a914b8bcb07f6344b42ab04250c86a6e8b75d3fdbbc688527c21030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e752ae677502f401b175ac686800000000"
        ),
        // Output #1: HTLC-timeout for HTLC #2 (offered 2000)
        (
            "htlc-timeout #2",
            "02000000000101ab84ff284f162cfbfef241f853b47d4368d171f9e2a1445160cd591c4c7d882b01000000000000000001d0070000000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e05004730440220649fe8b20e67e46cbb0d09b4acea87dbec001b39b08dee7bdd0b1f03922a8640022037c462dff79df501cecfdb12ea7f4de91f99230bb544726f6e04527b1f89600401483045022100803159dee7935dba4a1d36a61055ce8fd62caa528573cc221ae288515405a252022029c59e7cffce374fe860100a4a63787e105c3cf5156d40b12dd53ff55ac8cf3f01008576a91414011f7254d96b819c76986c277d115efce6f7b58763ac67210394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b7c820120876475527c21030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e752ae67a914b43e1b38138a41b37f7cd9a1d274bc63e3a9b5d188ac6868f6010000"
        ),
        // Output #2: HTLC-success for HTLC #1 (received 2000)
        (
            "htlc-success #1",
            "02000000000101ab84ff284f162cfbfef241f853b47d4368d171f9e2a1445160cd591c4c7d882b02000000000000000001d0070000000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e05004730440220770fc321e97a19f38985f2e7732dd9fe08d16a2efa4bcbc0429400a447faf49102204d40b417f3113e1b0944ae0986f517564ab4acd3d190503faf97a6e420d4335201483045022100a437cc2ce77400ecde441b3398fea3c3ad8bdad8132be818227fe3c5b8345989022069d45e7fa0ae551ec37240845e2c561ceb2567eacf3076a6a43a502d05865faa012001010101010101010101010101010101010101010101010101010101010101018a76a91414011f7254d96b819c76986c277d115efce6f7b58763ac67210394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b7c8201208763a9144b6b2e5444c2639cc0fb7bcea5afba3f3cdce23988527c21030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e752ae677502f501b175ac686800000000"
        ),
        // Output #3: HTLC-timeout for HTLC #3 (offered 3000)
        (
            "htlc-timeout #3",
            "02000000000101ab84ff284f162cfbfef241f853b47d4368d171f9e2a1445160cd591c4c7d882b03000000000000000001b80b0000000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e050047304402207bcbf4f60a9829b05d2dbab84ed593e0291836be715dc7db6b72a64caf646af802201e489a5a84f7c5cc130398b841d138d031a5137ac8f4c49c770a4959dc3c13630147304402203121d9b9c055f354304b016a36662ee99e1110d9501cb271b087ddb6f382c2c80220549882f3f3b78d9c492de47543cb9a697cecc493174726146536c5954dac748701008576a91414011f7254d96b819c76986c277d115efce6f7b58763ac67210394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b7c820120876475527c21030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e752ae67a9148a486ff2e31d6158bf39e2608864d63fefd09d5b88ac6868f7010000"
        ),
        // Output #4: HTLC-success for HTLC #4 (received 4000)
        (
            "htlc-success #4",
            "02000000000101ab84ff284f162cfbfef241f853b47d4368d171f9e2a1445160cd591c4c7d882b04000000000000000001a00f0000000000002200204adb4e2f00643db396dd120d4e7dc17625f5f2c11a40d857accc862d6b7dd80e0500473044022076dca5cb81ba7e466e349b7128cdba216d4d01659e29b96025b9524aaf0d1899022060de85697b88b21c749702b7d2cfa7dfeaa1f472c8f1d7d9c23f2bf968464b8701483045022100d9080f103cc92bac15ec42464a95f070c7fb6925014e673ee2ea1374d36a7f7502200c65294d22eb20d48564954d5afe04a385551919d8b2ddb4ae2459daaeee1d95012004040404040404040404040404040404040404040404040404040404040404048a76a91414011f7254d96b819c76986c277d115efce6f7b58763ac67210394854aa6eab5b2a8122cc726e9dded053a2184d88256816826d6231c068d4a5b7c8201208763a91418bc1a114ccf9c052d3d23e28d3b0a9d1227434288527c21030d417a46946384f88d5f3337267c5e579765875dc4daca813e21734b140639e752ae677502f801b175ac686800000000"
        ),
    ];

    // Get commitment transaction TXID for HTLC inputs
    let commitment_txid = commitment_tx.compute_txid();

    // Get the commitment keys for HTLC transactions
    let secp = Secp256k1::new();
    let channel_keys = ChannelKeyManager {
        funding_key: test_vector.local_funding_privkey.clone(),
        revocation_basepoint_secret: test_vector.local_revocation_basepoint_secret.clone(),
        payment_basepoint_secret: test_vector.local_payment_basepoint_secret.clone(),
        delayed_payment_basepoint_secret: test_vector.local_delayed_payment_basepoint_secret.clone(),
        htlc_basepoint_secret: test_vector.local_htlc_basepoint_secret.clone(),
        commitment_seed: test_vector.commitment_seed,
        secp_ctx: secp.clone(),
    };

    // Derive commitment keys
    let commitment_keys = CommitmentKeys::from_keys(
        test_vector.local_revocation_pubkey,
        test_vector.local_revocation_pubkey,
        test_vector.local_delayedpubkey,
        test_vector.local_htlcpubkey,
        test_vector.remote_htlcpubkey,
    );

    // Derive revocation key for HTLC scripts
    let revocation_pubkey = test_vector.local_revocation_pubkey.clone();

    // Build and sign HTLC #0 (received 1000) - htlc-success
    let htlc_0_payment_hash = Sha256::hash(&[0u8; 32]).to_byte_array();
    let htlc_0_script = create_received_htlc_script(
        &revocation_pubkey,
        &commitment_keys.local_htlc_key,
        &commitment_keys.remote_htlc_key,
        &htlc_0_payment_hash,
        500,
    );
    let mut htlc_0_tx = create_htlc_success_transaction(
        OutPoint::new(commitment_txid, 0),
        1000,
        &commitment_keys,
        test_vector.local_delay,
        test_vector.feerate_per_kw,
    );
    let mut htlc_0_remote_sig = hex::decode("3045022100d9e29616b8f3959f1d3d7f7ce893ffedcdc407717d0de8e37d808c91d3a7c50d022078c3033f6d00095c8720a4bc943c1b45727818c082e4e3ddbc6d3116435b624b").unwrap();
    let mut htlc_0_local_sig = hex::decode("30440220636de5682ef0c5b61f124ec74e8aa2461a69777521d6998295dcea36bc3338110220165285594b23c50b28b82df200234566628a27bcd17f7f14404bd865354eb3ce").unwrap();
    htlc_0_remote_sig.push(0x01);
    htlc_0_local_sig.push(0x01);
    let htlc_0_preimage = [0u8; 32];
    htlc_0_tx.input[0].witness = create_htlc_success_witness(
        htlc_0_remote_sig,
        htlc_0_local_sig,
        htlc_0_preimage,
        &htlc_0_script,
    );

    // Build and sign HTLC #2 (offered 2000) - htlc-timeout
    let htlc_2_payment_hash = Sha256::hash(&[0x02; 32]).to_byte_array();
    let htlc_2_script = create_offered_htlc_script(
        &revocation_pubkey,
        &commitment_keys.local_htlc_key,
        &commitment_keys.remote_htlc_key,
        &htlc_2_payment_hash,
    );
    let mut htlc_2_tx = create_htlc_timeout_transaction(
        OutPoint::new(commitment_txid, 1),
        2000,
        502,
        &commitment_keys,
        test_vector.local_delay,
        test_vector.feerate_per_kw,
    );
    let mut htlc_2_remote_sig = hex::decode("30440220649fe8b20e67e46cbb0d09b4acea87dbec001b39b08dee7bdd0b1f03922a8640022037c462dff79df501cecfdb12ea7f4de91f99230bb544726f6e04527b1f896004").unwrap();
    let mut htlc_2_local_sig = hex::decode("3045022100803159dee7935dba4a1d36a61055ce8fd62caa528573cc221ae288515405a252022029c59e7cffce374fe860100a4a63787e105c3cf5156d40b12dd53ff55ac8cf3f").unwrap();
    htlc_2_remote_sig.push(0x01);
    htlc_2_local_sig.push(0x01);
    htlc_2_tx.input[0].witness =
        create_htlc_timeout_witness(htlc_2_remote_sig, htlc_2_local_sig, &htlc_2_script);

    // Build and sign HTLC #1 (received 2000) - htlc-success
    let htlc_1_payment_hash = Sha256::hash(&[0x01; 32]).to_byte_array();
    let htlc_1_script = create_received_htlc_script(
        &revocation_pubkey,
        &commitment_keys.local_htlc_key,
        &commitment_keys.remote_htlc_key,
        &htlc_1_payment_hash,
        501,
    );
    let mut htlc_1_tx = create_htlc_success_transaction(
        OutPoint::new(commitment_txid, 2),
        2000,
        &commitment_keys,
        test_vector.local_delay,
        test_vector.feerate_per_kw,
    );
    let mut htlc_1_remote_sig = hex::decode("30440220770fc321e97a19f38985f2e7732dd9fe08d16a2efa4bcbc0429400a447faf49102204d40b417f3113e1b0944ae0986f517564ab4acd3d190503faf97a6e420d43352").unwrap();
    let mut htlc_1_local_sig = hex::decode("3045022100a437cc2ce77400ecde441b3398fea3c3ad8bdad8132be818227fe3c5b8345989022069d45e7fa0ae551ec37240845e2c561ceb2567eacf3076a6a43a502d05865faa").unwrap();
    htlc_1_remote_sig.push(0x01);
    htlc_1_local_sig.push(0x01);
    let htlc_1_preimage = [0x01; 32];
    htlc_1_tx.input[0].witness = create_htlc_success_witness(
        htlc_1_remote_sig,
        htlc_1_local_sig,
        htlc_1_preimage,
        &htlc_1_script,
    );

    // Build and sign HTLC #3 (offered 3000) - htlc-timeout
    let htlc_3_payment_hash = Sha256::hash(&[0x03; 32]).to_byte_array();
    let htlc_3_script = create_offered_htlc_script(
        &revocation_pubkey,
        &commitment_keys.local_htlc_key,
        &commitment_keys.remote_htlc_key,
        &htlc_3_payment_hash,
    );
    let mut htlc_3_tx = create_htlc_timeout_transaction(
        OutPoint::new(commitment_txid, 3),
        3000,
        503,
        &commitment_keys,
        test_vector.local_delay,
        test_vector.feerate_per_kw,
    );
    let mut htlc_3_remote_sig = hex::decode("304402207bcbf4f60a9829b05d2dbab84ed593e0291836be715dc7db6b72a64caf646af802201e489a5a84f7c5cc130398b841d138d031a5137ac8f4c49c770a4959dc3c1363").unwrap();
    let mut htlc_3_local_sig = hex::decode("304402203121d9b9c055f354304b016a36662ee99e1110d9501cb271b087ddb6f382c2c80220549882f3f3b78d9c492de47543cb9a697cecc493174726146536c5954dac7487").unwrap();
    htlc_3_remote_sig.push(0x01);
    htlc_3_local_sig.push(0x01);
    htlc_3_tx.input[0].witness =
        create_htlc_timeout_witness(htlc_3_remote_sig, htlc_3_local_sig, &htlc_3_script);

    // Build and sign HTLC #4 (received 4000) - htlc-success
    let htlc_4_payment_hash = Sha256::hash(&[0x04; 32]).to_byte_array();
    let htlc_4_script = create_received_htlc_script(
        &revocation_pubkey,
        &commitment_keys.local_htlc_key,
        &commitment_keys.remote_htlc_key,
        &htlc_4_payment_hash,
        504,
    );
    let mut htlc_4_tx = create_htlc_success_transaction(
        OutPoint::new(commitment_txid, 4),
        4000,
        &commitment_keys,
        test_vector.local_delay,
        test_vector.feerate_per_kw,
    );
    let mut htlc_4_remote_sig = hex::decode("3044022076dca5cb81ba7e466e349b7128cdba216d4d01659e29b96025b9524aaf0d1899022060de85697b88b21c749702b7d2cfa7dfeaa1f472c8f1d7d9c23f2bf968464b87").unwrap();
    let mut htlc_4_local_sig = hex::decode("3045022100d9080f103cc92bac15ec42464a95f070c7fb6925014e673ee2ea1374d36a7f7502200c65294d22eb20d48564954d5afe04a385551919d8b2ddb4ae2459daaeee1d95").unwrap();
    htlc_4_remote_sig.push(0x01);
    htlc_4_local_sig.push(0x01);
    let htlc_4_preimage = [0x04; 32];
    htlc_4_tx.input[0].witness = create_htlc_success_witness(
        htlc_4_remote_sig,
        htlc_4_local_sig,
        htlc_4_preimage,
        &htlc_4_script,
    );

    // Collect built transactions with names
    let built_htlc_txs = vec![
        ("htlc-success #0", htlc_0_tx),
        ("htlc-timeout #2", htlc_2_tx),
        ("htlc-success #1", htlc_1_tx),
        ("htlc-timeout #3", htlc_3_tx),
        ("htlc-success #4", htlc_4_tx),
    ];

    println!("\nHTLC Transaction Verification:");
    let mut all_match = true;
    for ((name, built_tx), (_, expected_hex)) in built_htlc_txs.iter().zip(expected_htlc_txs.iter())
    {
        let built_hex = encode::serialize_hex(built_tx);
        let matches = built_hex == *expected_hex;
        all_match = all_match && matches;

        let status = if matches { "✓" } else { "✗" };
        println!("\n{} {}", name, status);
        if !matches {
            println!("  Expected: {}", expected_hex);
            println!("  Built:    {}", built_hex);
        } else {
            println!("  Transaction matches BOLT3 vectors perfectly!");
        }
    }

    // Assert all HTLC transactions match
    assert!(
        all_match,
        "All HTLC transactions should match BOLT3 test vectors"
    );

    println!("\n✓ Commitment transaction verified against BOLT3 vectors!");
    println!("✓ All 5 HTLC transactions verified against BOLT3 vectors!");
    println!("✓ HTLC-success witness structure correct (with payment preimage)!");
    println!("✓ HTLC-timeout witness structure correct (with timeout path)!");
    println!("\n✓ Complete BOLT3 test vector validation passed!");
}

#[test]
fn test_bolt3_output_ordering() {
    println!("\n=== Testing: BOLT 3 Output Ordering ===\n");

    let mut outputs = vec![
        OutputWithMetadata {
            value: 3000,
            script: ScriptBuf::from_hex("0014aaaa").unwrap(),
            cltv_expiry: None,
        },
        OutputWithMetadata {
            value: 1000,
            script: ScriptBuf::from_hex("0014bbbb").unwrap(),
            cltv_expiry: None,
        },
        OutputWithMetadata {
            value: 2000,
            script: ScriptBuf::from_hex("0014cccc").unwrap(),
            cltv_expiry: None,
        },
    ];

    println!("Before sorting:");
    for (i, output) in outputs.iter().enumerate() {
        println!("  Output {}: {} sats", i, output.value);
    }

    sort_outputs(&mut outputs);

    println!("\nAfter sorting:");
    for (i, output) in outputs.iter().enumerate() {
        println!("  Output {}: {} sats", i, output.value);
    }

    assert_eq!(outputs[0].value, 1000);
    assert_eq!(outputs[1].value, 2000);
    assert_eq!(outputs[2].value, 3000);

    println!("\n✓ Output ordering verified!");
}

#[test]
fn test_bolt3_obscured_commitment_number() {
    println!("\n=== Testing: Obscured Commitment Number ===\n");

    let local_payment_basepoint = PublicKey::from_slice(
        &hex::decode("034f355bdcb7cc0af728ef3cceb9615d90684bb5b2ca5f859ab0f0b704075871aa").unwrap(),
    )
    .unwrap();

    let remote_payment_basepoint = PublicKey::from_slice(
        &hex::decode("032c0b7cf95324a07d05398b240174dc0c2be444d96b159aa6c7f7b1e668680991").unwrap(),
    )
    .unwrap();

    let commitment_number = 42u64;
    let expected_obscured = 0x2bb038521914u64 ^ 42;

    let commitment_transaction_number_obscure_factor =
        get_commitment_transaction_number_obscure_factor(
            &local_payment_basepoint,
            &remote_payment_basepoint,
        );

    let actual_obscured = commitment_transaction_number_obscure_factor
        ^ commitment_number;

    println!("Commitment number: {}", commitment_number);
    println!("Expected obscured: 0x{:012x}", expected_obscured);
    println!("Actual obscured:   0x{:012x}", actual_obscured);

    assert_eq!(actual_obscured, expected_obscured);

    println!("\n✓ Obscured commitment number matches!");
}

#[test]
fn test_bolt3_to_local_script() {
    println!("\n=== Testing: to_local Script Generation ===\n");

    let revocation_pubkey = PublicKey::from_slice(
        &hex::decode("0212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b19").unwrap(),
    )
    .unwrap();

    let local_delayedpubkey = PublicKey::from_slice(
        &hex::decode("03fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c").unwrap(),
    )
    .unwrap();

    let to_self_delay = 144u16;

    let script = create_to_local_script(&revocation_pubkey, &local_delayedpubkey, to_self_delay);

    let expected_script = "63210212a140cd0c6539d07cd08dfe09984dec3251ea808b892efeac3ede9402bf2b1967029000b2752103fd5960528dc152014952efdb702a88f71e3c1653b2314431701ec77e57fde83c68ac";

    println!("Expected script: {}", expected_script);
    println!("Actual script:   {}", hex::encode(script.as_bytes()));

    assert_eq!(hex::encode(script.as_bytes()), expected_script);

    println!("\n✓ to_local script matches!");
}