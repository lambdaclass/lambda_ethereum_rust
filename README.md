# Lambda Ethereum Rust Execution Client

[![Telegram Chat][tg-badge]][tg-url]
[![license](https://img.shields.io/github/license/lambdaclass/ethereum_rust)](/LICENSE)

[tg-badge]: https://img.shields.io/endpoint?url=https%3A%2F%2Ftg.sumanjay.workers.dev%2Frust_ethereum%2F&logo=telegram&label=chat&color=neon
[tg-url]: https://t.me/rust_ethereum

## Philosophy

Many long-established clients accumulate bloat over time. This often occurs due to the need to support legacy features for existing users or through attempts to implement overly ambitious software. The result is often complex, difficult-to-maintain, and error-prone systems.

In contrast, our philosophy is rooted in simplicity:

1. Write minimal code
2. Prioritize clarity
3. Embrace simplicity in design

We believe this approach is the best way to build a client that is both fast and resilient. By adhering to these principles, we will be able to iterate fast and explore next-generation features early, either from the Ethereum roadmap or from innovations from the L2s.

## Quick Start (localnet)

![Demo](https://raw.githubusercontent.com/lambdaclass/ethereum_rust/update-readme/Demo.png)

You'll need docker, docker compose and rust 1.80

### Mac

```shell
brew install kurtosis-tech/tap/kurtosis-cli && make
```

### Linux

[Install kurtosis](https://docs.kurtosis.com/install/#ii-install-the-cli) and run:

```shell
make
```

This will:

- Start [ethereum package](https://github.com/ethpandaops/ethereum-package), a private testnet on which multiple ethereum clients can interact.
- Start our execution client, and make it interact with the others.

## Dev Setup

### Build

To build the node, you will need the rust toolchain, which you can install from rustup:

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

You can now build the project:

```bash
make build
```

### Testing

For testing, we're using the following.

##### Ethereum Foundation Tests

These are the official execution spec tests, tests that should be implementation agnostic, you can run them like this:

```bash
make test
```

This will previously download it from the [official execution spec tests repo][https://github.com/ethereum/execution-spec-tests/] and run them with our glue code
under [cancun.rs][./cmd/ef_tests/tests/cancun.rs].

##### Crate Specific Tests

The second kind are each crate tests, you can run it like this:

```bash
make test CRATE=<crate>
```

##### Hive Tests

Finally, we have End-to-End tests with hive.
Hive is a system which simply sends RPC commands to our node,
and expects a certain response. You can read more about it [here][https://github.com/ethereum/hive/blob/master/docs/overview.md].
And you can run this set of tests with:

```bash
make run-hive-debug ethereum/rpc-compat
```

### Run locally

To run a localnet, we'll use our fork of [ethPandaOps' Ethereum Package](https://github.com/ethpandaops/ethereum-package), specifically [this branch](https://github.com/lambdaclass/ethereum-package/tree/ethereum-rust-integration) that adds support to our client. We have that included in our repo as a `make` target. Make sure to fetch it like follows:

```bash
make checkout-ethereum-package
```

If you haven't yet, install Kurtosis:

```bash
# Make sure to have docker installed

# Kurtosis cli
brew install kurtosis-tech/tap/kurtosis-cli
```

To run the localnet:

```bash
# Ethereum package is included in the repo as a make target.
make localnet
```

Here, we setup kurtosis and make it run with our node inside a docker file.
The config in charge of doing this is the file in under `/test_data/network_params.yaml`
To stop the localnet:

```bash
make stop-localnet
```

You can also run the node using the standalone CLI:

```bash
cargo run --bin ethereum_rust -- --network test_data/genesis-kurtosis.json
```

The `network` argument is mandatory, as it defines the parameters of the chain.
For more information about the different cli arguments check out the next section.

### CLI Commands

Ethereum Rust supports the following command line arguments:
- `--network <FILE>`: Receives a `Genesis` struct in json format. This is the only argument which is required. You can look at some example genesis files at `test_data/genesis*`.
- `--datadir <DIRECTORY>`: Receives the name of the directory where the Database is located.
- `--import <FILE>`: Receives an rlp encoded `Chain` object (aka a list of `Block`s). You can look at the example chain file at `test_data/chain.rlp`.
- `--http.addr <ADDRESS>`: Listening address for the http rpc server. Default value: localhost.
- `--http.port <PORT>`: Listening port for the http rpc server. Default value: 8545.
- `--authrpc.addr <ADDRESS>`: Listening address for the authenticated rpc server. Default value: localhost.
- `--authrpc.port <PORT>`: Listening port for the authenticated rpc server. Default value: 8551.
- `--authrpc.jwtsecret <FILE>`: Receives the jwt secret used for authenticated rpc requests. Default value: jwt.hex.
- `--p2p.addr <ADDRESS>`: Default value: 0.0.0.0.
- `--p2p.port <PORT>`: Default value: 30303.
- `--discovery.addr <ADDRESS>`: UDP address for P2P discovery. Default value: 0.0.0.0.
- `--discovery.port <PORT>`: UDP port for P2P discovery. Default value: 30303.
- `--bootnodes <BOOTNODE_LIST>`: Comma separated enode URLs for P2P discovery bootstrap.

## Roadmap

### Milestone 1: RPC Node

Add support to follow a post-Merge localnet as a read-only RPC Node. This first milestone will only support a canonical chain (every incoming block has to be the child of the current head).

RPC endpoints

- `debug_getRawBlock` ✅
- `debug_getRawHeader` ✅
- `debug_getRawReceipts` ✅
- `debug_getRawTransaction` ✅
- `engine_newPayloadV3` (excl. block building) ✅
- `eth_blobBaseFee` ✅
- `eth_blockNumber` ✅
- `eth_call` (at head block) ✅
- `eth_chainId` ✅
- `eth_createAccessList` (at head block) ✅
- `eth_estimateGas` ✅
- `eth_feeHistory`
- `eth_getBalance` (at head block) ✅
- `eth_getBlockByHash` ✅
- `eth_getBlockByNumber` ✅
- `eth_getBlockReceipts` ✅
- `eth_getBlockTransactionCountByNumber` ✅
- `eth_getCode` (at head block) ✅
- `eth_getStorageAt` (at head block) ✅
- `eth_getTransactionByBlockHashAndIndex` ✅
- `eth_getTransactionByBlockNumberAndIndex` ✅
- `eth_getTransactionByHash` ✅
- `eth_getTransactionCount` ✅

See issues and progress: <https://github.com/lambdaclass/ethereum_rust/milestone/1>

### Milestone 2: P2P Network

Implement DevP2P protocol, including RLPx `p2p` and `eth` capabilities. This will allow us to receive and send blocks and transactions from other nodes and is a prerequisite for the next milestones.

RPC endpoints

- `admin_nodeInfo` ✅

See issues and progress: <https://github.com/lambdaclass/ethereum_rust/milestone/2>

### Milestone 3: History & Reorgs

Implement support for block reorganizations and historical state queries. This milestone involves persisting the state trie to enable efficient access to historical states and implementing a tree structure for the blockchain to manage multiple chain branches.

RPC endpoints

- `engine_exchangeCapabilities`
- `engine_forkchoiceUpdatedV3`
- `eth_call` (at any block)
- `eth_createAccessList` (at any block)
- `eth_getBalance` (at any block)
- `eth_getCode` (at any block)
- `eth_getProof`
- `eth_getStorageAt` (at any block)

See issues and progress: <https://github.com/lambdaclass/ethereum_rust/milestone/4>

### Milestone 4: Syncing

Add snap sync protocol, which lets us get a recent copy of the blockchain state instead of going through all blocks from genesis. Since we don't support older versions of the spec by design, this is a prerequisite to being able to sync the node with public networks, including mainnet.

RPC endpoints

- `engine_forkchoiceUpdatedV3`
- `engine_newPayloadV3`
- `eth_syncing`

See issues and progress: <https://github.com/lambdaclass/ethereum_rust/milestone/3>

### Milestone 5: Block building

Keep transactions received from other nodes in memory, and add the ability to build new payloads, so that the consensus client can propose new blocks.

RPC endpoints

- `engine_getPayloadV3`
- `engine_newPayloadV3` (with block building)
- `eth_sendTransaction`
- `eth_sendRawTransaction`
