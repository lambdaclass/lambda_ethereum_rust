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
just build
```

### Test
Note: To execute EF tests, the test fixtures are required. To download them, run:
```bash
just download-vectors
```

To run the tests from a crate, run:
```bash
just test <crate>
```

Or just run all the tests:
```bash
just test
```

### Run

To run a localnet, we can use a fork of [Ethereum Package](https://github.com/ethpandaops/ethereum-package), specifically [this branch](https://github.com/lambdaclass/ethereum-package/tree/ethereum-rust-integration) that adds support to our client. We have that included in our repo as a git submodule. Make sure to fetch it like follows:

```bash
git submodule update --init
```

Let's now install kurtosis:

```bash
# Make sure to have docker installed

# Kurtosis cli
brew install kurtosis-tech/tap/kurtosis-cli
```

To run the localnet:

```bash
# Make sure we build our docker image with latest changes
docker build -t ethereum_rust .

# Ethereum package is included in the repo as a submodule.
kurtosis run --enclave lambdanet ethereum-package --args-file network_params.yaml
```

To stop the localnet:

```bash
kurtosis enclave stop lambdanet ; kurtosis enclave rm lambdanet
```

## Roadmap

### Milestone 1: RPC Node
Add support to follow a post-Merge localnet as a read-only RPC Node.

RPC endpoints
- `engine_newPayloadV3` (excl. block building)
- `eth_blobBaseFee`
- `eth_blockNumber`
- `eth_call`
- `eth_chainId`
- `eth_createAccessList`
- `eth_estimateGas`
- `eth_feeHistory`
- `eth_getBalance` (at head block)
- `eth_getBlockByHash`
- `eth_getBlockByNumber`
- `eth_getBlockReceipts`
- `eth_getBlockTransactionCountByNumber`
- `eth_getCode` (at head block)
- `eth_getStorageAt` (at head block)
- `eth_getTransactionByBlockHashAndIndex`
- `eth_getTransactionByBlockNumberAndIndex`
- `eth_getTransactionByHash`

See issues and progress: https://github.com/lambdaclass/ethereum_rust/milestone/1

### Milestone 2: P2P Network
Implement DevP2P protocol, including RLPx `p2p` and `eth` capabilities.

RPC endpoints
- `admin_nodeInfo`

See issues and progress: https://github.com/lambdaclass/ethereum_rust/milestone/2

### Milestone 3: Reorgs
Add support for block reorgs. Persist the Trie.

RPC endpoints
- `engine_exchangeCapabilities`
- `engine_forkchoiceUpdatedV3`
- `eth_getProof`
- `eth_getBalance` (at any block)
- `eth_getStorageAt` (at any block)
- `eth_getCode` (at head block)

See issues and progress: https://github.com/lambdaclass/ethereum_rust/milestone/4

### Milestone 4: Syncing
Support snap sync on public testnets and mainnet.

RPC endpoints
- `engine_forkchoiceUpdatedV3`
- `engine_newPayloadV3`
- `eth_syncing`

See issues and progress: https://github.com/lambdaclass/ethereum_rust/milestone/3

### Milestone 5: Block building
Keep a transaction pool and add ability to propose blocks

RPC endpoints
- `engine_getPayloadV3`
- `engine_newPayloadV3` (with block building)
