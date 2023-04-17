# Tappy

**Don't use for real money on the main network! Use on regtest or testnet only!**

A utility to create Taproot key and script spends on the command line in conjunction with Bitcoin Core.

## Commands

- init
    - Create an empty state and save it
- print
    - Print current state
- gen
    - Generate random state
- toggle
    - Activate a passive item or passivize an active item
- fund
    - Get address of transaction input to fund via Bitcoin Core
- spend
    - Create transaction witness and print raw transaction hex to send via Bitcoin Core
- in
    - Add transaction input
- out
    - Add transaction output
- utxo
    - Add UTXO for a transaction input
- locktime
    - Update locktime
- seq
    - Update sequence of a transaction input
- fee
    - Update transaction fee
- move
    - Convert transaction output into transaction input

## Building

```
cargo build
```

## Bitcoin Core Setup

1. Install Bitcoin Core (tested with version 24.0.1)
2. Run bitcoind on regtest or testnet
3. Make sure you have a wallet with funds

## Tappy State

tappy keeps track of its state using the file `state.json` in the current directory. Generate the initial state like so:

```
$ tappy init
```

You can view the current that like so:

```
$ tappy print
```

## Key Store

tappy keeps a set of key pairs that are _active_ or _passive_. Active keys are used during signing and passive ones are not. You can generate fresh keys by calling `tappy gen --keys` followed by the number of keys.

```
$ tappy gen --keys 10
```

Toggle the status of a key pair _(passive to active, and active to passive)_ by using `tappy toggle --key` followed by the xpub.

```
$ tappy toggle --key 1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f
```

## Image Store

There are also active and passive pairs of SHA-256 images and preimages. Generate by calling `tappy gen --images` followed by the number of images.

```
$ tappy gen --images 10
```

Toggle the status by calling `tappy toggle --image` followed by the SHA-256 image.

```
$ tappy toggle --image d166f218267103b44f1102a3ef05e87a9911b9f7cc7f0887f91e198e6a7d3fc4
```

## Transaction Setup

In tappy you create a Bitcoin transaction from inputs, utxos and outputs. This is represented in the tappy state. Inputs and outputs are specified by Taproot descriptors that use keys from the key store or combinations of them _(and, or, thres, multi, ...)_.

## Transaction Inputs

You add a new transaction input by calling `tappy in` followed by the input index and the descriptor.

```
$ tappy in 0 "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f)"
```

## Input UTXOs

Each input needs a UTXO to be spent. You add a UTXO to a transaction input by calling `tappy utxo` followed by the input index, txid, output index and value.

```
$ tappy utxo 0 16f37c90b463670fc647283ce864d1ac765a7a7e541db3bd4ef401a2e2db30fb 0 100000000
```

### Funding Unfunded Inputs

When you start using tappy, none of its keys controls any coins. To start a chain of tappy transactions, you need to fund the initial inputs using a Bitcoin Core wallet. Use `tappy fund` followed by the input index to get the input address.

```
$ tappy fund 0
: <ADDRESS>
```

Then send coins to that address using bitcoin-cli.

```
$ bitcoin-cli sendtoaddress <ADDRESS>
: <TXID>
```

### Funding Funded Inputs

If a transaction input is a previous output that we created in tappy, then you can get UTXO information from its txid. First, it's useful to have the transaction hex.

```
$ bitcoin-cli getrawtransaction <TXID>
: <TX_HEX>
```

You can get a human-readable version by appending `1` to the command.

```
$ bitcoin-cli getrawtransaction <TXID> 1
: <FORMATTED_TX>
```

