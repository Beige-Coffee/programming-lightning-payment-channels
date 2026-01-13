# Developing Our Penalty Mechanism

## Introducing Our Penalty Mechanism (Gently)

Now that we have a foundational understanding of asymmetric commitment transactions, we can begin to implement our penalty mechanism. Before digging into cryptography and getting in the weeds, let's review how things work at a higher level. To do this, we'll temporarily break a core tenet of Bitcoin and introduce a trusted third party. Don't worry, this trusted third party is ***only*** for educational purposes, as it will make it much easier to conceptually grasp how Lightning's penalty mechanism works. Once we have an intuitive understanding of what's going on, we'll replace the trusted third party with a series of cryptographic operations, making everything trustless again!

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/simple_revocation_key.png" alt="simple_revocation_key" width="100%" height="auto">
</p>

### Step 1

Imagine that, for each commitment transaction, a trusted third party generates a unique public key for Alice and a unique public key for Bob. **Alice and Bob take their respective public keys and add them to the output scripts on their own version of the commitment transaction that pays themselves** (also known as the `to_local` outputs). In other words, Alice adds this public key to the output that represents her side of the channel balance. Bob similarly adds his public key to the output on his version of the commitment transaction that pays himself. You can see this visually in the image above. Specifically, you'll notice that:

- Alice's `to_local` output has the following two spending paths:
  - One is locked to the new public key provided by the third party. We'll call this the **"Revocation Path"**.
  - The second is locked to Alice's **Delayed Payment Public Key**. It's called the Delayed Payment Public Key because, as you can see, Alice cannot spend from this path until `to_self_delay` blocks have been mined. This delay is specified as a relative timelock, which starts once Alice's version of the commitment transaction is confirmed.

The reason for adding the delay will become clear, if it's not already!

> ❗**NOTE:** At this point, neither Alice nor Bob know the private keys to either of the public keys that the trusted third party provided. This is a ***very*** important property because it means that, if either party attempts to broadcast a transaction from the **current state**, neither would know the private key to spend from the "revocation path". This makes it safe to publish the *current* state. Since your counterparty doesn't have the private key to spend from the revocation path, the only possible spending path would be via the **Delayed Payment Public Key**, which only you have the private key to! For example, if Alice published her version of the commitment transaction from Channel State 1 ***before*** moving on to Channel State 2, then neither Alice nor Bob can spend from the "revocation path". Therefore, Alice's funds are safe because only she can generate a signature to spend from her Delayed Payment Path!

### Step 2

When Alice and Bob decide to move to a new channel state, the trusted third party will do the following:

1. Provide a new public key for both Alice and Bob to use in their new commitment transactions (Channel State 2). As in Step 1, these public keys will go in their respective "Revocation Paths".
2. Provide Alice and Bob the private keys to **the prior state of the other person's commitment transaction**. For example, in the diagram above, once we reach Channel State 2, Alice will have the private key to spend from Bob's Revocation Path, and Bob will have the private key to spend from Alice's Revocation Path. **However, neither Alice nor Bob can spend from their own revocation path**.

### Step 3 (If someone cheats)

If Alice or Bob cheats, that, **by definition**, means they published an old commitment state. For example, let's say we're in **Commitment State 2**, but Alice publishes **her** commitment transaction from **Commitment State 1**. She is attempting to steal 1M sats back from Bob by publishing an old transaction that does not have these sats on Bob's side of the channel. Since our trusted third party gave Bob the private key to spend from **Alice's revocation path**, Bob can generate a signature to claim Alice's `to_local` output. Remember, Alice does not have the private key for this spending path, so she cannot claim her output via the **Revocation Path**.

The above mechanism ensures that old commitment states are effectively revoked, because publishing old commitment states risks losing all of your funds in the channel. Cool, eh?

#### Question: Why is the delayed payment spending path delayed with a relative timelock? Why do we need a timelock at all?

<details>
  <summary>Answer</summary>

The timelock is a crucial part of Lightning's security model. To see why, let's imagine there was no timelock and Alice cheats by publishing an old commitment state. In this scenario, there is nothing stopping Alice from spending her output immediately! Therefore, even though Bob has the private key to spend from the "Revocation Path", he wouldn't have sufficient time to act, so it's effectively worthless. Furthermore, you could imagine a scenario where Alice submits the transaction directly to a miner, so Bob will never even see the old commitment transaction in his mempool! He'll only see it after it's mined. 😞

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/revocation_no_timelock.png" alt="revocation_no_timelock" width="80%" height="auto">
</p>

By adding a relative timelock, **which doesn't start until the transaction is mined**, we guarantee that Bob has sufficient time to spot the transaction if it's mined. For example, if the delay is 2016 blocks (~2 weeks), then Alice cannot reclaim her funds until 2 weeks *after* the transaction has been mined. This gives Bob plenty of time to spend from the Revocation Path, if he has the private key.

If you've ever heard that Lightning has a "liveness" requirement, this is it! In other words, to safely operate on the Lightning Network, you need to be regularly online and monitoring the blockchain so you can spot if your counterparty attempts to cheat you. NOTE: you don't need to be "live" 24/7 to operate safely. As we saw in the above example, you just need to come online and check the blocks during the 2-week window to spot if your counterparty cheated. That said, it's still best practice to be consistently connected and monitoring the blockchain.

</details>


## Introducing Revocation Keys

Okay, now that we've reviewed the gist of how our penalty mechanism works, let's inch our way towards the actual protocol implementation by removing the third party and replacing them with cryptography. Remember, our end goal is to:

1. Create public keys, which we'll call **Revocation Public Keys**, that Alice or Bob can spend from if their counterparty attempts to cheat.
2. Ensure that neither Alice nor Bob knows the private key to their own **Revocation Public Key**.
3. When advancing to a new channel state, enable Alice and Bob to obtain (or, more specifically, calculate) the private key to **their counterparty's Revocation Public Key**.

This way, each party provides a way for their counterparty to claim their funds ***if and only if*** they cheat and publish an old transaction.

### Calculating A Revocation Public Key And Private Key

Below is a diagram showing, roughly, how Alice and Bob can exchange public and private key information in such a way that they satisfy the properties listed above. When viewing the diagram, also take note of how the keys tie back to the public and private keys that we created at the beginning of the course.

When you see a gear icon (see below for example), it means we're performing some type of algorithm or cryptographic operation. During our first pass, we'll explain what the algorithm *produces*, but we won't review how it works just yet. After we discuss the overall flow, we'll dig into the actual equations and, of course, implement them ourselves in code!

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/operation.png" alt="operation" width="5%" height="auto">
</p>

Let's get to it!

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/revocationSteps.png" alt="revocationSteps" width="100%" height="auto">
</p>

<details>
  <summary>Click to see how the above diagram relates to the "gentle" introduction</summary>

Honestly, this diagram might be an absolute mess and make little sense. If that's the case, I appologize in advance. Otherwise, I hope it's useful and makes it a little more clear how all of the cryptographic material connects back to the "gentle" introduction we just reviewed.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/diagram_connection.png" alt="diagram_connection" width="100%" height="auto">
</p>

</details>