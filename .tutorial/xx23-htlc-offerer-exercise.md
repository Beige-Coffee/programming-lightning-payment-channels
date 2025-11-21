# ⚡️ Create Offered HTLC Script

Wow! We've come a long way! Now, let's build one of the most complex scripts in Lightning - the **offered HTLC script**! As we just learned, this script is used when offering an HTLC to your counterparty. 

For this exercise, head over to `src/exercises/scripts/htlc.rs`. In this file, you'll find the `create_offered_htlc_script` function, which takes the following inputs:
- `revocation_pubkey`: Our **Revocation Public Key**, which is created by combining our counterparty's **Revocation Basepoint** with our **Per-Commitment Point**.
- `local_htlcpubkey`: Our **HTLC Public Key**, which is derived from our **HTLC Basepoint** and our **Per-Commitment Point**.
- `remote_htlcpubkey`:Our counterparty's **HTLC Public Key**, which is derived from their **HTLC Basepoint** and our **Per-Commitment Point**.
- `payment_hash`: The hash of the payment preimage.

```rust
pub fn create_offered_htlc_script(
    revocation_pubkey: &PublicKey,
    local_htlcpubkey: &PublicKey,
    remote_htlcpubkey: &PublicKey,
    payment_hash: &[u8; 32],
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
        .push_opcode(opcodes::OP_NOTIF)
        .push_opcode(opcodes::OP_DROP)
        .push_int(2)
        .push_opcode(opcodes::OP_SWAP)
        .push_slice(&local_htlcpubkey.serialize())
        .push_int(2)
        .push_opcode(opcodes::OP_CHECKMULTISIG)
        .push_opcode(opcodes::OP_ELSE)
        .push_opcode(opcodes::OP_HASH160)
        .push_slice(&payment_hash160)
        .push_opcode(opcodes::OP_EQUALVERIFY)
        .push_opcode(opcodes::OP_CHECKSIG)
        .push_opcode(opcodes::OP_ENDIF)
        .push_opcode(opcodes::OP_ENDIF)
        .into_script();
    
    script
}
```
<details>
  <summary>Step 1: Prepare the Hash Values</summary>

We'll start by preparing two hash values that will be used in the script. The first is the RIPEMD160 of the payment (preimage) hash. The second is the public key hash of the **Revocation Public Key**.

```rust
let payment_hash160 = Ripemd160::hash(payment_hash).to_byte_array();
let revocation_pubkey_hash = PubkeyHash::hash(&revocation_pubkey.serialize());
```

</details>

<details>
  <summary>Step 2: Start the Revocation Check</summary>

Just as we did earlier, we begin the proces of creating a `ScriptBuf` by using the `Builder` object in Rust Bitcoin.

The HTLC offerer script begins by checking if the provided value is equal to the hash of the **Revocation Public Key**. To do this, we use `DUP HASH160 <hash> EQUAL` to check if the two data elements are equal.

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

If the data provided (when hashed) is not equal to the **Hashed Revocation Public Key**, then we need to determine if this is a success spend (with preimage) or a timeout spend. We can do this by checking the size of the witness element.

If it's equal to 32, then we know it's a preimage! If it not, then it's a signature (~71-73 bytes), and we'll want to execute the timeout path.

```rust
.push_slice(remote_htlcpubkey.serialize())
.push_opcode(opcodes::OP_SWAP)
.push_opcode(opcodes::OP_SIZE)
.push_int(32)
.push_opcode(opcodes::OP_EQUAL)
.push_opcode(opcodes::OP_NOTIF)
```

</details>

<details>
  <summary>Step 4: Handle the Timeout Path (2-of-2 Multisig)</summary>

As we learned earlier, the timeout path requires both parties to cooperate using a 2-of-2 multisig, ensuring that the HTLC offerer (Alice) is unable to expire the HTLC early.

```rust
.push_opcode(opcodes::OP_DROP)
.push_int(2)
.push_opcode(opcodes::OP_SWAP)
.push_slice(&local_htlcpubkey.serialize())
.push_int(2)
.push_opcode(opcodes::OP_CHECKMULTISIG)
.push_opcode(opcodes::OP_ELSE)
```

Breaking this down:
- `DROP` removes the size value from the stack
- We push `2` (number of signatures required)
- `SWAP` rearranges the stack for CHECKMULTISIG
- We push the local HTLC pubkey
- We push `2` again (total number of public keys)
- `CHECKMULTISIG` verifies we have 2 valid signatures from the 2 pubkeys

</details>

<details>
  <summary>Step 5: Handle the Success Path (with Preimage)</summary>

Finally, if the witness element provided was exactly 32 bytes, then we execute the success path:

```rust
.push_opcode(opcodes::OP_HASH160)
.push_slice(&payment_hash160)
.push_opcode(opcodes::OP_EQUALVERIFY)
.push_opcode(opcodes::OP_CHECKSIG)
.push_opcode(opcodes::OP_ENDIF)
```

