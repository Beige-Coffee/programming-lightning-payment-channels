# Get Our First Commitment (Refund) Transaction

If you're reading this... congrats! You've made it to a **really** important checkpoint. We're going to use all of the code we've build thus far to generate our first commitment transaction. Since we're playing the part of Alice, this will be our "refund" transaction with will send our funds back to us if Bob goes offline.

<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/alice_refund_commit.png" alt="alice_refund_commit" width="60%" height="auto">
</p>

## 👉 Get Our First Commitment Transaction
Rememeber how we ran `cargo run -- funding` to generate our funding transaction? Well, we'll do something very similar for this exercise.

To prepare ourselves, we'll first need to open `transactions.txt` (in `src/exercises`) and get our Funding Transaction TxID. You should have recorded that when you created your Funding Transaction earlier. If you forgot, you can simply run `cargo run -- funding` again and record it now!

Once you have it, go to a **Shell** in your Repl and type in the below command. **Be sure to replace `txid` with the TxID of your Funding Transaction**!

```
cargo run -- commitment -t txid
```

Under the hood, this command is running the code in `src/interactive/commitment.rs` - feel free to explore this file, if you'd like!

If you do check out the file, you'll see that it's creating two sets of keys - one for us (Alice) and one for Bob. To do this, it's using the functions we created earler! Once the keys are created, it fetches the funding UTXO, which it can easily do because you provided the **Funding TxID** in the command! For this course, all funding outputs are guarenteed to be at output index 0, so we don't need to provide that in the command.

If you scroll through the rest of the code, you'll see it calls a few other functions that we've completed during this course:
- `create_commitment_transaction`: This creates an unsigned commitment transaction.
- `create_funding_script`: This creates the 2-of-2 multisig script, which is needed to generate a signature and pass into the witness.
- `sign_holder_commitmentment`: This generates our (Alice's) signature and adds it to the witness (along with Bob's signature and the witness script), resulting in a fully signed transaction ready to be broadcasted!

Once you run the command, you should see an output like this...

```
Tx ID:
5fbe34ddcd9d86f092611b52f38ca2dc63cc05d585b0ae74859f0dbe31fbed9d

Tx Hex: 02000000000101e80a0a05a9b6495c1a6259e450a5f339e0c615ebab523958e53360f5a9e0262600000000001ad8bb800234440f00000000001600148c4e98f51715d292104530224efba56176fb39b1b8d83c000000000022002069fa2c947fd3a8a103b076b46bc5bea6c1035fee25a14270c6a6e420d234a4ab0400483045022100cb6abfe33ec9a2bf83d06f2e46efb0d538b3de19d5d63342818f7d20e0f4e07c022034b86af66b84825dd0e0fd115f434af6d553de959c2f424c5ffe3d1ca94c0afa0147304402206080abdfbcc21fd8259bb74500e87d31eceeaf3a19064683e388ea26788b78e902206bec6fd181c14a1bccc75ae58a5cd671cfec2241b099b4d14a3d006eae6e8c670147522102744c609aeee71a07136482b71244a6217b3368431603e1e3994d0c2d226403af2103cfa114ffa28b97884a028322665093af66bb19b0cf91c81eae46e6bb7fff799a52ae8cfe0720
```

This is our commitment transaction! **Note: we have still NOT broadcasted our Funding Transaction yet**. That will come soon! Go ahead and copy the **Tx Hex** and **Tx ID** and save them in the file `src/exercises/transactions.txt` under the headings **Commitment Tx (No HTLCs) ID** and **Commitment Tx (No HTLCs) Hex**. 

# Decoding Our Commitment Transaction

Optionally, if you want to dig into the details, go ahead and run the below command in your shell, replacing `raw_tx_hex` with the transaction hex we just generated.

```
decodetx raw_tx_hex
```

You should get an output like the below. See if you can map this back to the image at the top of this page. You can also scroll down, and some of this will be described for you.

