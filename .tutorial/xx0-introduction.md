# Welcome to Programming Lightning: Intro to Payment Channels & The Bitcoin Lightning Network

Welcome! If you're reading this, chances are you're interested in learning how Bitcoin's Lightning Network works. Well, you've come to the right place. Programming Lightning is a comprehensive, in-depth course that teaches Lightning from scratch. Over the length of this course, you'll learn *exactly* how Lightning channels work by implementing a Lightning channel yourself.

In fact, by the end of this course, your Lightning implementation will even pass the [BOLT 3 Test Vectors](https://github.com/lightning/bolts/blob/master/03-transactions.md#appendix-b-funding-transaction-test-vectors). If you're unfamiliar with **"Test Vectors"**, they are a set of **pre-defined inputs** and **expected outputs** that help developers ensure their code aligns with the protocol. For example, in the context of Lightning, if your implementation passes the Test Vectors, then that means, given pre-specified inputs (**key seeds**, **transaction output amounts**, etc.), your implementation will produce the exact same output that any popular Lightning implementation (LND, LDK, Eclair, Core Lightning) will! As you can probably infer, Test Vectors are a critical piece of decentralized, open source Lightning development, as they enable Lightning implementations to ensure that they're working correctly and are interoperable with each other.

The goal of Programming Lightning is to give you hands-on development experience with Lightning. This portion of the workshop, **Intro to Payment Channels & The Bitcoin Lightning Network**, will provide an extensive overview of payment channels. We'll start by building our off-chain Lightning wallet, which will involve managing many different keys. To, hopefully, make this introduction focused and intuitive, we won't discuss the exact purpose of each key just yet, as we don't yet know how Lightning works!

Once we build our wallet and have a solid foundation, we'll begin implementing our Lightning channel. We'll start by reviewing the simplest "off-chain channel" construction possible, and we'll discuss each of the downsides that this simple construction has. Then, we'll tackle each one and, step by step, work our way towards a full BOLT-compliant Lightning channel in all its glory.

# Prerequisites

This course assumes that you have a working understanding of the information contained in **Mastering Bitcoin** and/or **Programming Bitcoin**. To get the most out of this course, you should already have a strong grasp on bitcoin transactions and bitcoin script. If you’d like to brush up beforehand, here are a few excellent resources:
- Script (Free): https://learnmeabitcoin.com/technical/script/
- Transactions (Free): https://learnmeabitcoin.com/technical/transaction/
- Transactions + Script ($119): https://www.udemy.com/course/base58-bitcoin-transactions-one/

# Replit

Programming Lightning is optimized to be completed on replit.com, thought you can fork the repo and run it locally, if you'd like. The Replit will have a starter project ready to go and a tutorial to guide you through the programming exercises. If you're 's recommended to use a **2-pane setup**, with the tutorial on one side and the following tabs on the other side:
- `src\exercises\exercises.rs`
- `src\exercises\transactions.txt`
- Replit Shell
- Replit Console

**This is what that would look like**:
<p align="center" style="width: 50%; max-width: 300px;">
  <img src="./tutorial_images/setup.png" alt="setup" width="100%" height="auto">
</p>

# Rust
All programming exercises in this course utilize the Rust programming language and make heavy use of the [**Rust Bitcoin**](https://github.com/rust-bitcoin/rust-bitcoin) library. I know what you're thinking --- why not Python? Well, there are a few reasons. One, Rust 

# Course Exercises

As you work through the course, you will come across emojis that signal an exercise is coming up.  Here's a quick overview of what you will see and what each emoji means:

👉 This emoji means the following console command or code block should be copy-and-pasted or typed out:
```
// some command or code will be here, with a copy button on the right,
```
⚡️ You'll see a lightning bolt when it's time to start a programming exercise.

### Coding Exercises
All coding exercises in this course will have the following two types of dropdowns, which you can use to help you complete each exercise.
1. **💡 Hint 💡**: Provides useful tips and direction to help you complete the exercises, but it does not provide the answer itself.
2. **Step-By-Step Instructions**: Provides detailed directions for how to complete the exercise.

# A Special Thanks
Throughout this course, you'll see many transaction diagrams designed to help you understand what's going on "under the hood" while providing enough abstraction to see the bigger picture. These diagrams are based on ones created by **Elle Mouton** in her article, [Opening and announcing a pre-taproot LN channel](https://ellemouton.com/posts/open_channel_pre_taproot/). Elle's diagrams are, by far, the clearest and most concise transaction visuals I've seen. I encourage you to visit her blog!