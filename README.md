# Lambda Ethereum Rust Execution Client

[![Telegram Chat][tg-badge]][tg-url]
[![license](https://img.shields.io/github/license/lambdaclass/ethereum_rust)](/LICENSE)

[tg-badge]: https://img.shields.io/endpoint?url=https%3A%2F%2Ftg.sumanjay.workers.dev%2Frust_ethereum%2F&logo=telegram&label=chat&color=neon
[tg-url]: https://t.me/rust_ethereum

## Philosophy

Many long-established clients accumulate bloat over time. This often occurs due to the need to support legacy features for existing users or through attempts to implement overly ambitious software. The result is often complex, difficult-to-maintain, and error-prone systems.

In contrast, our philosophy is rooted in simplicity. We strive to write minimal code, prioritize clarity, and embrace simplicity in design. We believe this approach is the best way to build a client that is both fast and resilient. By adhering to these principles, we will be able to iterate fast and explore next-generation features early, either from the Ethereum roadmap or from innovations from the L2s.

Read more about our engineering philosophy [here](https://blog.lambdaclass.com/lambdas-engineering-philosophy/)

## Design Principles

- Ensure effortless setup and execution across all target environments.
- Be vertically integrated. Have the minimal amount of dependencies.
- Be structured in a way that makes it easy to build on top of it, i.e rollups, vms, etc.
- Have a simple type system. Avoid having generics leaking all over the codebase.
- Have few abstractions. Do not generalize until you absolutely need it. Repeating code two or three times can be fine.
- Prioritize code readability and maintainability over premature optimizations.
- Avoid concurrency split all over the codebase. Concurrency adds complexity. Only use where strictly necessary.

## Quick Start (localnet)

![Demo](https://raw.githubusercontent.com/lambdaclass/ethereum_rust/8e3b69d727225686eec30b2c2b79cecdf7eac2d9/Demo.png)

### Prerequisites
- [Kurtosis](https://docs.kurtosis.com/install/#ii-install-the-cli)
- [Rust](#rust)
- [Docker](https://docs.docker.com/engine/install/)
```shell
make localnet
```

This make target will:
1. Build our node inside a docker image.
2. Fetch our fork [ethereum package](https://github.com/ethpandaops/ethereum-package), a private testnet on which multiple ethereum clients can interact.
3. Start the localnet with kurtosis.

If everything went well, you should be faced with our client's logs (ctrl-c to leave)

To stop everything, simply run:
```shell
make stop-localnet
```

## Dev Setup
### Build

#### Rust
To build the node, you will need the rust toolchain:
1. First, [install asdf](https://asdf-vm.com/guide/getting-started.html):
2. Add the rust plugin:
```shell
asdf plugin-add rust https://github.com/asdf-community/asdf-rust.git
```
3. cd into the project and run:
```shell
asdf install
```

You now should be able to build the client:
```bash
make build
```
### Database
Currently, the database is `libmdbx`, it will be set up
when you start the client. The location of the db's files will depend on your OS:
- Mac: `~/Library/Application Support/ethereum_rust`
- Linux: `~/.config/ethereum_rust`

You can delete the db with:
```bash
cargo run --bin ethereum_rust -- removedb
```

### Test

For testing, we're using three kinds of tests.

##### Ethereum Foundation Tests

These are the official execution spec tests, you can execute them with:

```bash
make test
```

This will download the test cases from the [official execution spec tests repo](https://github.com/ethereum/execution-spec-tests/) and run them with our glue code
under `cmd/ef_tests/tests`.

##### Crate Specific Tests

The second kind are each crate's tests, you can run them like this:

```bash
make test CRATE=<crate>
```
For example:
```bash
make test CRATE="ethereum_rust-blockchain"
```


##### Hive Tests

Finally, we have End-to-End tests with hive.
Hive is a system which simply sends RPC commands to our node,
and expects a certain response. You can read more about it [here](https://github.com/ethereum/hive/blob/master/docs/overview.md).
Hive tests are categorized by "simulations', and test instances can be filtered with a regex:
```bash
make run-hive-debug SIMULATION=<simulation> TEST_PATTERN=<test-regex>
```
This is an example of a Hive simulation called `ethereum/rpc-compat`, which will specificaly
run chain id and transaction by hash rpc tests:
```bash
make run-hive SIMULATION=ethereum/rpc-compat TEST_PATTERN="/eth_chainId|eth_getTransactionByHash"
```
If you want debug output from hive, use the run-hive-debug instead:
```bash
make run-hive-debug SIMULATION=ethereum/rpc-compat TEST_PATTERN="*"
```
This example runs **every** test under rpc, with debug output

### Run

Example run:
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
- `engine_exchangeCapabilities`
- `engine_exchangeTransitionConfiguration` ✅
- `engine_newPayload` ✅
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
- `eth_getFilterChanges`
- `eth_getFilterLogs`
- `eth_getLogs`
- `eth_getStorageAt` (at head block) ✅
- `eth_getTransactionByBlockHashAndIndex` ✅
- `eth_getTransactionByBlockNumberAndIndex` ✅
- `eth_getTransactionByHash` ✅
- `eth_getTransactionCount` ✅
- `eth_newBlockFilter`
- `eth_newFilter`
- `eth_newPendingTransactionFilter`
- `eth_uninstallFilter`

See issues and progress: <https://github.com/lambdaclass/ethereum_rust/milestone/1>

### Milestone 2: History & Reorgs
Implement support for block reorganizations and historical state queries. This milestone involves persisting the state trie to enable efficient access to historical states and implementing a tree structure for the blockchain to manage multiple chain branches.

RPC endpoints
- `engine_forkchoiceUpdated` (without `payloadAttributes`)
- `eth_call` (at any block) ✅
- `eth_createAccessList` (at any block) ✅
- `eth_getBalance` (at any block) ✅
- `eth_getCode` (at any block) ✅
- `eth_getProof` ✅
- `eth_getStorageAt` (at any block) ✅

See issues and progress: https://github.com/lambdaclass/ethereum_rust/milestone/4

### Milestone 3: Block building
Add the ability to build new payloads, so that the consensus client can propose new blocks based on transactions received from the RPC endpoints.

RPC endpoints
- `engine_forkchoiceUpdated` (with `payloadAttributes`)
- `engine_getPayload`
- `eth_sendRawTransaction` ✅

### Milestone 4: P2P Network
Implement DevP2P protocol, including RLPx `p2p` and `eth` features. This will let us get and send blocks and transactions from other nodes. We'll add the transactions we receive to the mempool. We'll also download blocks from other nodes when we get payloads where the parent isn't in our local chain.

RPC endpoints

- `admin_nodeInfo` ✅

See issues and progress: <https://github.com/lambdaclass/ethereum_rust/milestone/2>

### Milestone 5: Syncing
Add snap sync protocol, which lets us get a recent copy of the blockchain state instead of going through all blocks from genesis. Since we don't support older versions of the spec by design, this is a prerequisite to being able to sync the node with public networks, including mainnet.

RPC endpoints
- `eth_syncing`

See issues and progress: https://github.com/lambdaclass/ethereum_rust/milestone/3

# Crates documentation

In the next sections, you can dive further into the code internals.

-   [net](./crates/net/README.md)
