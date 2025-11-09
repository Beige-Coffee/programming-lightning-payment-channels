# Commitment Transaction Structure

Alright, let's take a break from public and private keys for a moment. Now that we understand how we'll derive the keys needed for Lightning, let's return to our scripts and implement them!

Remember, Alice and Bob have asymetric versions of the commitment transactions - meaning that Alice will have a special locking script on her `to_local` output and Bob will have a special locking script on his `to_local` output. As we just learned, this is how Alice and Bob can, effectively, protect each other against them cheating in the future.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/commitment_scripts.png" alt="commitment_scripts" width="100%" height="auto">
</p>

### ⚡️ Create To Remote Script

For this exercise, head over to `src/exercises/scripts/commitment.rs`! In this file, you'll find two functions - one to create the `to_remote` script and one to create the `to_local` script. Both functions will return a `ScriptBuf` type, which represents a bitcoin script in Rust Bitcoin.

Let's start with implementing `create_to_remote_script`, which takes the remote party's `remote_pubkey`. If you recall, this is simply the remote party's **Payment Basepoint**. This *does not* change for each commitment transaction, making it easier for a remote party to spend their funds from any given channel state, since they do not need to derive the private key for that specific state.

Since our counterparty should be able to spend these funds immediately, we simply create a **Pay-To-Witness-Public-Key-Hash** (**P2WPKH**) locking script, which only requires a valid signature to spend from. Below is what a P2WPKH output looks like.
- The first part is the **version byte**. In this case, it's `OP_0`, and it signals that this script is either **P2WPKH** or **P2WSH**. If it was `OP_1`, then this would be a **Pay-To-Taproot** (P2TR) output.
- Next, we place the hash of the public key. In this case, it's a 20-byte hash because we HASH160 the public key, which returns a 20 byte result.
```
OP_0 <20-byte-pubkey-hash>
```

Go ahead and try implementing the function below! You'll need to hash the public key using `Hash160::hash()` and use Rust Bitcoin's `Builder` to construct the script.


```rust
pub fn create_to_remote_script(remote_pubkey: &PublicKey) -> ScriptBuf {
    // P2WPKH format: OP_0 <20-byte-pubkey-hash>
    let pubkey_hash = Hash160::hash(&remote_pubkey.serialize());
    Builder::new()
        .push_int(0)
        .push_slice(pubkey_hash.as_byte_array())
        .into_script()
}
```

<details>
  <summary>Step 1: Hash the Public Key</summary>

First, we need to create a Hash160 of the remote party's public key. Hash160 is Bitcoin's standard hashing function that applies SHA256 followed by RIPEMD160, giving us a 20-byte hash.
```rust
let pubkey_hash = Hash160::hash(&remote_pubkey.serialize());
```

We call `.serialize()` on the public key to get its byte representation, then pass those bytes to `Hash160::hash()`. This gives us the 20-byte identifier that will go into our P2WPKH script.

</details>

<details>
  <summary>Step 2: Build the P2WPKH Script</summary>

Now we use Rust Bitcoin's `Builder` to construct our P2WPKH script. The format is simple: `OP_0` followed by the 20-byte public key hash.
```rust
Builder::new()
    .push_int(0)
    .push_slice(pubkey_hash.as_byte_array())
    .into_script()
```

The `push_int(0)` method creates the `OP_0` version byte, and `push_slice()` adds our 20-byte hash. Finally, `into_script()` converts the builder into a `ScriptBuf`. 

This script allows the remote party to spend these funds immediately by providing a valid signature in the witness field - no delays, no complications!

</details>

### ⚡️ Create To Local Script

Now, let's string together many of the pieces we've been learning about and build our `to_local` script. Remember, this script defines the spending conditions for any funds that the **holder** of the commitment transaction owns. In other words, Alice locks *her* funds to this script and Bob locks *his* funds to this script.

It has the following two spending conditions:
1. **Revocation Spending Path** - If the **holder** cheats by publishing an old state, their counterparty can punish them and spend from the revocation path immediatelly.
2. **Delayed Spending Path** - If the **holder** publishes the current state, then their counterparty does not know the secret to spend from the "revocation path". Therefore, the **holder** can spend from the "Delayed Spending Path" after `to_self_delay` blocks have passed since this transaction was mined.

The `create_to_local_script` takes the following parameters:
- `revocation_pubkey`: This is created by combining our counterparty's **Revocation Basepoint** with our **Per Commitment Point**.
- `local_delayedpubkey`: This created by combining our **Delayed Payment Basepoint** with our **Per Commitment Point**.
- `to_self_delay`: This is the number of blocks we must wait before we can spend the output. This is negotiated with Bob when we open the channel.


Go ahead and try implementing the function below! You'll need to use `Builder` to construct this conditional script with the CSV opcode.

```rust
pub fn create_to_local_script(
    revocation_pubkey: &PublicKey,
    local_delayedpubkey: &PublicKey,
    to_self_delay: u16,
) -> ScriptBuf {
    Builder::new()
        .push_opcode(opcodes::OP_IF)
        .push_slice(revocation_pubkey.serialize())
        .push_opcode(opcodes::OP_ELSE)
        .push_int(to_self_delay as i64)
        .push_opcode(opcodes::OP_CSV)
        .push_opcode(opcodes::OP_DROP)
        .push_slice(local_delayedpubkey.serialize())
        .push_opcode(opcodes::OP_ENDIF)
        .push_opcode(opcodes::OP_CHECKSIG)
        .into_script()
}
```

<details>
  <summary>Step 1: Build the Conditional Structure</summary>

We start by creating an IF/ELSE conditional structure. The IF branch is for the revocation case (immediate spending), and the ELSE branch is for the delayed case (normal spending after timelock).
```rust
Builder::new()
    .push_opcode(opcodes::OP_IF)
    .push_slice(revocation_pubkey.serialize())
    .push_opcode(opcodes::OP_ELSE)
```

When spending from this output, the witness will include a boolean value that determines which branch executes. If TRUE (1), the IF branch runs and the revocation key is used. If FALSE (0), the ELSE branch runs with the timelock.

</details>

<details>
  <summary>Step 2: Add the CSV Timelock and Delayed Key</summary>

In the ELSE branch, we implement the timelock using the CSV opcode. This ensures you must wait `to_self_delay` blocks before spending:
```rust
.push_int(to_self_delay as i64)
.push_opcode(opcodes::OP_CSV)
.push_opcode(opcodes::OP_DROP)
.push_slice(local_delayedpubkey.serialize())
```

The `OP_CSV` opcode checks that the spending transaction's sequence field indicates at least `to_self_delay` blocks have passed. We then use `OP_DROP` to remove the delay value from the stack (CSV doesn't consume it automatically), and push your delayed public key.

</details>

<details>
  <summary>Step 3: Close the Conditional and Verify Signature</summary>

Finally, we close the IF/ELSE structure with `OP_ENDIF` and add `OP_CHECKSIG` to verify the signature:
```rust
.push_opcode(opcodes::OP_ENDIF)
.push_opcode(opcodes::OP_CHECKSIG)
.into_script()
```

No matter which branch was taken (revocation or delayed), the script ends with one public key on the stack. `OP_CHECKSIG` verifies that the signature in the witness matches this key. By using a single `OP_CHECKSIG` at the end instead of in each branch, we make the script more efficient!

This script ensures that you can't close a channel and immediately spend your funds - you must wait. And if you try to cheat by broadcasting an old state, your counterparty can use the revocation key to take everything. This is what makes Lightning channels secure! ⚡️

</details>
