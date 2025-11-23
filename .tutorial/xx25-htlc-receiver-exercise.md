# ⚡️ Create Received HTLC Script

We're almost finished coding our BOLT 3-compliant Lightning implementation. Let's bring it home by building our **received HTLC script**!

For this exercise, head back to `src/exercises/scripts/htlc.rs`. In this file, you'll find the `create_received_htlc_script` function, which takes the following inputs:
- `revocation_pubkey`: Our **Revocation Public Key**, which is created by combining our counterparty's **Revocation Basepoint** with our **Per-Commitment Point**.
- `local_htlcpubkey`: Our **HTLC Public Key**, which is derived from our **HTLC Basepoint** and our **Per-Commitment Point**.
- `remote_htlcpubkey`:Our counterparty's **HTLC Public Key**, which is derived from their **HTLC Basepoint** and our **Per-Commitment Point**.
- `payment_hash`: The hash of the payment preimage.

```rust
pub fn create_received_htlc_script(
    revocation_pubkey: &PublicKey,
    local_htlcpubkey: &PublicKey,
    remote_htlcpubkey: &PublicKey,
    payment_hash: &[u8; 32],
    cltv_expiry: u32,
) -> ScriptBuf {


    let payment_hash160 = Ripemd160::hash(payment_hash).to_byte_array();
    let revocation_pubkey_hash = PubkeyHash::hash(&revocation_pubkey.serialize());

    let script = Builder::new()
        .push_opcode(opcodes::OP_DUP)
        .push_opcode(opcodes::OP_HASH160)
        .push_slice(&revocation_pubkey_hash)
        .push_opcode(opcodes::OP_EQUAL)
        .push_opcode(opcodes::OP_IF)
        .push_opcode(opcodes::OP_CHECKSIG)
        .push_opcode(opcodes::OP_ELSE)
        .push_slice(remote_htlcpubkey.serialize())
        .push_opcode(opcodes::OP_SWAP)
        .push_opcode(opcodes::OP_SIZE)
        .push_int(32)
        .push_opcode(opcodes::OP_EQUAL)
        .push_opcode(opcodes::OP_IF)
        .push_opcode(opcodes::OP_HASH160)
        .push_slice(payment_hash160)
        .push_opcode(opcodes::OP_EQUALVERIFY)
        .push_int(2)
        .push_opcode(opcodes::OP_SWAP)
        .push_slice(local_htlcpubkey.serialize())
        .push_int(2)
        .push_opcode(opcodes::OP_CHECKMULTISIG)
        .push_opcode(opcodes::OP_ELSE)
        .push_opcode(opcodes::OP_DROP)
        .push_int(cltv_expiry as i64)
        .push_opcode(opcodes::OP_CLTV)
        .push_opcode(opcodes::OP_DROP)
        .push_opcode(opcodes::OP_CHECKSIG)
        .push_opcode(opcodes::OP_ENDIF)
        .push_opcode(opcodes::OP_ENDIF)
        .into_script();

    script
}
```
<details>
  <summary>Step 1: Prepare the Hash Values</summary>

Just like we did with the HTLC Offerer script, let's start by preparing two hash values that will be used in the script. The first is the RIPEMD160 of the payment (preimage) hash. The second is the public key hash of the **Revocation Public Key**.

```rust
let payment_hash160 = Ripemd160::hash(payment_hash).to_byte_array();
let revocation_pubkey_hash = PubkeyHash::hash(&revocation_pubkey.serialize());
```

</details>

<details>
  <summary>Step 2: Start the Revocation Check</summary>

The revocation path is identical to the offered HTLC. 

We first check if the provided value is equal to the hash of the **Revocation Public Key**. To do this, we use `DUP HASH160 <hash> EQUAL` to check if the two data elements are equal.

```rust
Builder::new()
    .push_opcode(opcodes::OP_DUP)
    .push_opcode(opcodes::OP_HASH160)
    .push_slice(&revocation_pubkey_hash)
    .push_opcode(opcodes::OP_EQUAL)
    .push_opcode(opcodes::OP_IF)
    .push_opcode(opcodes::OP_CHECKSIG)
    .push_opcode(opcodes::OP_ELSE)
```

</details>

<details>
  <summary>Step 3: Set Up Success vs Timeout Logic</summary>

Okay, here's where received HTLC script starts to differ from offered HTLCs!

