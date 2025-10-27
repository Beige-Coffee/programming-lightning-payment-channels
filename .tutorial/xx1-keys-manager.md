# Lightning Wallet

Before we can begin our journey into programming Lightning network transactions, we'll need to build a wallet that can provide us with all of the public and private keys we'll need -- there are many! If you'd like a quick refresher on elliptic curve cryptography and what a public/private key is, **learn me a bitcoin** provides an excellent resource [here](https://learnmeabitcoin.com/technical/cryptography/elliptic-curve/).

This section is a little like eating your vegetables; it may not be your favorite thing to do, but it's a required pre-requisite to gain a strong and intuitive understanding of how Lightning works.

## Starting With The End In Mind
Let's start by reviewing our end goal: **We need to create unique public and private keys that can be used for each spending path in our commitment transaction outputs**. If this doesn't make sense to you yet, don't worry! The rest of this course will dive into commitment transactions in excrutiating detail. For now, we simply want to gently introduce the fact that there are **multiple types of public keys** and provide an intuition for **where they will be imbeded in the transaction**.

For the rest of this course, **we'll play the part of Alice** in implementing and operating our Lightning implementation. In the below diagram, you can see each public key that Alice will provide for this arbitrary channel state between Alice and Bob. NOTE: for simplicity, the Hash-Time-Locked-Contract (HTLC) output is *not* pictured, but Alice will have an HTLC public key embedded within the HTLC output for both her commitment transaction and Bob's commitment transaction. Again, if the words "HTLC" and "commitment transaction" don't make sense, that's totally okay!

The most important thing to take away from this diagram is the following:
- We'll need **different** public keys - one for each spending path.
- *Most* of the public keys that are placed in each spending path are a **combination** of two public keys: a **basepoint** and a **per commitment point**. We'll cover both of these shortly.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/basepoint_keys.png" alt="basepoint_keys" width="100%" height="auto">
</p>

Below is a list of the basepoints and basepoint secrets used in the Lightning Network. For now, we'll just be specifying their name and data structure. As we begin to program our Lightning channels, we'll learn *much* more about each and their specific purpose.
  - **Revocation Basepoint Secret**: Secret Key
  - **Revocation Basepoint**: Public Key
  - **Payment Basepoint Secret**: Secret Key
  - **Payment Basepoint**: Public Key
  - **Delayed Payment Secret**: Secret Key
  - **Delayed Payment**: Public Key
  - **HTLC Basepoint Secret**: Secret Key
  - **HTLC Basepoint**: Public Key
  - **Commitment Seed**: The commitment seed is a 256-bit scalar used to generate a series of secrets **for each commitment state**. As we'll soon learn, these secrets will combined with the above basepoints/secrets to generate the private and public keys used in each commitment transactions output scripts.

#### Question: Now that we've reviewed the various types of public and private keys we'll need to program our Lightning implementation, can you think of an effective way to organize these keys?
<details>
  <summary>Answer</summary>

To answer this question, we must discuss what it means to have an "effective" way to organize our keys. For instance, one possible solution is that we simply generate a new, random private key for each key type we'll need. In this scenario, we'd have five secrets we need to safely manage. That doesn't sound efficient or safe!

Another option is to leverage **BIP32 hierarchical deterministic (HD) key derivation**, whereby we can use one single seed and deterministially generate a series of *child* public and private keys from that seed. This is much more efficient and safe, as we only need to safely manage one seed.

Next, we'll explore using BIP32 to derive all of the keys we'll need to implement our Lightning channel.

</details>

