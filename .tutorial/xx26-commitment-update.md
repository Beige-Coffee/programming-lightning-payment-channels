# Updating Our Commitment Transaction

Believe it or not, we're almost done with our last coding exercises! At this point, we've implemented our HTLC Timeout Transaction, but we haven't add the HTLC to our commitment transaction just yet. Let's do that now!

For this next exercise, we're going to return the to `create_commitment_transaction` function that we created earlier in the course. If you need a reminder, check your `src/exercises/transactions/commitment.rs` file or view the function definition below.

```rust
pub fn create_commitment_transaction(
    funding_outpoint: OutPoint,
    to_local_value: u64,
    to_remote_value: u64,
    commitment_keys: &CommitmentKeys,
    local_payment_basepoint: &PublicKey,
    remote_payment_basepoint: &PublicKey,
    commitment_number: u64,
    to_self_delay: u16,
    dust_limit_satoshis: u64,
    feerate_per_kw: u64,
    offered_htlcs: &[HTLCOutput],
    received_htlcs: &[HTLCOutput],
) -> Transaction
```

If you recall, we largely ignored the two inputs `offered_htlcs` and `received_htlcs`, as we had not yet reviewed HTLCs. Now that we have a solid understanding of how HTLCs work and have implemented them ourselves, let's update the `create_commitment_transaction` function so that it will add HTLCs to our commitment transactions.

## ⚡️ Write A Function To Create HTLC Outputs

To begin this journey, we'll start by implementing a helper function that will produce a `Vec` of `OutputWithMetadata` for our HTLCs. This is very similar to the `create_commitment_transaction_outputs` function we implemented earlier. As we'll see shortly, by producing a `Vec` of `OutputWithMetadata` for our HTLCs, `to_local` and `to_remote` outputs, it will make it very easy for us to sort all outputs in our commitment transaction appropriately.

<details>
  <summary>Click to see the OutputWithMetadata Type</summary>

The OutputWithMetadata is a custom Type that represents a commitment transaction output. To be clear, this Type is not provided by Rust Bitcoin. It's something that is specific to the Programming Lightning course.

If you're familiar with Lightning and Hash Time-Locked Contracts (HTLCs), all of these fields may look familiar to you. If not, no worries at all! Below is a brief overview of what you'll need to know *for the purposes of this exercise*.
- `value`: This is simply the amount of bitcoin locked to this output.
- `script`: This is the script we're locking the bitcoin to. Since we've only learned about `to_local` and `to_remote` outputs thus far, you can imagine this holding the `ScriptBuf` type for those outputs.
- `cltv_expiry`: This .... is a surprise for later! If you know how HTLCs work, then this the expiry! It makes things easier and more intuitive if we include this in the OutputWithMetadata since we'll need to use it for sorting our outputs. Since we haven't covered HTLCs yet (and there is no expiry for `to_local` and `to_remote` outputs), we'll simply set this value to `None` for this exercise.

```rust
pub struct OutputWithMetadata {
    pub value: u64,
    pub script: ScriptBuf,
    pub cltv_expiry: Option<u32>,
}
```
</details>

For this exercise, head over to `src/exercises/transactions/commitment.rs`, and let's implement `create_htlc_outputs`, which takes the following inputs:
- `commitment_keys`: This is a custom sruct that holds all of the keys you'll need to complete this transaction. You can learn more about it below.
- `offered_htlcs`: A slice of HTLCs that we offered to our counterparty. These will be encumbered with an HTLC Offerer script.
- `received_htlcs`: A slice of HTLCs that our counterparty offered to us. These will be encumbered with an HTLC Receiver script.


 <details>
   <summary>Click to see the CommitmentKeys Type</summary>

The CommitmentKeys type is meant to hold all of the public keys we'll need for any given channel state. In other words, these keys have already been tweaked by the **Per Commitment Point** and are unique to a specific channel state.

 ```rust
pub struct CommitmentKeys {
    /// The per-commitment point used to derive the other keys
    pub per_commitment_point: PublicKey,

    /// The revocation key which allows the broadcaster's counterparty to punish
    /// them if they broadcast an old state
    pub revocation_key: PublicKey,

    /// Local party's HTLC key (derived from local_htlc_basepoint)
    pub local_htlc_key: PublicKey,

    /// Remote party's HTLC key (derived from remote_htlc_basepoint)
    pub remote_htlc_key: PublicKey,

    /// Local party's delayed payment key (for to_local output)
    pub local_delayed_payment_key: PublicKey,
}
 ```
 </details>

<details>
  <summary>Click to see the HTLCOutput Type</summary>