We still check the witness element size, but the logic is flipped: if the witness element is exactly 32 bytes (preimge), we take the IF branch. Otherwise, we execute the Else branch.

```rust
.push_slice(remote_htlcpubkey.serialize())
.push_opcode(opcodes::OP_SWAP)
.push_opcode(opcodes::OP_SIZE)
.push_int(32)
.push_opcode(opcodes::OP_EQUAL)
.push_opcode(opcodes::OP_IF)
```

</details>

<details>
  <summary>Step 4: Handle the Success Path (2-of-2 Multisig with Preimage)</summary>

For received HTLCs, we claim the payment (success path) by providing the preimage **and** the signatures required to spend from the 2-of-2 multisig.

```rust
.push_opcode(opcodes::OP_HASH160)
.push_slice(payment_hash160)
.push_opcode(opcodes::OP_EQUALVERIFY)
.push_int(2)
.push_opcode(opcodes::OP_SWAP)
.push_slice(local_htlcpubkey.serialize())
.push_int(2)
.push_opcode(opcodes::OP_CHECKMULTISIG)
.push_opcode(opcodes::OP_ELSE)
```

Breaking this down:
- `HASH160` hashes the 32-byte preimage
- We verify it matches our stored payment hash
- Then we require 2-of-2 signatures from both parties' HTLC keys

</details>

<details>
  <summary>Step 5: Handle the Timeout Path (with CLTV)</summary>

If there's no preimage, then the counterparty (Alice) can simply reclaim their funds after the CLTV expiry.

```rust
.push_opcode(opcodes::OP_DROP)
.push_int(cltv_expiry as i64)
.push_opcode(opcodes::OP_CLTV)
.push_opcode(opcodes::OP_DROP)
.push_opcode(opcodes::OP_CHECKSIG)
.push_opcode(opcodes::OP_ENDIF)
```

