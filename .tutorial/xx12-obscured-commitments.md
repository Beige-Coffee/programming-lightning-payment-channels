# Obscured Commitment Number
Okay, we're sooooo close to implementing our first commitment transaction! However, there is one more important part we have to review.

Imagine we (Alice) and Bob have been sending payments back and forth for a while now. We've even reached 1,000,000 payments - wow! However, what if Bob tries to pull a fast one and publish an old commitment transaction? Remember, each channel state uses a unique set of private and public keys!

*Don't be afraid to zoom into the diagram below!*

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/unknown_commit.png" alt="unknown_commit" width="100%" height="auto">
</p>

#### Question: How would we know which commitment state Bob is publishing so that we can punish him and claim all channel funds via the revocation path?

<details>
<summary>Answer</summary>

One idea is to simply store all previous transactions so that we can iterate through our history and see which state matches the transaction Bob posted. That said, this doesn't sound too efficient.

Another idea is to put the commitment number in the transaction itself! That way, by just looking at teh transaction itself, we can identify the commitment number, and generate the associated private key, and claim the funds from the revocation path.

Let's say we take this approach. **Where would we store the commitment number?** Remember, the max number of commitments we can generate is 2^48 - 1, so we need **6 bytes (48 bits)** of storage space!

Two possible options are the **locktime** and **sequence** fields, as those are not  used for a specific purpose in commitment transactions.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/tx_commitment_storage.png" alt="tx_commitment_storage" width="50%" height="auto">
</p>

#### Question: Why is it not a great idea to put the raw commitment number in the transaction, which is publicly observable on the blockchain?

<details>
<summary>Answer</summary>

If we embed the raw commitment number within the transaction, that would be a massive privacy leak, as anyone would be able to see how many commitment states our lightning channel had at the time of closure.