The `HTLCOutput` type represents an HTLC that needs to be added to a commitment transaction.

```rust
pub struct HTLCOutput {
    /// The amount of this HTLC in satoshis
    pub amount_sat: u64,
    
    /// The payment hash (32 bytes)
    pub payment_hash: [u8; 32],
    
    /// The CLTV expiry height for this HTLC
    pub cltv_expiry: u32,
}
```

</details>

```rust
fn create_htlc_outputs(
    commitment_keys: &CommitmentKeys,
    offered_htlcs: &[HTLCOutput],
    received_htlcs: &[HTLCOutput],
) -> Vec<OutputWithMetadata> {
    let mut outputs = Vec::new();

    // Create offered HTLC outputs (we offered, they can claim with preimage)
    for htlc in offered_htlcs {
        let script = create_offered_htlc_script(
            &commitment_keys.revocation_key,
            &commitment_keys.local_htlc_key,
            &commitment_keys.remote_htlc_key,
            &htlc.payment_hash,
        );
        outputs.push(OutputWithMetadata {
            value: htlc.amount_sat,
            script: script.to_p2wsh(),
            cltv_expiry: None,
        });
    }

    // Create received HTLC outputs (they offered, we can claim with preimage)
    for htlc in received_htlcs {
        let script = create_received_htlc_script(
            &commitment_keys.revocation_key,
            &commitment_keys.local_htlc_key,
            &commitment_keys.remote_htlc_key,
            &htlc.payment_hash,
            htlc.cltv_expiry,
        );

        outputs.push(OutputWithMetadata {
            value: htlc.amount_sat,
            script: script.to_p2wsh(),
            cltv_expiry: Some(htlc.cltv_expiry),
        });
    }

    outputs
}
```
<details>
  <summary>Step 1: Initialize the Outputs Vector</summary>

First, let's create a mutable vector to hold all our HTLC outputs (of type `OutputWithMetadata`).

```rust
let mut outputs = Vec::new();
```

</details>

<details>
  <summary>Step 2: Create Offered HTLC Outputs</summary>

Next, we'll iterate through each offered HTLC and create an `OutputWithMetadata`. Remember, you can use the `create_offered_htlc_script` script that you created earlier to build the output script for the HTLC.

<details>
  <summary>Click to see create_offered_htlc_script function definition</summary>

```rust
pub fn create_offered_htlc_script(
    revocation_pubkey: &PublicKey,
    local_htlcpubkey: &PublicKey,
    remote_htlcpubkey: &PublicKey,
    payment_hash: &[u8; 32],
) -> ScriptBuf
```

</details>

```rust
for htlc in offered_htlcs {
    let script = create_offered_htlc_script(
        &commitment_keys.revocation_key,
        &commitment_keys.local_htlc_key,
        &commitment_keys.remote_htlc_key,
        &htlc.payment_hash,
    );
    outputs.push(OutputWithMetadata {
        value: htlc.amount_sat,
        script: script.to_p2wsh(),
        cltv_expiry: None,
    });
}
```

</details>

<details>
  <summary>Step 3: Create Received HTLC Outputs</summary>

Now, let's iterate through each received HTLCs and create an `OutputWithMetadata`. Similar to the last step, you can use the `create_received_htlc_script` to build the output script for the HTLC.

<details>
  <summary>Click to see create_received_htlc_script function definition</summary>

```rust
pub fn create_received_htlc_script(
    revocation_pubkey: &PublicKey,
    local_htlcpubkey: &PublicKey,
    remote_htlcpubkey: &PublicKey,
    payment_hash: &[u8; 32],
    cltv_expiry: u32,
) -> ScriptBuf
```

</details>

```rust
for htlc in received_htlcs {
    let script = create_received_htlc_script(
        &commitment_keys.revocation_key,
        &commitment_keys.local_htlc_key,
        &commitment_keys.remote_htlc_key,
        &htlc.payment_hash,
        htlc.cltv_expiry,
    );

    outputs.push(OutputWithMetadata {
        value: htlc.amount_sat,
        script: script.to_p2wsh(),
        cltv_expiry: Some(htlc.cltv_expiry),
    });
}
```

</details>

<details>
  <summary>Step 4: Return the Outputs</summary>

Lastly, make sure to return the vector containing all HTLC outputs!

```rust
outputs
```
</details>


## ⚡️ Update Our create_commitment_transaction Function

Now, let's put everything together and update the `create_commitment_transaction` function to include HTLC outputs. Earlier in the course, we implemented most of this function but left out the HTLC functionality. Now that we've built all the HTLC scripts and helper functions, let's add them to our commitment transaction!