Below is a breakdown of what's going on:
- `DROP` removes the size value from the stack
- We push the `cltv_expiry` block height
- `OP_CLTV` enforces that this path can only be taken after that block height
- We `DROP` the timelock value (CLTV doesn't consume it)
- `CHECKSIG` verifies the signature using the remote HTLC pubkey (already on stack from Step 3)
- `ENDIF` closes the inner IF/ELSE (success vs timeout)

</details>

<details>
  <summary>Step 6: Close the Outer Conditional</summary>

Finally, we close the outer IF/ELSE structure.

```rust
.push_opcode(opcodes::OP_ENDIF)
.into_script();
```

</details>


# ⚡️ Create HTLC Success Transaction

Next up, let's build the **HTLC Success Transaction**! As we just learned, this is the counterpart to the HTLC Timeout Transaction - it enables Bob to claim a received HTLC when he obtains the payment preimage.

For this exercise, head over to `src/exercises/transactions/htlc/rs`.


The `create_htlc_success_transaction` takes the following parameters:
- `htlc_outpoint`: The outpoint (txid + vout) of the HTLC output we're spending from. 
- `htlc_amount`: The amount locked in the HTLC (in satoshis).
- `cltv_expiry`: The absolute block height when this HTLC expires.
- `local_keys`: Our commitment keys. See the dropdown below for more information.
- `to_self_delay`: The number of blocks that we must wait before we can claim our funds using the **Delayed Payment Public Key** path.
- `feerate_per_kw`: The fee rate in satoshis per 1000 weight units.

```rust
pub fn create_htlc_success_transaction(
    htlc_outpoint: OutPoint,
    htlc_amount: u64,
    local_keys: &CommitmentKeys,
    to_self_delay: u16,
    feerate_per_kw: u64,
) -> Transaction {
    let fee = calculate_htlc_success_tx_fee(feerate_per_kw);
    let output_amount = htlc_amount.saturating_sub(fee);

    let secp = Secp256k1::new();

    // Create to_local script
    let to_local_script = create_to_local_script(
        &local_keys.revocation_key,
        &local_keys.local_delayed_payment_key,
        to_self_delay,
    );

    let tx_in = TxIn {
            previous_output: htlc_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ZERO,
            witness: Witness::new(),
        };

    let tx_out = TxOut {
            value: Amount::from_sat(output_amount),
            script_pubkey: to_local_script.to_p2wsh(),
        };

    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![tx_in],
        output: vec![tx_out],
    }

}
```

<details>
  <summary>Step 1: Calculate Fees and Output Amount</summary>

Just like we did for the **HTLC Timeout Transaction**, we'll start by calculating the fee for the **HTLC Success Transaction**. However, we'll use a different function this time, as the **HTLC Success Transaction** will be larger since it includes a preimage in the witness.

A helper function, `calculate_htlc_success_tx_fee` is available to use for this exercise. You can see the function definition below or view it in `src/exercises/transactions/fees.rs`.

Once we have the fee for this transaction, which depends on the feerate, we'll determine the output amount by subtracting it from the `htlc_amount`. 

```rust
let fee = calculate_htlc_tx_fee(feerate_per_kw);
let output_amount = htlc_amount.saturating_sub(fee);
```

</details>

<details>
  <summary>Step 2: Create the to_local Output Script</summary>

Similar to the **HTLC Timeout Transaction**, the **HTLC Success Transaction** pays to a `to_local` script.

```rust
let to_local_script = create_to_local_script(
    &local_keys.revocation_key,
    &local_keys.local_delayed_payment_key,
    to_self_delay,
);
```

</details>

<details>
  <summary>Step 3: Create the Transaction Input</summary>

Next, let's define our HTLC Success input! Similar to the Timeout Transaction, we'll keep it unsigned, so we just need to create a `TxIn` object.

```rust
let tx_in = TxIn {
        previous_output: htlc_outpoint,
        script_sig: ScriptBuf::new(),
        sequence: Sequence::ZERO,
        witness: Witness::new(),
    };
```
</details>

<details>
  <summary>Step 4: Create the Transaction Output</summary>

Now, let's create a `TxOut` object. Remember to cast the `ScriptBuf` to a P2WSH output (`.to_p2wsh()`)!
```rust
let tx_out = TxOut {
        value: Amount::from_sat(output_amount),
        script_pubkey: to_local_script.to_p2wsh(),
    };
```

</details>

<details>
  <summary>Step 5: Assemble the Complete Transaction</summary>

Go ahead and create the complete `Transaction`! The notable difference between this exercise and the Timeout Transaction is that there is no locktime expiry!

```rust
Transaction {
    version: Version::TWO,
    lock_time: LockTime::ZERO,
    input: vec![tx_in],
    output: vec![tx_out],
}
```

</details>


# ⚡️ Finalize HTLC Success Transaction

Nice, our HTLC Success functionality is almost done! Now, just like we did with our HTLC Timeout Transaction, we need to write code to generate our signature and build the witness for our HTLC Success Transaction.

For this exercise, we'll complete `finalize_htlc_success`, which takes the following parameters:
- `keys_manager`: Our Channel Keys Manager, which holds our HTLC Basepoint Secret and can generate signatures.
- `tx`: The unsigned HTLC Success Transaction we created earlier.
- `input_index`: The index of the input we're signing on the HTLC Success Transaction.
- `htlc_script`: The received HTLC script that we're spending from.
- `htlc_amount`: The amount in the HTLC output (needed for signature generation).
- `remote_htlc_signature`: Our counterparty's signature (pre-signed when the HTLC was created).

Go ahead and try implementing the function below! To successfully complete this exercise, you'll need to generate your (local) HTLC signture and then add the following witness to the transaction.

```
0 <remotehtlcsig> <localhtlcsig>  <payment_preimage>
```

```rust
pub fn finalize_htlc_success(
    keys_manager: ChannelKeyManager,
    tx: Transaction,
    input_index: usize,
    htlc_script: &ScriptBuf,
    htlc_amount: u64,
    remote_htlc_signature: Vec<u8>,
    payment_preimage: [u8; 32],
) -> Transaction {

    let local_htlc_privkey = keys_manager.htlc_base_key;

    let local_htlc_signature = keys_manager.sign_transaction_input(
        &tx,
        input_index,
        &htlc_script,
        htlc_amount,
        &local_htlc_privkey,
    );

    let witness = Witness::from_slice(&[
        &[][..],                        // OP_0 for CHECKMULTISIG bug
        &remote_htlc_signature[..],
        &local_htlc_signature[..],
        &payment_preimage[..],
    ]);

    let mut signed_tx = tx;
    signed_tx.input[0].witness = witness;

    signed_tx
}
```

<details>
  <summary>Step 1: Fetch the Local HTLC Private Key</summary>

Since we'll need to generate our own signature, using the **HTLC Basepoint Secret**, we'll need to start by fetching the secret from our `ChannelKeyManager`. You can click the dropdown below if you need a reminder of the `ChannelKeyManager` structure.

<details>
  <summary>ChannelKeyManager</summary>

```rust
pub struct ChannelKeyManager {
    pub funding_key: SecretKey,
    pub revocation_base_key: SecretKey,
    pub payment_base_key: SecretKey,
    pub delayed_payment_base_key: SecretKey,
    pub htlc_base_key: SecretKey,
    pub commitment_seed: [u8; 32],
    pub secp_ctx: Secp256k1<All>,
}
```

</details>


```rust
let local_htlc_privkey = keys_manager.htlc_base_key;
```

</details>

<details>
  <summary>Step 2: Sign the Transaction Input</summary>

Next, let's generate our signature for the HTLC offerer output on our commitment transaction. To do this, we can use the `sign_transaction_input` function we created earlier in this course.

<details>
  <summary>Click to see sign_transaction_input function definition </summary>

We implemented the `sign_transaction_input` function earlier in this course. You may not have implemented it *exactly* like the below example, which is okay! That said, here is an example implementation to help jog your memory as you complete this exercise.

```rust
pub fn sign_transaction_input(
    &self,
    tx: &Transaction,
    input_index: usize,
    script: &ScriptBuf,
    amount: u64,
    secret_key: &SecretKey,
) -> Vec<u8> {
    let mut sighash_cache = SighashCache::new(tx);

    let sighash = sighash_cache
        .p2wsh_signature_hash(
            input_index,
            script,
            Amount::from_sat(amount),
            EcdsaSighashType::All,
        )
        .expect("Valid sighash");

    let msg = Message::from_digest(sighash.to_byte_array());
    let sig = self.secp_ctx.sign_ecdsa(&msg, secret_key);

    let mut sig_bytes = sig.serialize_der().to_vec();
    sig_bytes.push(EcdsaSighashType::All as u8);
    sig_bytes
}
```

</details>

```rust
let local_htlc_signature = keys_manager.sign_transaction_input(
    &tx,
    input_index,
    &htlc_script,
    htlc_amount,
    &local_htlc_privkey,
);
```

</details>


<details>
  <summary>Step 3: Build the Witness Stack</summary>

Now, let's build the witness stack!

```rust
let witness = Witness::from_slice(&[
    &[][..],                        // OP_0 for CHECKMULTISIG bug
    &remote_htlc_signature[..],
    &local_htlc_signature[..],
    &payment_preimage[..],          // The 32-byte preimage for success path
    htlc_script.as_bytes(),
]);
```

Below is a breakdown of each element:

1. **Empty byte array (`&[][..]`)**: First, we need to add a dummy element to the stack (`OP_0`), since there is an `OP_CHECKMULTISIG` error that pops an extra item of the stack.

2. **Remote HTLC signature**: Next, we add our counterparty's pre-signed signature. Remember, they give this to use when we are setting up the HTLC!

3. **Local HTLC signature**: Then we add our signature, which we just created.

4. **Payment preimage (`&payment_preimage[..]`)**: This is the key difference from the witness we created for the Timeout Transaction! Since the preimage is exactly 32 bytes, it will cause the Bitcoin script interpreter to evaluate the success path (IF branch). The interpreter will then hash the preimage and check if it matches the payment hash.

```
OP_DUP OP_HASH160 <RIPEMD160(SHA256(revocationpubkey))> OP_EQUAL
OP_IF
    OP_CHECKSIG
OP_ELSE
    <remote_htlcpubkey> OP_SWAP OP_SIZE 32 OP_EQUAL
    OP_IF
        # To local node via HTLC-success transaction.
        OP_HASH160 <RIPEMD160(payment_hash)> OP_EQUALVERIFY
        2 OP_SWAP <local_htlcpubkey> 2 OP_CHECKMULTISIG
```

5. **HTLC script**: Finally, we provide the full received HTLC script.

</details>

<details>
  <summary>Step 4: Insert Witness into the Transaction</summary>

Lastly, we'll add the witness to the transaction's input.

Don't forget to return the signed transaction!

```rust
let mut signed_tx = tx;
signed_tx.input[0].witness = witness;

signed_tx
```

</details>