```
{
  "txid": "250e8eb27d7da5459b930be597c2c3bedd9639056985c4848ef6fc3fd7bf0286",
  "hash": "ee0b1d4910b8173d70205470a0cbffcb1c510ea7708ce2ec9502dd1cfaed0d5a",
  "version": 2,
  "size": 346,
  "vsize": 181,
  "weight": 721,
  "locktime": 537394828,
  "vin": [
    {
      "txid": "2626e0a9f56033e5583952abeb15c6e039f3a550e459621a5c49b6a9050a0ae8",
      "vout": 0,
      "scriptSig": {
        "asm": "",
        "hex": ""
      },
      "txinwitness": [
        "",
        "3045022100e396e4d418de174c68b372d073b8ac89ffaa0e3e8665f7beb5f87c8246a030e902202b19b6d4e1fc1a824d2d0521760f9012a2b0d31fc584b559c7e11e026add89bc01",
        "304402203b8e3d47b8b1d0a9b99265ffd179ba7327419e52fd006bc59db4d969956549ed02202d6bf4932d60f53f586e1c50fc6f5e0e97ba2cca361c50ff8b0b11a917e5e61d01",
        "522102744c609aeee71a07136482b71244a6217b3368431603e1e3994d0c2d226403af2103cfa114ffa28b97884a028322665093af66bb19b0cf91c81eae46e6bb7fff799a52ae"
      ],
      "sequence": 2159794202
    }
  ],
  "vout": [
    {
      "value": 0.00000500,
      "n": 0,
      "scriptPubKey": {
        "asm": "0 8c4e98f51715d292104530224efba56176fb39b1",
        "desc": "addr(bcrt1q338f3aghzhffyyz9xq3ya7a9v9m0kwd3v7t2jl)#euqsj43f",
        "hex": "00148c4e98f51715d292104530224efba56176fb39b1",
        "address": "bcrt1q338f3aghzhffyyz9xq3ya7a9v9m0kwd3v7t2jl",
        "type": "witness_v0_keyhash"
      }
    },
    {
      "value": 0.04987640,
      "n": 1,
      "scriptPubKey": {
        "asm": "0 69fa2c947fd3a8a103b076b46bc5bea6c1035fee25a14270c6a6e420d234a4ab",
        "desc": "addr(bcrt1qd8aze9rl6w52zqasw66xh3d75mqsxhlwyks5yuxx5mjzp5355j4sxljgh7)#lvf4jlja",
        "hex": "002069fa2c947fd3a8a103b076b46bc5bea6c1035fee25a14270c6a6e420d234a4ab",
        "address": "bcrt1qd8aze9rl6w52zqasw66xh3d75mqsxhlwyks5yuxx5mjzp5355j4sxljgh7",
        "type": "witness_v0_scripthash"
      }
    }
  ]
}
```

### Locktime
In the locktime field, you'll see a value that looks similar to the below:

```
"locktime": 537394828,
```

As we learned earlier when we created the **obscured commitment number**, we place the upper 24 bits of the **obscured commitment number** in the locktime field - prefixed with 0x20 (8 bits). 

This ensures the resulting locktime will evaluate to something above 536,870,912 but below 546,937,241. Since anything above 500,000,000 is interpreted as a Unix timestamp, and 536,870,912 - 546,937,241 is, roughly, around 1987, the locktime will always be a valid locktime in the past!

As we can see above, the locktime is a valid locktime in the past, and only we know how to combine it with the sequence field to learn the commitment number for this state! 

### Input

If you look at the `vin`, you'll see the one input for our commitment transaction - the 2-of-2 multisig output! You should recognize that the `txid` is equal to the one you passed in when you ran the command.

`vout` is 0 because, as mentioned earlier, all funding outputs **for this course** will be index 0, but that is not the case in the "real world" - they can be any index.

