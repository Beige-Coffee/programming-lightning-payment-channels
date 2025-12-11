# Closing Lightning Channels

Wow! Things got pretty intense, but, if you made it through, you now understand how transactions on the Lightning Network work! Remember, whether you're sending a payment to a direct channel partner (Alice -> Bob) or routing it through multiple hops (Alice -> Bob -> Dianne), you will always send payments via an **HTLC**. This simplifies protocol design and enhances privacy, as there is no discernible difference between receiving a payment from your channel partner or from someone else, routed through your channel partner.

At some point, Alice and Bob may want to close their Lightning channel and move their funds out of this 2-of-2 multisig. When either party decides to do this, they have a few options to get it done.

## Cooperative Closure

The best option, by far, is to initiate a **cooperative closure**. During a cooperative closure, Alice and Bob will work together to settle any pending HTLCs and then create a new transaction that locks their respective balances to simple **Pay-To-Witness-Public-Key-Hash** scripts with no timelocks. This way, Alice and Bob can both spend their funds immediately.

Alice and Bob also have the ability to specify which address they'd like to lock their funds to, which may be a separate wallet that is not related to their Lightning wallet.

## Force Closure

A valid, but sub-optimal, way of closing a channel is by initiating a **force closure**. During a force closure, either party will publish their version of the current commitment transaction with any applicable HTLC Timeout/Success transactions.

Force closures can be initiated for a variety of reasons, such as:

- One party goes offline for an extended period of time.
- Two parties cannot agree on essential operations, such as which feerate to use on commitment transactions.
- One party attempts to cheat the other by publishing an old state.

## 👉 Closing Our Lightning Channel

Ideally, we'd do everything in our power to initiate a **cooperative close** for our Lightning channel. However, we already went through the trouble of creating our **HTLC Timeout** transaction, so let's perform an ugly force close instead!

First, let's confirm that our funding transaction is still unspent. This will reinforce the concept that all of the transactions we just created during this workshop were off-chain.

Enter this command within your **Shell**, ensuring to replace the `funding_tx_id` with your **Funding Tx ID**. The `0` after just indicates that the funding input was at index 0 of this transaction, which is guaranteed for this course but is not always true.
```
getutxo funding_tx_id 0
```

You should see something like the below. Notice that this shows the amount of our channel, and the transaction has 6 confirmations from when we opened the channel (unless you mined more since then!).
```
{
  "bestblock": "4538053a50040431e239cbbde64d1949e2079863872be806129b96c43c517d40",
  "confirmations": 6,
  "value": 0.05000000,
  ...
}
```

## 👉 Publish Our HTLC Transaction

For this exercise, we'll go through the ugly process of force closing our channel, which has 1 pending HTLC. To do this, let's start by broadcasting our **HTLC Tx Hex**. You can do this by entering the below command in your **Shell**. Make sure to replace `htlc_tx_hex` with your **HTLC Tx Hex**.
```
sendtx htlc_tx_hex
```

You should see a **Tx ID** pop up, indicating that it was successfully broadcast.

## 👉 Publish Our HTLC Timeout Transaction

Now do the same with your **HTLC Timeout Tx Hex**!
```
sendtx htlc_timeout_tx_hex
```

Whoops! You should have gotten a `non-final` error. Take a moment to see if you can fix this yourself, and then try re-broadcasting the HTLC Timeout Transaction. If you need some help, click below.

<details>
  <summary>Help</summary>

Remember that our **HTLC Timeout Transaction** is timelocked! This is because we gave our channel party a specific amount of time to fulfill the HTLC, so we can't close the channel and claim these funds until the timelock expires.

The Replit code that created the **HTLC Timeout Transaction** locked the transaction until block height 200. Try mining 50 blocks (`mine 50`) and see if you can broadcast the transaction now.

</details>