# Tappy

**Don't use for real money on the main network! Use on regtest or testnet only!**

Developer-friendly Taproot-only descriptor wallet on the command line that works in conjunction with Bitcoin Core.

Create custom key or script spends. Debug and test applications with handmade transactions. Learn about Taproot, descriptors, Miniscript, timelocks, and other transaction internals.

## Commands

- init
    - Create empty state
- print
    - Print current state
- key
    - Key pair
- img
    - Preimage-image pair
- addr
    - Temporary inbound address for creating UTXOs
- utxo
    - UTXO (unspent transaction output)
- in
    - Transaction input
- out
    - Transaction output
- locktime
    - Update locktime
- fee
    - Update transaction fee
- spend
    - Create transaction witness and print raw transaction hex to send via Bitcoin Core
- final
    - Finalize transaction and save transaction outputs as UTXOs

## Building

```
cargo build
```

## Bitcoin Core Setup

1. Install Bitcoin Core (tested with version 24.0.1)
2. Run bitcoind on regtest or testnet
3. Make sure you have a wallet with funds

## State

tappy keeps track of its state using the file `state.json` in the current directory. **Secrets are stored on the hard drive!** Generate the initial state like so:

```
$ tappy init
```

You can view the current state like so:

```
$ tappy print
```

## Key Store

tappy keeps a set of Schnorr key pairs. Generate fresh keys by calling `tappy key gen` followed by the number of keys.

```
$ tappy key gen 5
```

By default, keys are disabled for spending. Enable a key pair by calling `tappy key en` followed by the xpub.

```
$ tappy key en 1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f
```

Disable by `tappy key dis` plus the xpub.

```
$ tappy key dis 1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f
```

## Image Store

tappy also keeps a set of SHA-256 preimage-image pairs. Generate a pair by calling `tappy img gen` followed by the number of pairs.

```
$ tappy img gen 5
```

Enable preimage-image pairs for spending by calling `tappy img en` followed by the image.

```
$ tappy img en d166f218267103b44f1102a3ef05e87a9911b9f7cc7f0887f91e198e6a7d3fc4
```

Disable by `tappy img dis` plus the image.

```
$ tappy img dis d166f218267103b44f1102a3ef05e87a9911b9f7cc7f0887f91e198e6a7d3fc4
```

## Creating Transactions

In tappy you create a Bitcoin transaction from utxos, inputs and outputs. This is represented in the current state. Inputs and outputs are specified by Taproot descriptors that use keys/image from the key/image store or combinations of them _(and, or, thres, multi, ...)_.

## UTXO Set

tappy maintains a set of UTXOs (unspend transaction outputs) that it can spend.
Transaction inputs reference some unique UTXO that they want to spend.

### Manual UTXO