Here's the flow:
- `HASH160` hashes the 32-byte preimage (SHA256 + RIPEMD160)
- We compare it to our stored payment hash
- `EQUALVERIFY` checks they match and fails if not
- `CHECKSIG` verifies the signature using the remote HTLC pubkey (already on stack from Step 3)
- `ENDIF` closes the inner IF/ELSE (success vs timeout)

</details>

<details>
  <summary>Step 6: Close the Outer Conditional</summary>

We'll finish things up by closing the outer IF/ELSE structure, which separated revocation path from the other paths:

```rust
.push_opcode(opcodes::OP_ENDIF)
.into_script()
```

</details>


# ⚡️ Create HTLC Timeout Transaction

Next up, let's build the **HTLC Timeout Transaction**! Remember, this transaction enables the **HTLC offerer** to claim back their funds after the HTLC times out.

For this exercise, head over to `src/exercises/transactions/htlc/rs`.


The `create_htlc_timeout_transaction` takes the following parameters:
- `htlc_outpoint`: The outpoint (txid + vout) of the HTLC output we're spending from. 
- `htlc_amount`: The amount locked in the HTLC (in satoshis).
- `cltv_expiry`: The absolute block height when this HTLC expires.
- `local_keys`: Our commitment keys. See the dropdown below for more information.
- `to_self_delay`: The number of blocks that we must wait before we can claim our funds using the **Delayed Payment Public Key** path.
- `feerate_per_kw`: The fee rate in satoshis per 1000 weight units.

```rust
pub fn create_htlc_timeout_transaction(
    htlc_outpoint: OutPoint,
    htlc_amount: u64,
    cltv_expiry: u32,
    local_keys: &CommitmentKeys,
    to_self_delay: u16,
    feerate_per_kw: u64,
) -> Transaction {
    let fee = calculate_htlc_tx_fee(feerate_per_kw);
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
        lock_time: LockTime::from_consensus(cltv_expiry),
        input: vec![tx_in],
        output: vec![tx_out],
    }
}
```


<details>
  <summary>Step 1: Calculate Fees and Output Amount</summary>

First, we'll need to calculate the transaction fee and determine how many bitcoin will go into the output. Remember, this course has been focusing on commitment transactions that do not support anchor outputs or zero-fee commitments, so we'll need to deduce the fee from the amount.

If you recall from earlier, we learned that **HTLC Timeout Transactions** have a fixed weight of 663, which you can also confirm in the [Fees](https://github.com/lightning/bolts/blob/master/03-transactions.md#fees) section of BOLT 3. Now that we've HTLC Success Transactions, this should make intuitive sense, as the size of the transaction will not change - regardless of how many bitcoin are being sent or who is sending them!

A helper function, `calculate_htlc_timeout_tx_fee` is available to use for this exercise. You can see the function definition below or view it in `src/exercises/transactions/fees.rs`.

Once we have the fee for this transaction, which depends on the feerate, we'll determine the output amount by subtracting it from the `htlc_amount`. Here, we're using`saturating_sub`, as this prevents underflow. In other words, if the fee were larger than the HTLC amount, we'd get 0 instead of a panic. That said, in practice, this should not happen, as we would have "trimmed" this HTLC and not created an output for it.

```rust
let fee = calculate_htlc_tx_fee(feerate_per_kw);
let output_amount = htlc_amount.saturating_sub(fee);
```

<details>
  <summary>Click to see calculate_htlc_timeout_tx_fee</summary>

```rust
pub fn calculate_htlc_timeout_tx_fee(feerate_per_kw: u64) -> u64 {
    const HTLC_TX_WEIGHT: u64 = 663;
    (feerate_per_kw * HTLC_TX_WEIGHT) / 1000
}
```

</details>

</details>

<details>
  <summary>Step 2: Create the to_local Output Script</summary>

Remember, the timeout transaction contains the same `to_local` script as our commitment transaction! This way, we can ensure that our counterparty has a way to claim the funds if we attempt to cheat in the future by publishing this state (assuming we've moved on and this state is now old). 

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

Next, let's define our HTLC Timeout input! For now, we'll keep it unsigned, so we just need to create a `TxIn` object, using Rust Bitcoin, and specify the     `htlc_outpoint` as our `previous_output`.

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

Moving along, let's create a `TxOut` object, which we can do by specifying the amount and script pubkey. Remember, we can convert the script to a script pubkey by using the `to_p2wsh()` method on the `ScriptBuf`.

```rust
let tx_out = TxOut {
        value: Amount::from_sat(output_amount),
        script_pubkey: to_local_script.to_p2wsh(),
    };
```

We convert the `to_local_script` to P2WSH format using `.to_p2wsh()`. This means:
- The output contains a hash of the script
- When spending from this output, you'll need to provide the full script in the witness

</details>

<details>
  <summary>Step 5: Assemble the Complete Transaction</summary>

Finally, let's put it all together into a `Transaction`. Remember to account for the following:
1) Our non-anchor Lightning commitments will be version 2.
2) We need to set the `lock_time` field to the HTLC's `cltv_expiry` block height!

```rust
Transaction {
    version: Version::TWO,
    lock_time: LockTime::from_consensus(cltv_expiry),
    input: vec![tx_in],
    output: vec![tx_out],
}
```