## Lightning Off-Chain Wallet Strucure
At this point, we have a general idea of *which* keys we'll need to use in our Lightning implementation. That said, we don't yet know what they will be used for, but that will come in due time! Fun Fact: the following key derivation is actually very similar to how the [Lightning Network Deamon (LND)](https://github.com/lightningnetwork/lnd) works.

Let's set the scene by briefly reviewing **Bitcoin Improvement Proposal (BIP) 32**.

BIP 32  describes a **hierarchical deterministic** (**HD**) wallet structure and introduces the following characteristics for key management:
- **Single Source**: All public and private keys can be derived from a single seed. As long as you have access to the seed, you can re-derive the entire wallet.
- **Hierarchical**: All keys can be organized in a tree structure.
- **Deterministic**: All keys are generated the same exact way. Each time you restore you wallet from your seed, you'll get the exact same result.

### Derivation Paths
A few important BIPs, such as [BIP 43](https://bips.dev/43/) and [BIP 44](https://bips.dev/44/), build on BIP 32 and describe the following derivation scheme that can be used to organize keys.

```
m / purpose' / coin_type' / account' / change / address_index
```

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/BIP84.png" alt="BIP84" width="80%" height="auto">
</p>

Here is how to interpret the above scheme:
- `m`: This is the master extended key for the wallet.
- `/`: Whenever you see this, we are deriving a new child key.
- `purpose'`: The purpose specifies the wallet structure. The value in the purpose field typically reflects the BIP that describes the wallet scheme for that output type. For example, `m/84'` would mean that this wallet structure follows the derivation scheme described in [BIP 84](https://bips.dev/84/) and uses Pay-To-Witness-Public-Key-Hash (P2WPKH) serialization format. Since there is a `'`, we know this path is hardened.
- `coin_type'`: This represents the cryptocurrency that we're derving keys for. A coin type path was included so that hardware wallets can support multiple cryptocurrencies using a single seed. For example, `0` is Bitcoin, `1` is also Bitcoin (Not mainnet, so testnet, regtest, etc.), `2` is Litecoin. You can see the list [here](https://github.com/satoshilabs/slips/blob/master/slip-0044.md).
- `account'`: This allows wallet users to create separate "accounts" to separate their funds.
- `receiving/change`: This field separates into a **receiving** (`0`) index and **change** (`1`) index such that users can generate separate addresses, depending on if they are receiving payments or generating change addresses. NOTE: these are **normal children**, meaning they will have corresponding **extended public keys** which can derive child public keys without needing to know the private key.
- `index`: The index field specifies the actual keys used to generate addresses and receive bitcoin. The above levels in the HD wallet provide the structure that ultimately points to one of these keys, enabling efficient and deterministic organization.


### Implementing Our Wallet
We'll leverage the HD wallet structure to build our Lightning wallet, as this will enable us to derive all of the keys we need from a single seed. Below is an image depicting our wallet architecture. 

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/our_keys.png" alt="our_keys" width="80%" height="auto">
</p>

Here is how to interpret the above scheme:
- `m`: This is the master extended key for the wallet, derived from our wallet's seed.
- `purpose'`: We'll use `1017'` for the purpose. This is actually the same value that LND uses. It's an arbitrary choice and is not specified in any bitcoin or lightning protocol specification. We're simply using it as a unique value to plug into the derivation scheme.
- `coin_type'`: Since we're running regtest for this course, we'll use `1'` for this field.
- `account'`: This is where the magic happens. We'll specify a specific **key family** for each `account`. This will enable us to deterministically derive unique public and private keys for each channel our Lightning node opens.
- `receiving`: We won't be generating any change addresses with this field, so we'll keep `0` (receiving) as a defualt value here.
- `index`: The index will be unique for each channel we open.

By leveraging this architecture, we can create all of the public key, private key, and seed information that we'll need to operate our Lightning channel! Remember, at this point, you don't need to understand what these keys are used for yet. What's important is that you understand *how* we can create these keys.

## ⚡️ Build a KeysManager
Let's put the above theory into practice! Over the next few exercises, we're going to use [rust bitcoin](https://docs.rs/bitcoin/latest/bitcoin/) to implement an HD wallet that can manage and derive the keys we'll need for our lightning node. Cool, eh!

Let's start by creating a new `KeysManager` from a seed (random 256-bit value). The `KeysManager` is a custom type defined in `src/exercises/types.rs` and can be seen in the dropdown below.

<details>
  <summary>Click to see KeysManager</summary>

As you can see, the `KeysManager` is a simple struct that holds three essential components for our HD wallet:

1. **`secp_ctx`** - A secp256k1 context that we'll use for all cryptographic operations (signing transactions, deriving public keys, etc.).

2. **`master_key`** - The root extended private key (`Xpriv`) for our HD wallet. This is derived directly from our seed and serves as the starting point for deriving all other keys we'll need. This is the master key (`m`) in the derivation path above!

3. **`network`** - The Bitcoin network we're operating on (mainnet, testnet, regtest, etc.). This is important because, as we learned earlier, the key derivation will differ depending on the network.

```rust
pub struct KeysManager {
    pub secp_ctx: Secp256k1<All>,
    pub master_key: Xpriv,
    pub network: Network,
}
```

</details>

Head over to `src/exercises/keys/derivation.rs`, and let's implement our first function!

This function, `new_keys_manager`, will take a `seed` and Bitcoin `network` and return a `KeysManager` with the master key that will anchor our entire Lightning wallet.

```rust
pub fn new_keys_manager(seed: [u8; 32], network: Network) -> KeysManager {
    let secp_ctx = Secp256k1::new();
    let master_key = Xpriv::new_master(network, &seed).expect("Valid seed");

    KeysManager {
        secp_ctx,
        master_key,
        network,
    }
}
```

<details>
  <summary>Step 1: Initialize the Secp256k1 Context</summary>

Since we'll be performing cryptographic operations in this exercise, we'll need to start by defining a variable that can perform those cryptographic operations for us. To do that, we can use the `Secp256k1` crate.

```rust
let secp_ctx = Secp256k1::new();
```
</details>

<details>
  <summary>Step 2: Derive the Master Key from the Seed</summary>

Remember, our function takes a 32-byte `seed` as input, so we can use it to generate a BIP-32 extended private key (master key) from which all other keys will be derived!

To do this, we can leverage the `Xpriv` type, provided by the `bip32` crate in rust-bitcoin. Since the derivation path will depend on the network type, we'll need to pass 'network' the `new_master` function, available on the `Xpriv` type.

This function will return a `Result` type, so we'll need to unwrap it before we can use it. In a robust application, we will want to handle this error more precisely.

```rust
let master_key = Xpriv::new_master(network, &seed).unwrap();
```
</details>

<details>
  <summary>Step 3: Construct and Return the KeysManager Struct</summary>

Finally, let's assemble all the above components into our `KeysManager` struct and return it!

```rust
KeysManager {
    secp_ctx,
    master_key,
    network,
}
```
</details>


## ⚡️ Derive Private Key
Now that we have our `KeysManager`, which holds our master key, `m`, let's implement a function that can derive a private key from the derivation path we reviewed above. This function will enable us to derive all of the private keys (and, subsequently, public keys) that we'll need to use for our Lightning transactions.

```
m / purpose' / coin_type' / key_family' / change / channel_id_index
```

To complete this exercise, you'll need to implement the `derive_key` on the `KeysManager` that we initialized in the prior transaction. When implementing -- accomplished using the `impl` keyword -- a function in Rust, the function you're implementing will have access to any internal fields that the struct contains. In this case, you'll want to use the master key to derive new child private keys at the specified derivation path!

Speaking of derivation paths, there is a new type that we'll need to use for this exercise: `KeyFamily`. As we learned earlier, the **key family** acts as the `account` in our derivation path and specifies the use of the keys being derived. Similar to `KeysManager`, `KeyFamily` is a custom type defined in `src/exercises/types.rs`. Click the dropdown to learn more.

<details>
  <summary>Click to see KeyFamily</summary>

The `KeyFamily` enum defines the different **types** of keys our Lightning node needs to derive. Each variant corresponds to a specific use case in Lightning channels:
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyFamily {
    MultiSig = 0,
    RevocationBase = 1,
    HtlcBase = 2,
    PaymentBase = 3,
    DelayBase = 4,
    CommitmentSeed = 5,
    NodeKey = 6,
}
```

Each `KeyFamily` variant has a numeric value that slots into the **key family** position of our derivation path:
```
m / 1017' / 0' / <key_family>' / 0 / <channel_id_index>
                      ↑
                This number!
```

For example, when we need to derive a key for the funding multisig, we'd use `KeyFamily::MultiSig` (which equals `0`), giving us the path `m/1017'/0'/0'/0/<channel_id_index>`.

</details>

```rust
impl KeysManager {
    pub fn derive_key(&self, key_family: KeyFamily, channel_id_index: u32) -> SecretKey {
        // Path: m/1017'/0'/<key_family>'/0/<channel_id_index>
        let path_str = format!("m/1017'/0'/{}'/0/{}", key_family as u32, channel_id_index);
        let path = DerivationPath::from_str(&path_str).expect("Valid derivation path");

        let derived = self
            .master_key
            .derive_priv(&self.secp_ctx, &path)
            .expect("Valid derivation");

        derived.private_key
    }
}
```

<details>
  <summary>Step 1: Construct the Derivation Path String</summary>

The first thing we need to do is build our BIP-32 derivation path as a string. Remember, our path follows this structure:
```
m / 1017' / 0' / <key_family>' / 0 / <channel_id_index>
```

We can use Rust's `format!` macro to build this string dynamically, plugging in our `key_family` and `channel_id_index` values. Note that we need to cast `key_family` to a `u32` to get its numeric value!
```rust
let path_str = format!("m/1017'/0'/{}'/0/{}", key_family as u32, channel_id_index);
```

</details>

<details>
  <summary>Step 2: Parse the String into a DerivationPath</summary>

Now that we have our path as a string, we need to convert it into a `DerivationPath` type that rust-bitcoin can actually use for key derivation. The `DerivationPath::from_str()` function handles this for us.

This function returns a `Result`, so we'll use `.expect()` to unwrap it. In production code, you'd want more robust error handling, but for our educational purposes, this works great!
```rust
let path = DerivationPath::from_str(&path_str).expect("Valid derivation path");
```

</details>

<details>
  <summary>Step 3: Derive the Child Private Key</summary>

Here's where the magic happens! We'll use our `master_key` to derive a child private key at the specified path. The `derive_priv` method takes two arguments: our secp256k1 context (for crypto operations) and the derivation path we just created.

Since this is an `impl` function, we can access the `KeysManager`'s internal fields using `self`. Pretty convenient!
```rust
let derived = self
    .master_key
    .derive_priv(&self.secp_ctx, &path)
    .expect("Valid derivation");
```

</details>

<details>
  <summary>Step 4: Extract and Return the Secret Key</summary>

The `derive_priv` function returns an `Xpriv` (extended private key), but we just need the raw `SecretKey` for our Lightning operations. We can extract it from the `private_key` field and return it!
```rust
derived.private_key
```

</details>

## ⚡️ Derive Channel Keys
Great work! We're well on our way to building a wallet that can power our Lightning implementation. Let's continue this journey by building a function that can generate all of the keys and cryptographic material we'll need to operate our Lightning channel.

To successfully complete this exercise, you'll need to implement `derive_channel_keys` on our `KeysManager`. This function will take a `channel_id_index`, which we can use for the index of our derivation path, thus creating unique keys for each channel.

This function will return a `ChannelKeyManager`, which is a struct that holds all of the cryptographic material we need for a given channel. To complete this exercise, you'll need to derive the correct key for each key family using the `derive_key` function we just implemented, then assemble them all into a `ChannelKeyManager`.

<details>
  <summary>Click to see ChannelKeyManager</summary>

The `ChannelKeyManager` holds the specific set of keys for a **single Lightning channel**.
```rust
pub struct ChannelKeyManager {
    pub funding_key: SecretKey,
    pub revocation_base_key: SecretKey,
    pub payment_base_key: SecretKey,
    pub delayed_payment_base_key: SecretKey,
    pub htlc_base_key: SecretKey,
    pub commitment_seed: [u8; 32],
    pub secp_ctx: Secp256k1,
}
```
</details>

```rust
impl KeysManager {
    pub fn derive_channel_keys(&self, channel_id_index: u32) -> ChannelKeyManager {
        // Use derive_key for each key family
        let funding_key = self.derive_key(KeyFamily::MultiSig, channel_id_index);
        let revocation_base_key = self.derive_key(KeyFamily::RevocationBase, channel_id_index);
        let payment_base_key = self.derive_key(KeyFamily::PaymentBase, channel_id_index);
        let delayed_payment_base_key = self.derive_key(KeyFamily::DelayBase, channel_id_index);
        let htlc_base_key = self.derive_key(KeyFamily::HtlcBase, channel_id_index);
        let commitment_seed = self
            .derive_key(KeyFamily::CommitmentSeed, channel_id_index)
            .secret_bytes();

        ChannelKeyManager {
            funding_key,
            revocation_base_key,
            payment_base_key,
            delayed_payment_base_key,
            htlc_base_key,
            commitment_seed,
            secp_ctx: self.secp_ctx.clone(),
        }
    }
}
```
Remember, at this point, it's not vital that you know what each key is used for. We're purposfully holding that information off until later, because each key will be introduced when we need it, which will hopefully make for a more intuitive and fruitful learning experience.

<details>
  <summary>Step 1: Derive the Funding Key</summary>

Let's start by deriving the `MultiSig` key family. We'll call our `derive_key` function and specify the correct key family. It's also worth noting that you don't necessarily need to derive this key first to pass this exercise. For simplicity and continuity, we derive keys in the same order as their index. 

Since this is inside an `impl KeysManager` block, we can access the `derive_key` function using `self`.

```rust
let funding_key = self.derive_key(KeyFamily::MultiSig, channel_id_index);
```

</details>

<details>
  <summary>Step 2: Derive the Revocation Base Key</summary>

Next up is the `RevocationBase`. This is very similar to the last piece of code you wrote!

```rust
let revocation_base_key = self.derive_key(KeyFamily::RevocationBase, channel_id_index);
```

</details>

<details>
  <summary>Step 3: Derive the Payment Base Key</summary>

At this point, you're a pro. You know what to do!

```rust
let payment_base_key = self.derive_key(KeyFamily::PaymentBase, channel_id_index);
```

</details>

<details>
  <summary>Step 4: Derive the Delayed Payment Base Key</summary>

Let's derive another!

```rust
let delayed_payment_base_key = self.derive_key(KeyFamily::DelayBase, channel_id_index);
```

</details>

<details>
  <summary>Step 5: Derive the HTLC Base Key</summary>

If you've heard that Lightning has lots of keys, you heard correctly!

```rust
let htlc_base_key = self.derive_key(KeyFamily::HtlcBase, channel_id_index);
```

</details>

<details>
  <summary>Step 6: Derive the Commitment Seed</summary>

Okay, hold on a second! The `CommitmentSeed` is a bit different from the other keys. We derive it almost the exact same way, but we need to store it as raw bytes (a 32-byte array) rather than a `SecretKey` type. Luckily, `SecretKey` has a handy `.secret_bytes()` method that gives us exactly what we need!

As we'll shortly see, we store the raw bytes because we use these bytes as a **seed** and **not** an **private key**. 

```rust
let commitment_seed = self
    .derive_key(KeyFamily::CommitmentSeed, channel_id_index)
    .secret_bytes();
```

</details>

<details>
  <summary>Step 7: Construct and Return the ChannelKeyManager</summary>

Finally, let's bundle all these keys together into a `ChannelKeyManager` struct and return it! We'll also clone the secp256k1 context so our `ChannelKeyManager` can perform its own cryptographic operations.
```rust
ChannelKeyManager {
    funding_key,
    revocation_base_key,
    payment_base_key,
    delayed_payment_base_key,
    htlc_base_key,
    commitment_seed,
    secp_ctx: self.secp_ctx.clone(),
}
```

</details>