We recommend the use of [hal](https://github.com/stevenroose/hal) for even better formatting.
Among other things, hal displays values in satoshi, which is the required format for tappy utxos.

```
$ hal tx decode <TX_HEX>
: <FORMATTED_TX>
```

## Transaction Outputs

You add a new transaction output by calling `tappy out` followed by the output index, descriptor and value. Omit the value to signify that all input funds minus the other outputs minus fees should go to this output.

```
$ tappy out 0 "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f)"
```

## Locktime

Set the locktime of the transaction to any block height. Locktimes in unix time are not supported. The locktime is **disabled** unless at least one input has a relative timelock (which may be zero)! Enabling locktime without relative timelock is not supported.

```
$ tappy locktime 785572
```

## Sequence

Enable a relative timelock for a transaction input by calling `tappy seq` followed by the input index, `enable` and the relative block height. The latter is set to zero by default, which enables locktime for the entire transaction but makes the input immediately spendable. Relative timelocks in unix time are not supported.

```
$ tappy seq 0 enable 1337
```

Disable the relative timelock of a transaction input by calling `tappy seq` followed by the input index and `disable`.

```
$ tappy seq 0 disable
```

## Fee

Set the fee to whatever seems reasonable. _It should not be zero._ During spending the feerate will be displayed, so if Bitcoin Core rejects your transaction you can bump the fee.

```
$ tappy fee 1000
```

## Spending

With everything set, attempt to create a spending transaction by calling `tappy spend`. If any keys were missing then you will get an error message. Otherwise, tappy will return a transaction hex.

```
$ tappy spend
: <TX_HEX>
```

Use bitcoin-cli to broadcast this transaction.

```
$ bitcoin-cli sendrawtransaction <TX_HEX>
: <TXID>
```

## After Spending

Outputs of the previous transaction become inputs of the next transaction, forming chains. Use `tappy move` followed by the output and input index to transform an output into an input. You need to add UTXO information based on the txid after that.

```
$ tappy move 0 0
```

## Key Spend

```
$ tappy in 0 "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f)"
$ tappy utxo 0 16f37c90b463670fc647283ce864d1ac765a7a7e541db3bd4ef401a2e2db30fb 0 100000000
$ tappy out 0 "tr(9fb5213dd37f61c98629500a436ae8f390b03d37d3609af2f01d515d4e899800)" 50000000
$ tappy out 1 "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f)" 
$ tappy spend
```

## Multisig

```
$ tappy in 0 "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f)"
$ tappy utxo 0 16f37c90b463670fc647283ce864d1ac765a7a7e541db3bd4ef401a2e2db30fb 0 100000000
$ tappy out 0 "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f,multi_a(2,816945ddf16d3a568644d5fe174dca7d68ed2931d3ee4edefbd96d09ae30ec2e,75910f6c72d67cd2530d17ecba4bce9058d003564887925d46201e18513d804a,9fb5213dd37f61c98629500a436ae8f390b03d37d3609af2f01d515d4e899800))"
$ tappy spend
```

## Miniscript

```
$ tappy in 0 "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f)"
$ tappy utxo 0 16f37c90b463670fc647283ce864d1ac765a7a7e541db3bd4ef401a2e2db30fb 0 100000000
$ tappy out 0 "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f,and_v(or_c(pk(816945ddf16d3a568644d5fe174dca7d68ed2931d3ee4edefbd96d09ae30ec2e),v:pk(75910f6c72d67cd2530d17ecba4bce9058d003564887925d46201e18513d804a)),pk(9fb5213dd37f61c98629500a436ae8f390b03d37d3609af2f01d515d4e899800)))"
$ tappy spend
```

## Multiple Inputs, Multiple Outputs

```
$ tappy in 0 "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f)"
$ tappy in 1 "tr(9fb5213dd37f61c98629500a436ae8f390b03d37d3609af2f01d515d4e899800)"
$ tappy utxo 0 16f37c90b463670fc647283ce864d1ac765a7a7e541db3bd4ef401a2e2db30fb 0 100000000
$ tappy utxo 1 16f37c90b463670fc647283ce864d1ac765a7a7e541db3bd4ef401a2e2db30fb 1 100000000
$ tappy out 0 "tr(1ffa25da651d709df36d7563fffb5416a54ff2a9702ac66d8fde4c9d029d4c2f)" 150000000
$ tappy out 1 "tr(9fb5213dd37f61c98629500a436ae8f390b03d37d3609af2f01d515d4e899800)"
$ tappy spend
```

## Known Limitations

It's awkward to manage multiple chains of transactions whose outputs diverge. Currently the best solution is to keep multiple copies of `state.json`, one for each chain.