For this exercise, head back to `src/exercises/transactions/commitment.rs` and update the function to account for any `offered_htlcs` or `received_htlcs` that may be passed in.

To successfully complete this exercise, you'll need to:
1. Create HTLC outputs using the `create_htlc_outputs` helper function we just implemented.
2. Sort all outputs (including HTLCs) according to BOLT #3 specifications.


```rust
pub fn create_commitment_transaction(
    funding_outpoint: OutPoint,
    to_local_value: u64,
    to_remote_value: u64,
    commitment_keys: &CommitmentKeys,
    local_payment_basepoint: &PublicKey,
    remote_payment_basepoint: &PublicKey,
    commitment_number: u64,
    to_self_delay: u16,
    dust_limit_satoshis: u64,
    feerate_per_kw: u64,
    offered_htlcs: &[HTLCOutput],
    received_htlcs: &[HTLCOutput],
) -> Transaction {
    // Calculate fee based on number of HTLCs
    let num_htlcs = offered_htlcs.len() + received_htlcs.len();
    let fee = calculate_commitment_tx_fee(feerate_per_kw, num_htlcs);
    let mut output_metadata = Vec::new();

    let channel_outputs = create_commitment_transaction_outputs(
        to_local_value,
        to_remote_value,
        commitment_keys,
        remote_payment_basepoint,
        to_self_delay,
        dust_limit_satoshis,
        fee,
    );

    let htlc_outputs = create_htlc_outputs(&commitment_keys, &offered_htlcs, &received_htlcs);

    // Add to_local and to_remote outputs
    output_metadata.extend(channel_outputs);

    // Add all HTLC outputs
    output_metadata.extend(htlc_outputs);

    // Sort everything once
    sort_outputs(&mut output_metadata);

    // Convert to TxOut
    let outputs: Vec<TxOut> = output_metadata
        .iter()
        .map(|meta| TxOut {
            value: Amount::from_sat(meta.value),
            script_pubkey: meta.script.clone(),
        })
        .collect();

    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: funding_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: outputs,
    };

    set_obscured_commitment_number(
        &mut tx,
        commitment_number,
        local_payment_basepoint,
        remote_payment_basepoint,
    );

    tx
}
```

<details>
  <summary>Step 1: Create HTLC Outputs</summary>

First, we need to create all the HTLC outputs. This should be relatively easy, as all we need to do is pass the relevant information into the function we just implemented! 

```rust
let htlc_outputs = create_htlc_outputs(&commitment_keys, &offered_htlcs, &received_htlcs);
```

</details>

<details>
  <summary>Step 2: Sort All Outputs</summary>

As we learned earlier in the course, the Lightning network spec (specifically, BOLT 3: [Transaction Output Ordering](https://github.com/lightning/bolts/blob/master/03-transactions.md#transaction-output-ordering)) specifies that outputs should be ordered in the following manner:
- First, according to their value - smallest first.
  - If there is a tie, the output with the lexicographically lesser `scriptpubkey` comes first, then selecting the shorter script (if they differ in length).
  - For HTLC outputs, if there is a tie after sorting via the above, then they are ordered in increasing `cltv_expiry` order.

In a previous exercise, we used `sort_outputs` to sort a `Vec` of `OutputWithMetadata` according to the above logic. If you completed `create_commitment_transaction` when we first implemented it, then you would have likely used the `sort_outputs` helper function (see below) to sort the outputs.

<details>
  <summary>Click to see sort_outputs function</summary>

```rust
pub fn sort_outputs(outputs: &mut Vec<OutputWithMetadata>) {
    outputs.sort_by(|a, b| {
        a.value
            .cmp(&b.value)
            .then(a.script.cmp(&b.script))
            .then(a.cltv_expiry.cmp(&b.cltv_expiry))
    });
}
```
</details>

For this exercise, you'll need to make sure that the HTLC outputs are added to the `Vec` of outputs ***before*** passing it into the `sort_outputs` function. This will ensure all outputs are added in the correct order.

Below is an example of how you would string everything together.

```rust
// initialize mutable object to hold outputs
let mut output_metadata = Vec::new();

// build htlc outputs
let htlc_outputs = create_htlc_outputs(&commitment_keys, &offered_htlcs, &received_htlcs);

// Add to_local and to_remote outputs
output_metadata.extend(channel_outputs);

// Add all HTLC outputs
output_metadata.extend(htlc_outputs);

// Sort everything once
sort_outputs(&mut output_metadata);
```
</details>