</details>

# ⚡️ Finalize HTLC Timeout Transaction

Our HTLC Timeout functionality is almost fully implemented! There are just two important pieces left: generating our signature and building the witness. So, for this exercise, we'll tackle those two steps by building the `finalize_htlc_timeout` function.

This function takes the following parameters:
- `keys_manager`: Our Channel Keys Manager, which holds our HTLC Basepoint Secret and can generate signatures.
- `tx`: The unsigned HTLC timeout transaction we created earlier.
- `input_index`: The index of the input we're signing on the HTLC Timeout Transaction.
- `htlc_script`: The offered HTLC script that we're spending from.
- `htlc_amount`: The amount in the HTLC output (needed for signature generation).
- `remote_htlc_signature`: Our counterparty's signature (pre-signed when the HTLC was created).

Go ahead and try implementing the function below! To successfully complete this exercise, you'll need to generate your (local) HTLC signture and then add the following witness to the transaction.

```
0 <remotehtlcsig> <localhtlcsig> <> htlc_script
```

```rust
pub fn finalize_htlc_timeout(
    keys_manager: ChannelKeyManager,
    tx: Transaction,
    input_index: usize,
    htlc_script: &ScriptBuf,
    htlc_amount: u64,
    remote_htlc_signature: Vec<u8>,
) -> Transaction {

    let local_htlc_privkey = keys_manager.htlc_base_key;

    let local_htlc_signature = keys_manager.sign_transaction_input(
        &tx,
        input_index,
        &htlc_script,
        htlc_amount,
        &local_htlc_privkey,
    );

    // Build witness stack
    let witness = Witness::from_slice(&[
        &[][..],                        // OP_0 for CHECKMULTISIG bug
        &remote_htlc_signature[..],
        &local_htlc_signature[..],
        &[][..],                        // OP_FALSE for timeout path
        htlc_script.as_bytes(),
    ]);

    let mut signed_tx = tx;
    signed_tx.input[0].witness = witness;

    signed_tx
}
```

<details>
  <summary>Step 1: Get the Local HTLC Private Key</summary>

First, we need to retrieve our HTLC private key from the keys manager:
```rust
let local_htlc_privkey = keys_manager.htlc_base_key;
```

This is the private key that corresponds to our `local_htlcpubkey` that was used in the offered HTLC script. We'll use this to create our signature for the 2-of-2 multisig.

</details>

<details>
  <summary>Step 2: Sign the Transaction Input</summary>

Now we create our signature for this specific transaction input:
```rust
let local_htlc_signature = keys_manager.sign_transaction_input(
    &tx,
    input_index,
    &htlc_script,
    htlc_amount,
    &local_htlc_privkey,
);
```

The `sign_transaction_input` method handles all the complexity of creating a valid signature:
- It creates the sighash (the hash of the transaction data to be signed)
- Signs it with our private key
- Returns a DER-encoded signature with the appropriate SIGHASH flag

This signature proves we authorize spending from the HTLC output.

</details>

<details>
  <summary>Step 3: Build the Witness Stack</summary>

Now comes the critical part - building the witness stack in the exact order that the HTLC script expects:
```rust
let witness = Witness::from_slice(&[
    &[][..],                        // OP_0 for CHECKMULTISIG bug
    &remote_htlc_signature[..],
    &local_htlc_signature[..],
    &[][..],                        // OP_FALSE for timeout path
    htlc_script.as_bytes(),
]);
```

Let's break down each element:

1. **Empty byte array (`&[][..]`)**: This is the famous CHECKMULTISIG bug workaround. Due to an off-by-one error in Bitcoin's CHECKMULTISIG implementation, it pops one extra element from the stack. We provide a dummy OP_0 to satisfy this.

2. **Remote HTLC signature**: The counterparty's pre-signed signature. They gave us this when we first added the HTLC to the channel.

3. **Local HTLC signature**: Our signature that we just created in Step 2.

4. **Empty byte array (`&[][..]`)**: This is OP_FALSE, which tells the script to take the timeout path (remember, the script checks the size - not 32 bytes means timeout, and uses NOTIF). This empty array ensures we don't take the success path.

5. **HTLC script**: The full offered HTLC script. In SegWit (P2WSH), we always provide the actual script in the witness when spending.

The order matters! The script will pop these elements in reverse order and execute them.

</details>

<details>
  <summary>Step 4: Attach the Witness to the Transaction</summary>

Finally, we add the witness to the transaction's input:
```rust
let mut signed_tx = tx;
signed_tx.input[0].witness = witness;
```

We make the transaction mutable (since we're modifying it), then set the witness field of the first input (index 0). 

Note: We use `input[0]` because HTLC timeout transactions only have one input - the HTLC output from the commitment transaction.

</details>

<details>
  <summary>Step 5: Return the Finalized Transaction</summary>
```rust
signed_tx
```

We return the now-complete transaction, which includes:
- The transaction structure (inputs, outputs, lock_time, etc.)
- The witness data with both signatures and the script

This transaction is now ready to be broadcast to the Bitcoin network!

</details>