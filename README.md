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

## Usage

### Build

To build the main executable and its crates, run:

```bash
make build
```

### Test

Note: To execute EF tests, the test fixtures are required. To download them, run:

```bash
make download-test-vectors
```

To run the tests from a crate, run:

```bash
make test CRATE=<crate>
```

Or just run all the tests:

```bash
make test
```

### Run

To run a localnet, we can use a fork of [Ethereum Package](https://github.com/ethpandaops/ethereum-package), specifically [this branch](https://github.com/lambdaclass/ethereum-package/tree/ethereum-rust-integration) that adds support to our client. We have that included in our repo as a `just` target. Make sure to fetch it like follows:

```bash
make checkout-ethereum-package
```

Let's now install kurtosis:

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

To stop the localnet:

```bash
make stop-localnet
```

You can also run the node using the standalone CLI:

```bash
cargo run --bin ethereum_rust -- --network test_data/genesis-kurtosis.json
```

The `network` argument is mandatory, as it defines the parameters of the chain.

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

See issues and progress: https://github.com/lambdaclass/ethereum_rust/milestone/1

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

See issues and progress: https://github.com/lambdaclass/ethereum_rust/milestone/2

### Milestone 5: Syncing

Add snap sync protocol, which lets us get a recent copy of the blockchain state instead of going through all blocks from genesis. Since we don't support older versions of the spec by design, this is a prerequisite to being able to sync the node with public networks, including mainnet.

RPC endpoints

- `eth_syncing`

See issues and progress: https://github.com/lambdaclass/ethereum_rust/milestone/3

## Crates

- [net]: handles the ethereum networking protocols