tappy starts off without any UTXOs, so we must manually add them. Coins must enter the system. We set a _temporary inbound address_ to create a UTXO with a locking script of our choosing. Use `tappy addr set` followed by a [descriptor](https://github.com/bitcoin/bitcoin/blob/master/doc/descriptors.md). Use the [Policy to Miniscript compiler](https://bitcoin.sipa.be/miniscript/) to generate valid Miniscript.

```
$ tappy addr set "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f)"
: Fund this address: bcrt1pwkjuv2laefk6wqnhmqqurxnuhsc8jmmyn4xa48l4v26z3q4z6gjs5wymts
```

Fund this address using bitcoin-cli. In this example we send 1 BTC = 100000000 sat. The RPC returns the transaction id.

```
$ bitcoin-cli sendtoaddress bcrt1pwkjuv2laefk6wqnhmqqurxnuhsc8jmmyn4xa48l4v26z3q4z6gjs5wymts 1
: 3e59661081cbdbfa69e68a9e679a88f3d9070e209aeb11ff424ea06c806a1e7a
```

Get the full transaction hex using bitcoin-cli.

```
$ bitcoin-cli getrawtransaction 3e59661081cbdbfa69e68a9e679a88f3d9070e209aeb11ff424ea06c806a1e7a
: <TX_HEX>
```

Append `1` for human-readable format.

```
$ bitcoin-cli getrawtransaction 3e59661081cbdbfa69e68a9e679a88f3d9070e209aeb11ff424ea06c806a1e7a 1
```

We recommend the use of [hal](https://github.com/stevenroose/hal) for even better formatting.
Among other things, hal displays values in satoshi, which is the required format for tappy utxos.

```
$ hal tx decode <TX_HEX>
```

Given this information, convert the inbound address into a UTXO by calling `tappy addr utxo` followed by the transaction id, the output index (vout) and the value in satoshi.

```
$ tappy addr utxo 3e59661081cbdbfa69e68a9e679a88f3d9070e209aeb11ff424ea06c806a1e7a 0 100000000
```

### Automatic UTXO

Phew, manually typing all of this stuff was a lot of work. Fortunately, tappy can add UTXOs that result from your transactions almost automatically. See [Finalizing](https://github.com/uncomputable/tappy#finalizing) below for more.

## Transaction Input

Add a new transaction input by calling `tappy in` followed by the input index, `new` and the utxo index.

```
$ tappy in 0 new 0
```

Call `tappy utxo list` to list all UTXOs with their index.

```
$ tappy utxo list
```

## Transaction Output

Add a new transaction output by calling `tappy out` followed by the output index, [descriptor](https://github.com/bitcoin/bitcoin/blob/master/doc/descriptors.md) and value in satoshi.

```
$ tappy out 0 "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f)" 99999000
```

Omit the value to signify that all input funds minus the other outputs minus fees should go to this output. This works for at most one output.

```
$ tappy out 0 "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f)"
```

## Fee

Set the fee to whatever seems reasonable. _It should not be zero._ During spending the feerate will be displayed, so if Bitcoin Core rejects your transaction you can bump the fee. Call `tappy fee` followed by the value in satoshi.

```
$ tappy fee 1000
```

## Locktime

Transaction inputs with absolute timelocks (`after(n)`) enforce the transaction locktime to be at least `n`. A transaction is valid if the height of its containing block is strictly greater than its locktime.

Set the locktime by calling `tappy locktime` followed by the block height. Locktime in unix time is not supported.

```
$ tappy locktime 785572
```

The locktime is **disabled** if all inputs have the default sequence. Timelock opcodes will fail and locktime will be ignored. Change the sequence number of any input to a relative locktime (which may be zero) to enable locktime. Other ways to enable locktime are not supported.

```
$ tappy in 0 seq enable 0
```

## Sequence

While locktime applies to the whole transaction, sequence applies to a single input. Transaction inputs with relative timelocks (`older(n)`) enforce the sequence of that input to be a relative locktime of at least `n`. A transaction is valid if the height of its containing block is strictly greater than the height of the utxo block plus `n`.

Set a relative locktime for an input by calling `tappy in` followed by the input index, `seq enable` and the block height. Relative locktime in unix time is not supported.

```
$ tappy in 0 seq enable 10
```

Disable the relative locktime of in input by calling `tappy in` followed by the input index and `seq disable`.

```
$ tappy in 0 seq disable
```

## Spending

With everything set, attempt to create a spending transaction by calling `tappy spend`. Remember to enable the required keys and images, and pay attention to the inputs' timelocks. tappy will return a transaction hex.

```
$ tappy spend
: <TX_HEX>
```

Use bitcoin-cli to broadcast this transaction. You will receive a transaction id if it worked.

```
$ bitcoin-cli sendrawtransaction <TX_HEX>
: <TXID>
```

## Finalizing

Make sure to save the UTXOs that you just created by broadcasting the spending transaction. Call `tappy finalize` followed by the transaction id.

```
$ tappy final 3e59661081cbdbfa69e68a9e679a88f3d9070e209aeb11ff424ea06c806a1e7a
```

All transaction outputs are automatically converted and added to the UTXO set. The current transaction is cleared and a new transaction is created for the next spend. By default, the first output of the old transaction becomes the first input of the new transaction.

## Key Spend

```
$ tappy in 0 new 0
$ tappy out 0 new "tr(9fb5213dd37f61c98629500a436ae8f390b03d37d3609af2f01d515d4e899800)"
$ tappy fee 1000
$ tappy spend
$ tappy final f4a5f64b1552803ee93db9f35d2faa67be82bd1d508c1e16045112aa6f77d468
```

## Multisig

```
$ tappy in 0 new 0
$ tappy out 0 new "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f,multi_a(2,816945ddf16d3a568644d5fe174dca7d68ed2931d3ee4edefbd96d09ae30ec2e,75910f6c72d67cd2530d17ecba4bce9058d003564887925d46201e18513d804a,9fb5213dd37f61c98629500a436ae8f390b03d37d3609af2f01d515d4e899800))"
$ tappy fee 1000
$ tappy spend
$ tappy final f4a5f64b1552803ee93db9f35d2faa67be82bd1d508c1e16045112aa6f77d468
```

## Absolute Timelock

```
$ tappy in 0 new 0
$ tappy out 0 new "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f,and_v(v:pk(+++816945ddf16d3a568644d5fe174dca7d68ed2931d3ee4edefbd96d09ae30ec2e),after(10)))"
$ tappy fee 1000
$ tappy spend
$ tappy final f4a5f64b1552803ee93db9f35d2faa67be82bd1d508c1e16045112aa6f77d468
```

## Relative Timelock

```
$ tappy in 0 new 0
$ tappy out 0 new "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f,and_v(v:pk(+++816945ddf16d3a568644d5fe174dca7d68ed2931d3ee4edefbd96d09ae30ec2e),older(10)))"
$ tappy fee 1000
$ tappy spend
$ tappy final f4a5f64b1552803ee93db9f35d2faa67be82bd1d508c1e16045112aa6f77d468
```

## Multiple Inputs, Multiple Outputs

```
$ tappy in 0 new 0
$ tappy in 1 new 1
$ tappy out 0 new "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f)" 50000000
$ tappy out 1 new "tr(9fb5213dd37f61c98629500a436ae8f390b03d37d3609af2f01d515d4e899800)"
$ tappy fee 1000
$ tappy spend
$ tappy final f4a5f64b1552803ee93db9f35d2faa67be82bd1d508c1e16045112aa6f77d468
```
