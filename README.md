# Lambda Ethereum Rust Execution Client

[![Telegram Chat][tg-badge]][tg-url]
[![rust](https://github.com/lambdaclass/ethereum_rust/actions/workflows/ci.yaml/badge.svg)](https://github.com/lambdaclass/ethereum_rust/actions/workflows/ci.yaml)
[![license](https://img.shields.io/github/license/lambdaclass/ethereum_rust)](/LICENSE)

[tg-badge]: https://img.shields.io/endpoint?url=https%3A%2F%2Ftg.sumanjay.workers.dev%2Frust_ethereum%2F&logo=telegram&label=chat&color=neon
[tg-url]: https://t.me/rust_ethereum

## Roadmap

### Milestone Zero (Setup)

- Repository boilerplate
- Http Server setup
- Support the following RPC endpoints
    - `eth_chainId`
    - `engine_exchangeCapabilities`

### First Milestone (Blocks)

*Needs: RLP*

- Spin a local-net with https://github.com/kurtosis-tech/ethereum-package
    - Add CLI to the node with minimal configuration
- Receive blocks from consensus node
- Store block headers and bodies in DB
- Store transactions and receipts
- Support the following RPC endpoints
    - `engine_newPayloadV3` (partially)
    - `eth_getBlockByHash`
    - `eth_getBlockByNumber`
    - `eth_blockNumber`
    - `eth_getBlockReceipts`
    - `eth_getBlockTransactionCountByNumber`
    - `eth_getTransactionByBlockHashAndIndex`
    - `eth_getTransactionByBlockNumberAndIndex`

### Second Milestone (EVM)

*Needs: rEVM integration, Patricia Merkle Tree*

- Call the EVM to perform the state transition
- Verify post state hash
- Store the state in DB
- Support the following RPC endpoints:
    - `eth_getBalance`
    - `eth_getCode`
    - `eth_getStorageAt`
    - `eth_getProof`

### Third Milestone (Consensus)

*Needs: Block downloader, Blockchain tree*

- Support for forkchoice update from consensus client
- Downloading missing blocks
- Support the following RPC endpoints:
    - `eth_syncing`
    - `engine_forkchoiceUpdatedV3`
    - `engine_newPayloadV3` (full)

### Fourth Milestone (Transactions)

*Needs: DevP2P*

- Support to download initial unconfirmed transactions via p2p request
- Support receiving and re-propagating transactions in gossip
- Support sending transactions via RPC
- Support the following RPC endpoints:
    - `eth_getTransactionByHash`
    - `eth_sendTransaction`
    - `eth_sendRawTransaction`
    - `eth_call`
    - `eth_createAccessList`
    - `eth_getTransactionReceipt`

### Fifth Milestone
TBD

## Usage

### Build

To build the main executable and its crates, run:
```
just build
```

### Test
To run the tests from a crate, run:
```
just test <crate>
```

Or just run all the tests:
```
just test-all
```
