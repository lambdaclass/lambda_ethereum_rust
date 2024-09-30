# Ethereum Rust L2 Operator

## ToC

- [Components](#components)
    - [L1 Watcher](#l1-watcher)
    - [L1 Transaction Sender](#l1-transaction-sender)
    - [Block Producer](#block-producer)
    - [Proof Data Provider](#proof-data-provider)

## Components

The L2 operator is composed of the following components:

### L1 Watcher

This component handles the L1->L2 messages. Without rest, it is always watching the L1 for new deposit events defined as `DepositInitiated()` that contain the deposit transaction to be executed on the L2. Once a new deposit event is detected, it will insert the deposit transaction into the L2.

In the future, it will also be watching for other L1->L2 messages.

### L1 Transaction Sender

As the name suggests, this component sends transactions to the L1. But not any transaction, only commit and verify transactions.

Commit transactions are sent when the operator wants to commit to a new block. These transactions contain the block data to be committed in the L1.

Verify transactions are sent by the operator after the prover has successfully generated a proof of block execution to verify it. These transactions contain the proof to be verified in the L1.

### Block Producer

This component is responsible for producing new blocks ready to be committed. For the moment, as we do not have consensus in the L2, this component is a mock of the Engine API.

### Proof Data Provider

TODO
