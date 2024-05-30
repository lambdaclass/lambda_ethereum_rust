# ethereum_rust
Ethereum Execution client

# Milestones

## Milestone Zero (Setup)

- Repository boilerplate
- Http Server setup
- Support the following RPC endpoints
    - `eth_chainId`
    - `engine_exchangeCapabilities`

## First Milestone (Blocks)

*Needs: RLP, DevP2P*

- Spin a local-net with https://github.com/kurtosis-tech/ethereum-package
    - Add CLI to the node with minimal configuration
- Receive block headers and bodies through gossip
- Store block headers and bodies in DB
- Store transactions and receipts
- Support the following RPC endpoints
    - `eth_chainId`
    - `eth_getBlockByHash`
    - `eth_getBlockByNumber`
    - `eth_blockNumber`
    - `eth_getBlockReceipts`
    - `eth_getBlockTransactionCountByNumber`
    - `eth_getTransactionByBlockHashAndIndex`
    - `eth_getTransactionByBlockNumberAndIndex`

## Second Milestone (EVM)

*Needs: rEVM integration*, *Patricia Merkle Tree*

- Call the EVM to perform the state transition
- Verify post state hash
- Store the state in DB
- Support the following RPC endpoints:
    - `eth_getBalance`
    - `eth_getCode`
    - `eth_getStorageAt`
    - `eth_getProof`

## Third Milestone (Consensus)

Needs: *Block downloader, Blockchain tree*

- Support for forkchoice update from consensus client
- Downloading missing blocks
- Support the following RPC endpoints:
    - `engine_exchangeCapabilities`
    - `eth_syncing`
    - `engine_forkchoiceUpdatedV3`
    - `engine_newPayloadV3`

## Fourth Milestone (Transactions)

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
