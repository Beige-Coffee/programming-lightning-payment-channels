# HTLC Offerer

Alright, we're getting a little ahead of ourselves! At this point, we've learned how we can add and remove HTLCs from commitment transactions, while revoking old channel states along the way. It's pretty cool stuff. That said, we still haven't learned what the HTLC script looks like!

There are a few reasons we've pushed this off. First, the script will change depending on if you're **offering** or **receiving** the HTLC. Second, the scripts are pretty... complicated.

In our ongoing example, Alice is **offering** the HTLC, since she is attempting to make a payment through Bob. On the other hand, Bob is **receiving** the HTLC, since he is accepting the HTLC and, if successful, the HTLC funds would be moved to Bob's side of the channel.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/htlc_offerer.png" alt="htlc_offerer" width="100%" height="auto">
</p>

## HTLC Offerer Pseudo Script

The HTLC scripts are not easy on the eyes, so let's inch our way towards them, starting with pseudo script. As a reminder, "pseudo script" means we'll be writing the script in plain english, describing the spending paths that will eventually need to be implemented in Bitcoin's native programming language, [**Script**](https://en.bitcoin.it/wiki/Script).

Alice, the ***HTLC offerer***, has to create an HTLC output where:
1) **Bob** can spend the output if he has the **Revocation Private Key** for this specific commitment state. This protects Bob in the future if Alice attempts to cheat by publishing this HTLC commitment transaction **after** they have agreed to move to a new channel state.
2) **Bob** can spend the output if he has the **preimage**.
3) **Alice** can spend (effectively, reclaim) the output if the **HTLC expires**. Remember, Alice proposed a block height at which the HTLC expires.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/AliceNoTimeoutTx.png" alt="AliceNoTimeoutTx" width="100%" height="auto">
</p>

**This is very nuanced and hard to see, but there is actually a bit of a dilemma here!** Remember, all `to_self` spending paths ***must*** be delayed by `to_self_delay` blocks to ensure that Bob has time to cliam the funds if Alice cheats (and vice versa). Therefore, Alice must timelock her spending path using `OP_CHECKSEQUENCEVERIFY`, which is a **relative timelock** that only starts once the transaction is mined.

#### Question: Looking at the simplified transaction, can you spot why this commitment transaction structure would be a problem? HINT: it has to do with the timelocks.
<details>
  <summary>
    Answer
</summary>

The HTLC needs to be timelocked with an **absolute timelock** set to some block height in the future. This ensures that Bob can safely forward the HTLC without Alice arbitrarily deciding to expire the contract early. If Alice was able to expire the contract whenever she pleased, there is a chance she expires the contract ***after*** Bob pays Dianne but ***before*** Bob claims his funds from Alice.

Additionally, as we just discussed, Alice's expiry path has a **relative timelock**, which provides Bob with a `to_self_delay` block delay to sweep Alice's funds via the Revocation Path, if Alice attempts to cheat in the future.

If we tried to fit all of these conditions into the same output, we'd run into a bit of an issue. Namely, the **relative timelock**, `OP_CHECKSEQUENCEVERIFY`, does not start until the transaction is mined, which can only happen *after* the **absolute timelock** is satisfied.

This means, in the worst case, if Alice times out the HTLC once the contract expires, **she will still have to wait another** `to_self_delay` **blocks before she can spend the output**. This gives Bob extra time to provide the preimage and claim the output. Do you know how we can fix this?

</details>

## Addressing The Dilemma
To fix this timelock dilemma, we'll add a second transaction for Alice, known as the **HTLC Timeout Transaction**. This transaction will use the same script as our `to_local` output, however, it will feature the following differences:
1) The transaction with have a `Locktime` set to the **HTLC's block height expiration**, known as `cltv_expiry` (Check Locktime Verify Expiry) in the protocol. This means that the transaction cannot be mined until the `cltv_expiry` block height has passed.
2) The **input** for this transaction will be the **HTLC output** from the commitment transaction.
3) The HTLC Timeout Transaction transaction will spend from a **2-of-2 multisig path in the commitment transaction's HTLC output script**. Therefore, it will require signatures from both Alice and Bob to spend. **Alice and Bob will pre-sign the HTLC Timeout Transaction when creating the HTLC output on their commitment transactions**, effectively both agreeing to the "expiry" terms of the contract.
    - ❗NOTE: As we'll see shortly, the public keys used for the 2-of-2 mutisig and the associated signatures for the HTLC Timeout Transaction are derived from the **HTLC Basepoint**.
   
<details>
  <summary>
    Click to see how Alice and Bob exchange signatures for the HTLC Timeout Transaction
</summary>

Remember how the `commitment_signed` message had a field for `htlc_signatures`, and we said we'd discuss it later? Well, now it's "later"!

When Alice and Bob add new HTLCs to their commitment transactions, they will exchange the signatures needed to spend from the HTLC output (on the commitment transaction) using the HTLC Timeout transaction. SPOILER ALTER: Bob will also have a second HTLC transaction, called the "HTLC Success Transaction", which is why Alice sends him an HTLC signature.

In the diagram below, you'll see that, when adding an HTLC and moving to a new channel state, Bob sends Alice an HTLC Signature, which Alice will use in her **HTLC Timeout Transaction** if she needs to publish it on-chain. Bob is able to create Alice's **HTLC Timeout Transaction** locally and send her the signature because he has all the information he needs locally to do so. Remember, Alice gave Bob her **Delayed Payment Basepoint** and **Revocation Basepoint** when they opened the channel, and Alice sent Bob the `cltv_expiry` in the `update_add_htlc` message, so Bob has everything he needs to generate the signature for Alice.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/htlc_sig_exchange.png" alt="htlc_sig_exchange" width="100%" height="auto">
</p>

