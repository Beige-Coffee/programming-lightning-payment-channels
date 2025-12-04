# Changing Our Public Keys For Each Commitment State

## Per-Commitment Secrets & Points

Alright, things are heating up fast. Hold on, we still have a lot to cover!

In the last exercise, we learned that Alice and Bob will each generate a series of **Per-Commitiment Secrets** (Private Keys) and **Per-Commitment Points** (Public Keys) for each channel state. As we saw, these points are then used to create *unique* **Revocation Public Keys** for each channel state. Below is an image depicting this process from Alice's point of view. All she needs is a **commitment seed**, which she created when she opened the channel with Bob, and the **channel state index**. NOTE: in the below picture, the colors for Alice's keys have been changed back to match the colors in the original Channel Keys legend.

SPOILER ALERT: The **Per-Commitment Points** is actually used for more than just the **Revocation Public Keys**, but more on that soon!  

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/alice_per_commitment_points.png" alt="alice_per_commitment_points" width="50%" height="auto">
</p>

So, at this point, the outstanding question on *everyone's* mind is: **"How do we create the Per-Commitment Secret from the Commitment Seed?!"** In other words, what does the gear icon actually mean?

We'll, let's dig into that now.

## Deriving A Per Commitment Secret
BOLT 3, in the [Per-commitment Secret Requirements](https://github.com/lightning/bolts/blob/master/03-transactions.md#per-commitment-secret-requirements) section, provides a specific algorithm for generating the secret for any given channel state. The algorithm, listed below, has the following two parameters:
- **seed**: This is the commitment seed for the channel.
- **I**: This is the index. Per BOLT 3, the index starts at 281474976710655 for the first channel state, and it's decremented by 1 for each new state.

#### todo!: explain how it works

```
generate_from_seed(seed, I):
P = seed
for B in 47 down to 0:
    if B set in I:
        flip(B) in P
        P = SHA256(P)
return P
```

### ⚡️ Build Per-Commitment Secret

Now that we've reviewed how the formula works, let's implement it ourselves! To do this, head back over to `src/exercises/keys/channel_keys_manager.rs`. Since this is where we implemented our `ChannelKeysManager`, which holds our channel keys (ex: commitment seed), it will be useful to implement the ability to generate a per-commitment secret here!

```rust
impl ChannelKeyManager {
pub fn build_commitment_secret(&self, commitment_number: u64) -> [u8; 32] {
    let mut res: [u8; 32] = self.commitment_seed.clone();
    for i in 0..48 {
        let bitpos = 47 - i;
        if commitment_number & (1 << bitpos) == (1 << bitpos) {
            res[bitpos / 8] ^= 1 << (bitpos & 7);
            res = Sha256::hash(&res).to_byte_array();
        }
    }
    res
  }
}
```

<details>
<summary>Step 1: Initialize the Result with the Commitment Seed</summary>

We start by creating a mutable variable that will hold our result. We initialize it with a copy of our commitment_seed - this seed is the starting point for generating all commitment secrets.
The mut keyword makes this variable mutable, which we'll need because we're going to modify it through the hashing process.

```rust
let mut res: [u8; 32] = self.commitment_seed.clone();
```

</details>


</details>
<details>
  <summary>Step 2: Loop Through 48 Bits and Check Each One</summary>
  
The BOLT 3 algorithm examines 48 bits of the `commitment_number` (giving us 2^48 possible commitments). For each iteration, we calculate the bit position starting from bit 47 down to 0, then check if that bit is set in the commitment number using bitwise operations.
The expression `commitment_number & (1 << bitpos)` checks if the bit at position `bitpos` is set to 1.

```rust
for i in 0..48 {
    let bitpos = 47 - i;
    if commitment_number & (1 << bitpos) == (1 << bitpos) {
```
</details>

<details>
<summary>Step 3: Flip the Bit and Hash</summary>

When we find a set bit, we flip the corresponding bit in our result array, then hash the entire result with SHA256.

We calculate which byte contains our bit (`bitpos / 8`) and which bit within that byte (`bitpos & 7`), then use XOR to flip it. The hash operation makes the process irreversible, which is key to the security of commitment secrets.

```rust
if commitment_number & (1 << bitpos) == (1 << bitpos) {
  res[bitpos / 8] ^= 1 << (bitpos & 7);
  res = Sha256::hash(&res).to_byte_array();
}
```
</details>

<details>
  <summary>Step 4: Return the Commitment Secret</summary>
  
After processing all 48 bits, we return the final 32-byte secret unique to this commitment_number

```rust
res
```

</details>

### ⚡️ Build Per-Commitment Point

Now that we have the ability to generate a Per-Commitment Secret, let's build the functionality to turn that into a Per-Commitment Point. To do this, we'll implement the function `derive_per_commitment_point`, on our `ChannelKeysManager`.

This function will take one input: the `commitment_number`. We'll pass the commitment number into the `build_commitment_secret` function we created in the last exercise, which will return a `SecretKey` type. Just was we've done in previous exercises, we'll convert the secret key to a public key and return it!

```rust
impl ChannelKeyManager {
    pub fn derive_per_commitment_point(&self, commitment_number: u64) -> PublicKey {
        let secret = self.build_commitment_secret(commitment_number);
        let secret_key = SecretKey::from_slice(&secret).expect("Valid secret");
        PublicKey::from_secret_key(&self.secp_ctx, &secret_key)
    }
}
```

<details>
  <summary>Step 1: Generate the Commitment Secret</summary>

First, we use the `build_commitment_secret` function we just implemented to generate the unique secret for this commitment number

```rust
let secret = self.build_commitment_secret(commitment_number);
```
</details>

<details>
  <summary>Step 2: Convert Bytes to SecretKey</summary>

Our commitment secret is currently just a 32-byte array. To perform elliptic curve operations on it, we need to convert it into a proper `SecretKey` type that the secp256k1 library can work with.

The `from_slice` function parses our bytes and validates that they represent a valid secret key on the secp256k1 curve.

```rust
let secret_key = SecretKey::from_slice(&secret).expect("Valid secret");
```

</details>


<details>
  <summary>Step 3: Derive and Return the Public Key</summary>
    
Finally, we derive the public key from our secret key using elliptic curve multiplication. This public key is our **Per-Commitment Point** - it's safe to share with our channel partner and will be used in key derivation for this specific commitment transaction.

```rust
PublicKey::from_secret_key(&self.secp_ctx, &secret_key)
```
</details>

## Lightning Key Derivation

At this point, we should have a good intuition for how we can derive new **Revocation Public Keys** for each commitment state. However, the fun doesn't stop just yet! In Lightning, we actually derive new pubic keys for most of the keys used in each commitment transaction.

For example, remember how we introduced a **Delayed Payment Public Key**, which is used in the delayed spending path (after the `to_self_delay` relative delay)? We'll, somewhat-similarly to the Revocation Public Key, this public key is also derived by combining it's basepoint (**Delayed Payment Basepoint**) with a given state's **Per-Commitment Point**.

Take a look at the visual below. In it, you can see that we use each state's **Per-Commitment Point** and combine it with a **Basepoint** such that it produces a new public key, which we can place in the actual locking script.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/alice_delayed_payment_derivation.png" alt="alice_delayed_payment_derivation" width="100%" height="auto">
</p>

However, the crucial difference here is that the **Delayed Payment Public Key** is not derived the same way that the **Revocation Public Key** is. Instead, it uses the *local party's* **Basepoint**. For example, Alice will use her **Delayed Payment Basepoint** and combine it with *her* **Per-Commitment Point**. She'll use the below equation to calculate the **Delayed Payment Public Key** , which she will put in *her* delayed spending path. The reason she *won't* use Bob's Basepoint is because this path is only ever meant to be spent by Alice, so it's important that she is able to derive the private key to spend from this path at any time.

```
pubkey = basepoint + SHA256(per_commitment_point || basepoint) * G
```

## Delayed Public Keys in Locking Scripts
Now that we know how we'll derive the **Delayed Payment Public Key** for a given transaction, let's review the overall flow one more time to make sure everything makes sense! Then, we'll code it up!

In the below diagram, you'll see the overall **Channel Establishment** message flow that we reviewed earlier. In it, you can see all of the **Basepoints** that Alice and Bob send to each other in the `open_channel` and `accept_channel` messages. If you've been paying extra close attention, you may notice that we've now included the **First Per-Commitment Points** in the `open_channel` and `accept_channel` messages! Per the [BOLT 2 specification](https://github.com/lightning/bolts/blob/master/02-peer-protocol.md#the-open_channel-message) the **First Per-Commitment Points** are actually included in these messages, but we left them out earlier because we had not yet introduced them!

At the bottom of the diagram, you can see how, for Alice's version of the commitment transaction, we calculate *her* **Delayed Payment Public Key** by plugging *her* **Delayed Payment Basepoint** and *her* **First Per-Commitment Point** into the formula provided in the BOLT 3 specification. On the other hand, we use *Bob's* **Delayed Payment Basepoint** and *Bob's* **First Per-Commitment Point** when calculating *his* **Delayed Payment Public Key**.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/delayed_payment_derivation.png" alt="delayed_payment_derivation" width="100%" height="auto">
</p>

#### Question: Why does Alice need Bob's Delayed Payment Basepoint? It looks like she only uses her Delayed Payment Basepoint to create the Delayed Payment Public Key that goes into her to_local witness script...
<details>
  <summary>Answer</summary>

Remember, each new transaction that Alice and Bob create is spending from the 2-of-2 funding output! Since Alice and Bob each have their own version of the commitment transactions, they will each require unique signatures from eachother to ensure their commitment transactions are valid.

Therefore, Alice will need to re-create *Bob's* version of the commitment transaction locally so that she can generate a signature for it and send that signature to Bob! Bob will do the same.

To ensure that Alice and Bob can create each other's commitment transaction's locally, they will share their Basepoints at when opening their channel.

</details>

### ⚡️ Derive Public Keys

Alright, let's get our hands dirty by implementing `derive_public_key`, a function that takes a **Basepoint** and **Per Commitment Point** and returns a public key for a specific commitment transaction. In case you're wondering, the function definition doesn't specify which basepoint for a few reasons. First, we can use this function to derive a **Delayed Payment Public Key** for our counterparty, which uses *their** **Delayed Payment Basepoint** and their **Per-Commitment Point**. Additionally, as we'll see later, there are other public keys that we'll derive using different **Basepoints**!

```rust
pub fn derive_public_key(
    basepoint: &PublicKey,
    per_commitment_point: &PublicKey,
    secp_ctx: &Secp256k1<All>,
) -> PublicKey {
    // pubkey = basepoint + SHA256(per_commitment_point || basepoint)
    let mut engine = Sha256::engine();
    engine.input(&per_commitment_point.serialize());
    engine.input(&basepoint.serialize());
    let res = Sha256::from_engine(engine);

    let hashkey = PublicKey::from_secret_key(
        &secp_ctx,
        &SecretKey::from_slice(res.as_byte_array())
            .expect("Hashes should always be valid keys unless SHA-256 is broken"),
    );

    basepoint.combine(&hashkey).expect("Addition only fails if the tweak is the inverse of the key. This is not possible when the tweak contains the hash of the key.")
}
```

<details>
  <summary>Step 1: Create a SHA256 Hashing Engine</summary>
We start by creating a SHA256 hasher that we'll use to hash our key material together. The `engine()` method gives us a hasher we can feed data into incrementally.

```rust
let mut engine = Sha256::engine();
```
</details>

<details>
<summary>Step 2: Hash the Per-Commitment Point and Basepoint</summary>

Next, we hash the concatenation of our per-commitment point and basepoint. The order matters here - per-commitment point first, then basepoint, as specified in BOLT 3!

We serialize each public key to bytes using `.serialize()` which gives us the compressed 33-byte representation, then feed those bytes into our hasher with `.input()`.

```rust
engine.input(&per_commitment_point.serialize());
engine.input(&basepoint.serialize());
```
</details>

<details>
<summary>Step 3: Finalize the Hash and Convert to Public Key</summary>

Now we finalize the hash and convert the resulting 32 bytes into a public key. This might seem odd - we're treating a hash output as a secret key and deriving its public key - but this is exactly what the BOLT 3 specification calls for!
  
The hash output is guaranteed to be a valid secret key (the only way it wouldn't be is if SHA-256 itself is broken). We multiply this secret by the generator point G to get our "tweak" public key.

```rust
let res = Sha256::from_engine(engine);
let hashkey = PublicKey::from_secret_key(
    &secp_ctx,
    &SecretKey::from_slice(res.as_byte_array())
        .expect("Hashes should always be valid keys unless SHA-256 is broken"),
);
```
</details>

<details>
  <summary>Step 4: Combine the Basepoint with the Hash Key</summary>
  
Finally, we add our basepoint to the hash key using elliptic curve point addition. The `.combine()` method performs this addition on the secp256k1 curve.

This operation can only fail if the tweak is the exact inverse of the basepoint (which would result in the point at infinity). But since our tweak includes a hash of the basepoint itself, this is cryptographically impossible!

```rust
basepoint.combine(&hashkey).expect("Addition only fails if the tweak is the inverse of the key. This is not possible when the tweak contains the hash of the key.")
```
</details>

## Deriving Private Keys
Okay, so we have our public keys ready to go! But, how do we generate the private keys so that we can spend from any given commitment state? For example, the diagram below depicts a situation where Alice needs to claim her funds from the first commitment state, which we've been calling the "Refund" transaction. To do this, she needs to spend from the **Delayed Payment Public Key**, which is unique to this commitment state.

To be clear, we can imagine that Alice is locking her funds to another public key for which she knows the private key. It's shown in a black color below to indicate that it's not a public key related to our Lightning wallet. It can be any other public key that Alice controls.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/alice_refund_claim.png" alt="alice_refund_claim" width="100%" height="auto">
</p>

If you recall from earlier when we created our **Delayed Payment Public Key**, we combined the **Delayed Payment Basepoint** with the **Per-Commitment Point**. Therefore, to generate the private key to spend from the **Delayed Payment Public Key**, we'll have to combine the **Delayed Payment Secret** with the same tweak we added to our **Delayed Payment Secret** to create the **Delayed Payment Public Key**. BOLT 3 provides us with the equation to do just this! You can see it below.

```
privkey = basepoint_secret + SHA256(per_commitment_point || basepoint)
```

#### Question: Take a look at the newly added sequence field in the "input" section of Alice's refund claim transaction. What is to_self_delay here?
<details>
  <summary>Answer</summary>

Recall how we added a delay so that Alice had to wait `to_self_delay` (ex: 2016 blocks or ~2 weeks) before she could claim her funds from this output, if it's mined? This was to give Bob time to claim these funds first, if Alice was cheating by broadcasting this transaction *after* they had already agreed to move to a new channel state.

We'll, in this example, we're assuming that Alice is playing nice and is fairly claiming these funds back. To do this, she will have to set the `sequence` field in the input, which specifies the output she's spending, to the `to_self_delay` value that was embedded in the script. If you're intersted in reading the details, you can read the OP_CHECKSEQUENCEVERIFY BIP [here](https://github.com/bitcoin/bips/blob/master/bip-0112.mediawiki). 

If you're busy (or are intimidated by the BIP - they can be scary), here is the TLDR: The `sequence` field specifies a relative timelock on the input - meaning that a transaction cannot be mined until that amount of blocks (or time) has passed **since the input was mined**. OP_CHECKSEQUENCEVERIFY, when being evaluated on the stack, will check if the provided delay (`to_self_delay`, in our case) is greater than or equal to the value in the `sequence` field. By doing this, we can restrict the delayed spending path such that Alice cannot spend from that path until the relative timelock has experied. Cool, eh?

</details>

### ⚡️ Derive Private Keys

For this exercise, we'll implement `derive_private_key`, a function that takes a `base_secret` (like our `delayed_payment_basepoint_secret`), a `per_commitment_point`, and returns the derived private key we can use to sign for that specific commitment.

```rust
pub fn derive_private_key(
    base_secret: &SecretKey,
    per_commitment_point: &PublicKey,
    secp_ctx: &Secp256k1<All>,
) -> SecretKey {
    // privkey = base_secret + SHA256(per_commitment_point || basepoint)
    let basepoint = PublicKey::from_secret_key(secp_ctx, base_secret);
    let mut engine = Sha256::engine();
    engine.input(&per_commitment_point.serialize());
    engine.input(&basepoint.serialize());
    let res = Sha256::from_engine(engine).to_byte_array();
    base_secret.clone().add_tweak(&Scalar::from_be_bytes(res).unwrap())
        .expect("Addition only fails if the tweak is the inverse of the key. This is not possible when the tweak contains the hash of the key.")
}
```

<details>
  <summary>Step 1: Derive the Basepoint Public Key</summary>

First, we need to derive the public key (basepoint) from our base secret. We need this because the hash in our derivation formula includes the basepoint, not the secret!

```rust
let basepoint = PublicKey::from_secret_key(secp_ctx, base_secret);
```
</details>


<details>
  <summary>Step 2: Hash the Per-Commitment Point and Basepoint</summary>
  
Just like we did for public key derivation, we create a SHA256 hasher and feed it the per-commitment point and basepoint in that exact order. This hash will become our "tweak" value

```rust
let mut engine = Sha256::engine();
engine.input(&per_commitment_point.serialize());
engine.input(&basepoint.serialize());
let res = Sha256::from_engine(engine).to_byte_array();
```
</details>

<details>
  <summary>Step 3: Add the Tweak to the Base Secret</summary>

Now for the key operation! We add our hash (the tweak) to the base secret using scalar addition on the secp256k1 curve. The `.add_tweak()` method handles this securely.

We need to convert our hash bytes into a `Scalar` (a number modulo the curve order) using `Scalar::from_be_bytes()`. The operation can only fail if the tweak happens to be the exact inverse of our key, but since the tweak includes a hash of the public key derived from our secret, this is cryptographically impossible.

```rust
base_secret.clone().add_tweak(&Scalar::from_be_bytes(res).unwrap())
.expect("Addition only fails if the tweak is the inverse of the key. This is not possible when the tweak contains the hash of the key.")
```
</details>