# Ethereum Rust L2

The main differences between this mode and regular Ethereum Rust are:

- There is no consensus, only one sequencer proposes blocks for the network.
- Block execution is proven using a RISC-V zkVM and its proofs are sent to L1 for verification.
- A set of Solidity contracts to be deployed to the L1 are included as part of network initialization.
- Two new types of transactions are included

## ToC

- [Roadmap](#roadmap)
    - [Milestone 0](#milestone-0)
    - [Milestone 1 (MVP)](#milestone-1-mvp)
    - [Milestone 2 (State diffs + blobs + base token)](#milestone-2-state-diffs--blobs--base-token)
    - [Milestone 3 (Validium + Account Abstraction)](#milestone-3-validium--account-abstraction)
- [Prerequisites](#prerequisites)
    - [Rust](#rust)
    - [Foundry](#foundry)
- [How to run](#how-to-run)
    - [The command you're looking for](#the-command-youre-looking-for)
    - [The other command you will look for in the future](#the-other-command-you-will-look-for-in-the-future)
    - [Other useful commands](#other-useful-commands)
        - [General](#general)
        - [L1](#l1)
        - [L2](#l2)
- [Local L1 Rich Wallets](#local-l1-rich-wallets)

## Roadmap

| Milestone | Description | Status |
| --------- | ----------- | ------ |
| 0 | Users can deposit Eth in the L1 (Ethereum) and receive the corresponding funds on the L2. | 🏗️ |
| 1 | The network supports basic L2 functionality, allowing users to deposit and withdraw funds to join and exit the network, while also interacting with the network as they do normally on the Ethereum network (deploying contracts, sending transactions, etc). | 🏗️ | 
| 2 | The network now commits to state diffs instead of the full state, lowering the commit transactions costs and supports EIP4844. The L2 can be deployed using a custom native token, meaning that an ERC20 can be the common currency, with fees payed in that currency. | ❌ |
| 3 | The L2 can be initialized in Validium Mode, meaning the Data Availability layer is no longer the L1, but rather a DA layer of the user's choice. The L2 supports native account abstraction following EIP 4337, allowing for custom transaction validation logic and paymaster flows. | ❌ |
| 4 | The L2 has added security mechanisms in place, running on Trusted Execution Environments and Multi Prover setup where multiple guarantees (Execution on TEEs, zkVMs/proving systems) are required for settlement on the L1. | ❌ |

### Milestone 0

Users can deposit Eth in the L1 (Ethereum) and receive the corresponding funds on the L2.

#### Status

|        | Name                           | Description                                                                 | Status |
| --------- | ----------------------------- | --------------------------------------------------------------------------- | ------ |
| Contracts | `CommonBridge`                | Deposit method implementation                                                         | ✅     |
|           | `BlockExecutor`               | Commit and verify methods (placeholders for this stage)          | ✅     |
| VM |     | Adapt EVM to handle deposits |   🏗️    |
| Operator  | `Sequencer`                   | Proposes new blocks to be executed                                          | ✅     |
|           | `L1Watcher`                   | Listens for and handles L1 deposits                                         | 🏗️     |
|           | `L1TxSender`                  | commits new block proposals and sends block execution proofs to be verified | 🏗️     |
|           | Deposit transactions handling | new transaction type for minting funds corresponding to deposits            | 🏗️     |
| CLI | `stack` | Support commands for initializing the stack | ✅     |
| CLI | `config` | Support commands for stack config management | ✅     |
| CLI | `wallet deposit` | Support command por depositing funds on L2 | 🏗️     |
| CLI | `wallet transfer` | Support command for transferring funds on L2   | 🏗️     |


### Milestone 1: MVP

The network supports basic L2 functionality, allowing users to deposit and withdraw funds to join and exit the network, while also interacting with the network as they do normally on the Ethereum network (deploying contracts, sending transactions, etc).

#### Status

|        | Name                            | Description                                                                                                           | Status |
| --------- | ------------------------------ | --------------------------------------------------------------------------------------------------------------------- | ------ |
| Contracts | `CommonBridge`                 | Withdraw method implementation                                                                                        | ❌     |
|           | `BlockExecutor`                | Commit and verify implementation                                                                                      | 🏗️     |
|           | `Verifier`                     |  verifier                                                                                                      | 🏗️     |
| Operator  | `ProofDataProvider`            | Feeds the `ProverDataClient` with block data to be proven and delivers proofs to the `L1TxSender` for L1 verification | 🏗️     |
|           | Withdraw transactions handling |    New transaction type for burning funds on L2 and unlock funds on L1                                                                                                                   | 🏗️     |
| Prover    | `ProofDataClient`              |  Asks for block execution data to prove, generates proofs of execution and submits proofs to the `ProofDataProvider`                                                                                                                     | 🏗️     |

### Milestone 2: State diffs + blobs + custom native token

The network now commits to state diffs instead of the full state, lowering the commit transactions costs and supports EIP4844.

The L2 can be deployed using a custom native token, meaning that an ERC20 can be the common currency, with fees payed in that currency.

#### Status

|           | Name          | Description                                            | Status |
| --------- | ------------- | ------------------------------------------------------ | ------ |
| Contracts | BlockExecutor | Differentiate whether to execute in calldata or blobs mode                                                      |  ❌      |
|  | CommonBridge | For base token deposits, msg.value = 0 and valueToMintOnL2 > 0 |  ❌      |
|  |  | For base token withdrawals, we need to infer the base token  |  ❌      |
|  |  | Add track of chain's base token |  ❌      |
| VM        |               | The VM should return which storage slots were modified |   ❌     |
| Operator  |  ProofDataProvider  |  Sends state diffs to the prover   |   ❌     |
|   |  L1TxSender  |  Differentiate whether to send the commit transaction with calldata or blobs   |   ❌     |
| Prover    | RISC-V zkVM   | Adapt state proofs                                                       |    ❌    |
|    |    | Add program for proving blobs                                                       |    ❌    |
| CLI    | `reconstruct-state`   | Add a command for reconstructing the state                                                       |    ❌    |
|     | `deposit`   | Handle base token deposits                                                       |    ❌    |
|     | `withdraw`   | Handle base token withdrawals                                                       |    ❌    |
|     | `init`   | Adapt stack initialization to either send blobs or calldata                                                       |    ❌    |
|Misc  |    | Add a DA integration example for Validium mode                                                       |    ❌    |

### Milestone 3: Validium + Account Abstraction

The L2 can be initialized in Validium Mode, meaning the Data Availability layer is no longer the L1, but rather a DA layer of the user's choice.

The L2 supports native account abstraction following EIP 4337, allowing for custom transaction validation logic and paymaster flows.

#### Status

|           | Name          | Description                                            | Status |
| --------- | ------------- | ------------------------------------------------------ | ------ |
| Contracts | BlockExecutor | Do not check data availability in Validium mode                                                      |  ❌      |
| VM        |               | The VM should return which storage slots were modified |   ❌     |
| Operator  |  L1TxSender  |  Do no send data in commit transactions   |   ❌     |
| CLI    | `init`   | Adapt stack initialization to support Validium stacks                                                       |    ❌    |

TODO: Expand on account abstraction tasks.

### Milestone 4: Security (TEEs and Multi Prover support)

The L2 has added security mechanisms in place, running on Trusted Execution Environments and Multi Prover setup where multiple guarantees (Execution on TEEs, zkVMs/proving systems) are required for settlement on the L1. This better protects against possible security bugs on implementations.

#### Status

|           | Name          | Description                                            | Status |
| --------- | ------------- | ------------------------------------------------------ | ------ |
| VM/Prover        |               | Support proving with multiple different zkVMs |   ❌     |
| Contracts        |               | Support verifying multiple different zkVM executions |   ❌     |
| VM        |               | Support running the operator on a TEE environment |   ❌     |

## Prerequisites

- [Rust (explained in the repo's main README)](../../README.md)
- [Foundry](#foundry)

### Foundry

1. First, install `foundryup`:
    ```shell
    curl -L https://foundry.paradigm.xyz | bash
    ```
2. Then run `foundryup`:
    ```shell
    foundryup
    ```

## How to run

### Install `ethereum_rust_l2` CLI

First of all, you need to install the `ethereum_rust_l2` CLI. You can do that by running the command below:

```
cargo install --path ../../cmd/ethereum_rust_l2
```

> [!IMPORTANT]
> Most of the CLI interaction needs a configuration to be set. You can set a configuration with the `config` command.

### Configure your stack

> [!TIP]
> You can create multiple configurations and switch between them.

```
ethereum_rust_l2 config create <config_name>
```

![](../../cmd/ethereum_rust_l2/assets/config_create.cast.gif)

### Initialize the stack

> [!IMPORTANT]
> Add the SPI_PROVER=mock env variable to the command (to run the prover you need ).

```
ethereum_rust_l2 stack init
```

![](../../cmd/ethereum_rust_l2/assets/stack_init.cast.gif)

### Restarting the stack

> [!WARNING]
> This command will cleanup your running L1 and L2 nodes.

```
ethereum_rust_l2 stack restart
```

![](../../cmd/ethereum_rust_l2/assets/stack_restart.cast.gif)

## Local L1 Rich Wallets

Most of them are [here](https://github.com/ethpandaops/ethereum-package/blob/main/src/prelaunch_data_generator/genesis_constants/genesis_constants.star), but there's an extra one:

```
{
    "address": "0x3d1e15a1a55578f7c920884a9943b3b35d0d885b",
    "private_key": "0x385c546456b6a603a1cfcaa9ec9494ba4832da08dd6bcf4de9a71e4a01b74924"
}
```

## Docs

- [Ethereum Rust L2 Docs](./docs/README.md)
- [Ethereum Rust L2 CLI Docs](../../cmd/ethereum_rust_l2/README.md)

## 📚 References and acknowledgements

The following links, repos, companies and projects have been important in the development of this library and we want to thank and acknowledge them.

- [Matter Labs](https://matter-labs.io/)
- [Optimism](https://www.optimism.io/)