</details>


Together, these updates allow Alice to enforce the **absolute timelock** and **relative timelock**. This is because Alice cannot publish the **HTLC Timeout Transaction** until the HTLC expires at block height `cltv_expiry`. Furthermore, once published, she will need to wait an additional `to_self_delay` blocks (via the `OP_CHECKSEQUENCEVERIFY` timelock) to claim her funds from this output.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/Alice2ndStageTx.png" alt="Alice2ndStageTx" width="100%" height="auto">
</p>


## Putting It All Together

As a brief review, the HTLC output has the following spending conditions:

1) **Revocation Path**: If Bob has the revocation secret (in case Alice cheats by broadcasting an old transaction), he can immediately claim the output.
2) **Preimage Path**: If Bob provides the preimage, he can immediately claim the output.
3) **Expiry Path**: If Bob doesn't provide the preimage, Alice can claim the output via the **2-of-2 multisig path**, which must be claimed using the **HTLC Timeout Transaction**. This allows Alice to expire the HTLC and reclaim her funds - while also enforcing the `to_self_delay`.

For the HTLC Timeout Transaction, we have the following spending paths:
- **Revocation Path**: Bob can claim the output immediately with the revocation secret.
- **Delayed Path**: Alice can claim the output after the `to_self_delay`.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/HTLCOffererTx.png" alt="HTLCOffererTx" width="100%" height="auto">
</p>

Now that we can see the full HTLC script, let's draw attention to the **`OP_CHECKMULTISIG`** spending path, which sends to the **HTLC Timeout Transaction**. If Bob does not cooperate to "unwind" the HTLC by either fulfilling or failing it, then Alice's only option to expire the HTLC is to publish the **HTLC Timeout transaction**. Since the **HTLC Timeout transaction** requires one signature from Alice and one from Bob, both parties effectively agree on the expiry terms when they exchange signatures during the process of adding the HTLC. This way, Bob can rest assured that Alice is only able to expire the HTLC using the **HTLC Timeout Transaction**, which Bob knows has an absolute timelock set to `cltv_expiry`.


If you're interested in seeing a full breakdown of the HTLC script to better understand how to spend from each path, see the dropdowns below!

<details>
  <summary>
    Click here to see an in-depth breakdown of spending from the HTLC Timeout Path
</summary>
    
<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/htlc_timeout_eval.png" alt="htlc_timeout_eval" width="100%" height="auto">
</p>

</details>

<details>
  <summary>
    Click here to see an in-depth breakdown of spending from the Preimage Path
</summary>

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/preimage_htlc_eval.png" alt="preimage_htlc_eval" width="100%" height="auto">
</p>

</details>

## Deriving HTLC Keys

Now that we have a better understanding of our HTLC scripts and how the HTLC public keys will play a part, let's review how they are derived.

### Public Keys
HTLC public keys and secrets are derived exactly the same as the **Delayed Public Keys**, which we reviewed earlier. As a reminder, to derive the public key, you use the following formula, where:
- `basepoint` = The basepoint of the key you're deriving (ex: **Delayed Payment Basepoint**, **HTLC Basepoint**)
- `per_commitment_point` = The **Per-Commitment Point** for the given commitment transaction.

```
pubkey = basepoint + SHA256(per_commitment_point || basepoint) * G
```

### Private Keys

The corresponding private keys are similarly derived, but we use the **Basepoint Secrets** instead of the **Basepoints**.


```
privkey = basepoint_secret + SHA256(per_commitment_point || basepoint)
```

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/htlc_derivation.png" alt="htlc_derivation" width="100%" height="auto">
</p>


### Question: What should we do if Alice wants to forward a payment that is below the dust limit?

<details>
  <summary>
    Answer
</summary>

If you've ever used Lightning to send a small payment, such as 1 sat, you know first-hand that it's **possible** to send payments that are below the dust limit. However, it doesn't make sense to add an output for such a payment, as creating the output would cost more than the output is worth.

That's why, per [BOLT 3](https://github.com/lightning/bolts/blob/master/03-transactions.md#commitment-transaction-outputs), any HTLC payments (minus fees) below the `dust_limit_satoshis` threshold are added to the transaction fees during routing. It's worth elaborating on two things:
- `dust_limit_satoshis` is specified by each channel party, allowing them to set **their own** threshold for amounts they won't create commitment outputs for. If the two parties have different dust limits, their commitment transactions won't be identical: one might include an HTLC output, while the other "trims" it (adding the value to fees instead). This is a neat feature of Lightning that is enabled by the fact that commitments are asymmetric. Since Alice and Bob exchange important information when they start their channel, they will know each other's dust limit and create their signatures (which they send to each other) appropriately.
- When determining if an HTLC is below the dust limit, you must also factor in fees for the second-stage transaction **if your Lightning channel does not support anchor outputs or zero fee commitments**. For example, if you're routing a payment for 600 sats, but the second-stage Timeout transaction fees are 300 sats, your resulting P2WSH output would be below the P2WSH dust threshold of 330 satoshis. You can view the various dust thresholds in BOLT 3 [here](https://github.com/lightning/bolts/blob/master/03-transactions.md#dust-limits).



Below is a diagram showcasing how to calculate fees for the second-stage HTLC transaction. Remember, if the Lightning channel supports anchor outputs or zero-fee commitments, then you do not need to include fees on this transaction, as they can be included later. However, if a Lightning channel does not support anchor outputs or zero fee commitments, then you should trim HTLCs if, when including second-stage fees, they amount is below the dust threshold.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/htlc_fees.png" alt="htlc_fees" width="100%" height="auto">
</p>


*NOTE*: If your Lightning channel supports zero fee commitments, then trimmed HTLCs are added to the shared anchor output, until it reaches 240 sats.

</details>