Since we're using SegWit (to prevent transaction maleability), the witness data has been moved out of the `scriptSig` and into the `txinwitnesss`, so the `scriptSig` is blank.
```
"vin": [
  {
    "txid": "2626e0a9f56033e5583952abeb15c6e039f3a550e459621a5c49b6a9050a0ae8",
    "vout": 0,
    "scriptSig": {
      "asm": "",
      "hex": ""
    },
```

### Witness

Take a moment and see if you can guess what is in the witness field!

Answer... Remember, since there is a bug in `OP_CHECKMULTISIG` which pops an extra item off the stack, we must first add an empty element ("").

Then, we add the two signatures for the public keys in the 2-of-2 multisig script.

Finally, we add the multisig script itself, since we previously locked to the hash, so we need to provide the script so that bitcoin can check if we truly are able to spend the coins locked in the input.

```
"txinwitness": [
"",
        "3045022100e396e4d418de174c68b372d073b8ac89ffaa0e3e8665f7beb5f87c8246a030e902202b19b6d4e1fc1a824d2d0521760f9012a2b0d31fc584b559c7e11e026add89bc01",
        "304402203b8e3d47b8b1d0a9b99265ffd179ba7327419e52fd006bc59db4d969956549ed02202d6bf4932d60f53f586e1c50fc6f5e0e97ba2cca361c50ff8b0b11a917e5e61d01",
        "522102744c609aeee71a07136482b71244a6217b3368431603e1e3994d0c2d226403af2103cfa114ffa28b97884a028322665093af66bb19b0cf91c81eae46e6bb7fff799a52ae"

]
```

### Sequence

The last part of the input is the `sequence` field.

```
"sequence": 2159794202
```

We place the lower 24 bits of the **obscured commitment number** here, prefixed with 0x80 (8 bits).

By prefixing this field with 0x90, we disable any relative locktimes (in relation to the 2-of-2 multisig funding transaction).


### Outputs

Here, we can see the `to_local` and `to_remote` outputs! Can you tell which is which?

It's much easier to identify them by simply looking at the amounts, but see if you can identify which is which by looking at the `"asm"` only. It's possible!

If you noticed the `type` field, that would have given it away as well. Remember, the `to_remote` is a P2WPKH, while the `to_local` is a P2WSH.

However, another way to tell which is which is by the *size* of the "asm", which is meant to be a human-readable script. The `0` stands for `OP_0` and the data after is the hash! If you recall, the `to_remote` takes the HASH160 (results in 20 bytes) of the Public Key, while the `to_remote` takes the SHA256 (results in 32 bytes) of the witness script. So the `to_local` will be longer!

```
"vout": [
  {
    "value": 0.00000500,
    "n": 0,
    "scriptPubKey": {
      "asm": "0 8c4e98f51715d292104530224efba56176fb39b1",
      "desc": "addr(bcrt1q338f3aghzhffyyz9xq3ya7a9v9m0kwd3v7t2jl)#euqsj43f",
      "hex": "00148c4e98f51715d292104530224efba56176fb39b1",
      "address": "bcrt1q338f3aghzhffyyz9xq3ya7a9v9m0kwd3v7t2jl",
      "type": "witness_v0_keyhash"
    }
  },
  {
    "value": 0.04987640,
    "n": 1,
    "scriptPubKey": {
      "asm": "0 69fa2c947fd3a8a103b076b46bc5bea6c1035fee25a14270c6a6e420d234a4ab",
      "desc": "addr(bcrt1qd8aze9rl6w52zqasw66xh3d75mqsxhlwyks5yuxx5mjzp5355j4sxljgh7)#lvf4jlja",
      "hex": "002069fa2c947fd3a8a103b076b46bc5bea6c1035fee25a14270c6a6e420d234a4ab",
      "address": "bcrt1qd8aze9rl6w52zqasw66xh3d75mqsxhlwyks5yuxx5mjzp5355j4sxljgh7",
      "type": "witness_v0_scripthash"
    }
  }
]
```