To prevent this privacy leak, the Lightning protocol specifies that we must *obscure* the number of commitments by using an **XOR** operation with a SHA256-derived factor (based on *both channel partner's*  **Payment Basepoints**). Since the **Payment Basepoints** should only be known by the channel parties, this ensures that outsiders will be unable to decipher the actual number of commitments. Remember, Alice and Bob exchange **Payment Basepoints** when setting up their channel.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/payment_basepoint_send.png" alt="payment_basepoint_send" width="100%" height="auto">
</p>

Since the number of commitments requires up to 6 bytes to store, we separate the obscured commitment number into two 24 bit chunks.
- The upper 24 bits are placed in the **Locktime** field, prefixed with `0x20` (8 bits) since this is a 4 byte field.
  - We prefix with `0x20` because it ensures the resulting locktime will evaluate to something above 536,870,912 but below 546,937,241. Since anything above 500,000,000 is interpreted as a Unix timestamp, and 536,870,912 - 546,937,241 is, roughly, around 1987, the locktime will always be a valid locktime in the past. Therefore, this workaround enables us to use this field for purposes other than the locktime - like storing data.
- The lower 24 bits are placed in the **sequence** field, prefixed with `0x80` (8 bits) since this is a 4 byte field.
  - Similar to the locktime field, we prefix with `0x80` because it disables any relative locktimes (in relation to the 2-of-2 multisig funding transaction). We're then free to use the rest of the 24 bits to store our commitment transaction data!

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/obscured_commitment.png" alt="obscured_commitment" width="100%" height="auto">
</p>

</details>

</details>


## ⚡️ Write A Function To Generate An Obscure Factor 

Let's get to work! First and foremost, to generate an obscured commitment number, we'll need to write a function to generate an obscure factor. This is what we'll use to XOR our commitment number.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/obscure_factor.png" alt="obscure_factor" width="50%" height="auto">
</p>

If you head over to `src/exercises/transactions/commitment.rs`, you'll find the function`get_commitment_transaction_number_obscure_factor`. This function takes the following parameters:
- `initiator_payment_basepoint` (`&PublicKey`): This is the Payment Basepoint of the channel opener.
- `receiver_payment_basepoint` (`&PublicKey`): This is the Payment Basepoint of the channel receiver.

For this exercise, we need to create a function that:
- Takes both parties' payment basepoints (`initiator_payment_basepoint` and `receiver_payment_basepoint`)
- Returns a `u64` containing the 48-bit obscure factor

```rust
pub fn get_commitment_transaction_number_obscure_factor(
    initiator_payment_basepoint: &PublicKey,
    receiver_payment_basepoint: &PublicKey,
) -> u64 {
  
  // Step 1: Initialize the SHA256 Engine

  // Step 2: Step 2: Serialize and Input Both Basepoints

  // Step 3: Finalize the Hash
  
  // Step 4: Extract and Return Lower 48 Bits

}
```

<details>
  <summary>Step 1: Create the SHA256 Hash Engine</summary>

First, we create a SHA256 hash engine that we'll use to hash the payment basepoints:
```rust
let mut sha = Sha256::engine();
```

The `Sha256::engine()` creates a new hash engine that we can incrementally add data to before finalizing the hash.

</details>

<details>
  <summary>Step 2: Hash the Basepoints in Order</summary>

Next, we hash both payment basepoints in a deterministic order - initiator first, then receiver:
```rust
sha.input(&initiator_payment_basepoint.serialize());
sha.input(&receiver_payment_basepoint.serialize());
```

This ensures that both parties calculate the same obscure factor. The channel initiator (the party that opened the channel) always has their payment basepoint hashed first. We serialize each public key to get its byte representation before hashing.

</details>

<details>
  <summary>Step 3: Finalize the Hash</summary>

Now we finalize the hash and convert it to a byte array:
```rust
let res = Sha256::from_engine(sha).to_byte_array();
```

This gives us a 32-byte (256-bit) SHA256 hash. We'll use the last 6 bytes of this hash to create our obscure factor.

</details>

<details>
  <summary>Step 4: Extract and Combine the Last 6 Bytes</summary>

Finally, we take bytes 26-31 (the last 6 bytes) and combine them into a single 48-bit number stored in a `u64`:
```rust
((res[26] as u64) << 5 * 8)
    | ((res[27] as u64) << 4 * 8)
    | ((res[28] as u64) << 3 * 8)
    | ((res[29] as u64) << 2 * 8)
    | ((res[30] as u64) << 1 * 8)
    | ((res[31] as u64) << 0 * 8)
```

This bit manipulation takes each byte and shifts it into the correct position:
- `res[26]` goes into the most significant byte position (shifted left by 40 bits)
- `res[27]` goes into the next position (shifted left by 32 bits)
- And so on...
- `res[31]` goes into the least significant byte position (no shift)

The `|` operator combines all these shifted values with bitwise OR. The result is a 48-bit number (6 bytes) that serves as our obscure factor!

</details>

## ⚡️ Set Obscured Commitment Number in Transaction

Now, let's bring this full circle by implementing a function that will set the obscured commitment number for a transaction in a given commitment state.

To do this, we'll complete `set_obscured_commitment_number`, which is also located in `src/exercises/transactions/commitment.rs`.

This function takes the following inputs:
- **tx** (`&mut Transaction`): This is a *mutable* reference to a transaciton. In Rust terms, this just means that we are not taking ownership of the `Transaction` and that it's mutable, so we can change it.
- **commitment_number** (`u64`): This is the commitment number for the given channel state.
- **initiator_payment_basepoint** (&PublicKey): This is the Payment Basepoint of the channel opener.
- **receiver_payment_basepoint** (&PublicKey):This is the Payment Basepoint of the channel receiver.

Go ahead and give it a try! To complete this function, you'll need to use the function we created in last exercise to calculate the obscured commitment factor. You can then use that to derive the upper and lower 8 bits of the obscured commitment number (Steps 2 and 3 in the diagram below).

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/obscured_commitment.png" alt="obscured_commitment" width="100%" height="auto">
</p>

```rust
pub fn set_obscured_commitment_number(
    tx: &mut Transaction,
    commitment_number: u64,
    initiator_payment_basepoint: &PublicKey,
    receiver_payment_basepoint: &PublicKey,
) {
    let commitment_transaction_number_obscure_factor =
        get_commitment_transaction_number_obscure_factor(
            &initiator_payment_basepoint,
            &receiver_payment_basepoint,
        );

    let obscured_commitment_transaction_number = commitment_transaction_number_obscure_factor
        ^ (INITIAL_COMMITMENT_NUMBER - commitment_number);

    // Upper 24 bits in locktime
    let locktime_value =
        ((0x20 as u32) << 8 * 3) | ((obscured_commitment_transaction_number & 0xffffffu64) as u32);
    tx.lock_time = LockTime::from_consensus(locktime_value);

    // Lower 24 bits in sequence
    let sequence_value = Sequence(
        ((0x80 as u32) << 8 * 3) | ((obscured_commitment_transaction_number >> 3 * 8) as u32),
    );
    tx.input[0].sequence = sequence_value;
}
```
<details>
  <summary>Step 1: Calculate the Obscure Factor</summary>

First, we need to get our obscure factor using the function we just wrote:
```rust
let commitment_transaction_number_obscure_factor =
    get_commitment_transaction_number_obscure_factor(
        &initiator_payment_basepoint,
        &receiver_payment_basepoint,
    );
```

This gives us the 48-bit value we'll use to obscure the actual commitment number.

</details>

<details>
  <summary>Step 2: Calculate the Obscured Commitment Number</summary>

Now we XOR the obscure factor with the inverted commitment number:
```rust
let obscured_commitment_transaction_number = commitment_transaction_number_obscure_factor
    ^ (INITIAL_COMMITMENT_NUMBER - commitment_number);
```

Wait, why `INITIAL_COMMITMENT_NUMBER - commitment_number`? In Lightning, commitment numbers count DOWN from 281474976710655 (0xFFFFFFFFFFFF in hex, which is 48 bits of all 1s). So:
- First commitment: commitment_number = 281474976710655
- Second commitment: commitment_number = 281474976710654
- Third commitment: commitment_number = 281474976710653
- And so on...

By subtracting from `INITIAL_COMMITMENT_NUMBER`, we get a value that counts UP from 0, which is what we actually encode into the transaction. The XOR operation then obscures this value.

</details>

<details>
  <summary>Step 3: Encode Lower 24 Bits in lock_time</summary>

The lower 24 bits of the obscured commitment number go into the transaction's `lock_time` field:
```rust
let locktime_value =
    ((0x20 as u32) << 8 * 3) | ((obscured_commitment_transaction_number & 0xffffffu64) as u32);
tx.lock_time = LockTime::from_consensus(locktime_value);
```

Let's break this down:
- `0x20 << 8 * 3` puts the byte `0x20` in the most significant byte position (leftmost byte). This is a marker bit that indicates this is a Lightning commitment transaction.
- `obscured_commitment_transaction_number & 0xffffffu64` extracts the lower 24 bits (3 bytes) of our obscured number using a bitmask
- The `|` operator combines these, giving us a 32-bit value with the marker byte and our 24 bits
- `LockTime::from_consensus()` converts this into the proper `LockTime` type

</details>

<details>
  <summary>Step 4: Encode Upper 24 Bits in sequence</summary>

The upper 24 bits of the obscured commitment number go into the `sequence` field of the first input:
```rust
let sequence_value = Sequence(
    ((0x80 as u32) << 8 * 3) | ((obscured_commitment_transaction_number >> 3 * 8) as u32),
);
tx.input[0].sequence = sequence_value;
```

Similarly:
- `0x80 << 8 * 3` puts the byte `0x80` in the most significant byte position. This is another marker that indicates this transaction uses BIP 68 (relative lock-time).
- `obscured_commitment_transaction_number >> 3 * 8` shifts right by 24 bits (3 bytes) to get the upper 24 bits of our obscured number
- The `|` operator combines these into a 32-bit value
- We wrap it in `Sequence()` to create the proper type
- We set this on the first input (`tx.input[0]`)

</details>
