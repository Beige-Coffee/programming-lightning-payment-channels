# Channel Keys
At this point, we've implemented the functionality to create the following private keys, known as "basepoint secrets" in the Lightning protocol, and the commitment seed for a given Lightning channel.
  - **Funding Secret**: Secret Key
  - **Revocation Basepoint Secret**: Secret Key
  - **Payment Basepoint Secret**: Secret Key
  - **Delayed Payment Secret**: Secret Key
  - **HTLC Basepoint Secret**: Secret Key
  - **Commitment Seed**: The commitment seed is a 256-bit scalar used to generate a series of secrets **for each commitment state**.

NOTE: all of the above secrets are specifically defined in [BOLT #3: Bitcoin Transaction and Script Formats](https://github.com/lightning/bolts/blob/master/03-transactions.md) ***except*** for the "funding" private key and public key. We included it in our `KeyFamily`, but it doesn't have to be. For example, LDK -- a popular API-driven Lightning implementation -- allows for developers to bring their own on-chain wallet and, therefore, "funding" keys. This will become more clear, and we will revisit this point, when "open our Lightning channel".

Next, we'll need to derive the public keys associated with each private key. As we'll soon see, these public keys will be used to derive the actual public keys that are used in each commitment state in Lightning. Also, note that Alice, Bob, and every "node" on the Lightning network will have their own set of these keys, which they will use to operate the channel.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/our_channel_keys.png" alt="our_channel_keys" width="90%" height="auto">
</p>

## ⚡️ Convert Private Keys to Public Keys

Now that we can derive a complete set of private keys for a channel, we need to convert them to public keys! Remember, in Lightning (and Bitcoin), we keep our private keys secret but share our public keys with our channel counterparty. They'll need our public keys to construct transactions and verify our signatures.

To complete this exercise, you'll need to implement the `to_public_keys` function that converts each private key in our `ChannelKeyManager` to its corresponding public key, bundling them into a `ChannelPublicKeys` struct.

Let's take a look at what `ChannelPublicKeys` contains:

<details>
  <summary>Click to see ChannelPublicKeys</summary>

The `ChannelPublicKeys` struct holds the set of public keys that get shared with your channel counterparty. These keys mirror the private keys in our `ChannelKeyManager`, but they're safe to share publicly!

```rust
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ChannelPublicKeys {
    pub funding_pubkey: PublicKey,
    pub revocation_basepoint: PublicKey,
    pub payment_basepoint: PublicKey,
    pub delayed_payment_basepoint: PublicKey,
    pub htlc_basepoint: PublicKey,
}
```
</details>

```rust
impl ChannelKeyManager {
    pub fn to_public_keys(&self) -> ChannelPublicKeys {
        ChannelPublicKeys {
            funding_pubkey: PublicKey::from_secret_key(&self.secp_ctx, &self.funding_key),
            revocation_basepoint: PublicKey::from_secret_key(
                &self.secp_ctx,
                &self.revocation_base_key,
            ),
            payment_basepoint: PublicKey::from_secret_key(&self.secp_ctx, &self.payment_base_key),
            delayed_payment_basepoint: PublicKey::from_secret_key(
                &self.secp_ctx,
                &self.delayed_payment_base_key,
            ),
            htlc_basepoint: PublicKey::from_secret_key(&self.secp_ctx, &self.htlc_base_key),
        }
    }
```

<details>
  <summary>Step 1: Derive the Funding Public Key</summary>

Let's start by converting our private `funding_key` into a public key. The `PublicKey::from_secret_key` function takes two arguments: the secp256k1 context and the secret key to convert.
```rust
funding_pubkey: PublicKey::from_secret_key(&self.secp_ctx, &self.funding_key),
```

</details>

<details>
  <summary>Step 2: Derive the Revocation Basepoint</summary>

Next, we'll convert the revocation base key to a public key. Same pattern as before!
```rust
revocation_basepoint: PublicKey::from_secret_key(
    &self.secp_ctx,
    &self.revocation_base_key,
),
```

</details>

<details>
  <summary>Step 3: Derive the Payment Point</summary>

Now let's derive the payment point from our payment base key.
```rust
payment_basepoint: PublicKey::from_secret_key(&self.secp_ctx, &self.payment_base_key),
```

</details>

<details>
  <summary>Step 4: Derive the Delayed Payment Basepoint</summary>

Convert the delayed payment base key to its public counterpart.
```rust
delayed_payment_basepoint: PublicKey::from_secret_key(
    &self.secp_ctx,
    &self.delayed_payment_base_key,
),
```

</details>

<details>
  <summary>Step 5: Derive the HTLC Basepoint</summary>

Finally, let's convert the HTLC base key to a public key.
```rust
htlc_basepoint: PublicKey::from_secret_key(&self.secp_ctx, &self.htlc_base_key),
```

</details>

<details>
  <summary>Step 6: Construct and Return the ChannelPublicKeys</summary>

That's it! We've converted all five private keys to public keys. Now we just need to bundle them into a `ChannelPublicKeys` struct and return it. Rust's struct initialization syntax makes this super clean:
```rust
ChannelPublicKeys {
    funding_pubkey: PublicKey::from_secret_key(&self.secp_ctx, &self.funding_key),
    revocation_basepoint: PublicKey::from_secret_key(
        &self.secp_ctx,
        &self.revocation_base_key,
    ),
    payment_basepoint: PublicKey::from_secret_key(&self.secp_ctx, &self.payment_base_key),
    delayed_payment_basepoint: PublicKey::from_secret_key(
        &self.secp_ctx,
        &self.delayed_payment_base_key,
    ),
    htlc_basepoint: PublicKey::from_secret_key(&self.secp_ctx, &self.htlc_base_key),
}
```

Great work! You've now implemented the complete flow: from a random seed, to a master key, to channel-specific private keys, and finally to the public keys you'd share with your counterparty. This is the foundation of Lightning key management!

</details>
