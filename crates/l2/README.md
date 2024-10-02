# Rust Ethereum L2

## ToC

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

## Prerequisites

- [Rust (explained in the repo's main README)](../../README.md)
- [Foundry](#foundry)

#### Foundry

1. First, install `foundryup`:
    ```shell
    curl -L https://foundry.paradigm.xyz | bash
    ```
2. Then run `foundryup`:
    ```shell
    foundryup
    ```

## How to run

### The command you're looking for

Running the below command will start both a local L1 (reth for the moment, but `ethereum_rust` in the future) in the port `8545` and a local L2 (`ethereum_rust`) in the port `1729`.

```
make init
```

This command has five steps that can also be run individually:

- `make init-l1` - Starts the L1 (reth) node, creating the volumes necessary for running a docker compose file with reth's docker image.
- `make contract-deps` - Installs the libs used by the contracts in the foundry project.
- `make setup-prover` - Build the ELF for the SP1 prover program.
- `make deploy-l1` - Deploys the L1 contracts to the L1 node. This runs the [`DeployL1` script](./contracts/script/DeployL1.s.sol).
- `make init-l2` - Starts the L2 (`ethereum_rust`) node.

### The command you will be looking for

> [!WARNING]
> This command will cleanup your running L1 and L2 nodes.

Use this command to restart the whole setup with a clean state.

```
make restart
```

### Other useful commands

#### General

- `make down` - Stops the L1 and L2 nodes.
- `make clean` - Cleans the L1 state.

#### L1

- `make init-l1` - Starts the L1 node.
- `make deploy-l1` - Deploys the L1 contracts.
- `make down-l1` - Stops the L1 node.
- `make clean-l1` - Cleans the L1 state.
- `make restart-l1` - Restarts the L1 node.

#### L2

- `make init-l2` - Starts the L2 node.
- `make down-l2` - Stops the L2 node.
- `make restart-l2` - Restarts the L2 node.

#### Contracts

- `make contract-deps` - Installs the libs used by the contracts in the foundry project.
- `make clean-contract-deps` - Cleans the contract dependencies.
- `make restart-contract-deps` - Restarts the contract dependencies (cleans and then installs).

#### Prover

- `make setup-prover` - Build the ELF for the SP1 prover program.

## Local L1 Rich Wallets

Most of them are [here](https://github.com/ethpandaops/ethereum-package/blob/main/src/prelaunch_data_generator/genesis_constants/genesis_constants.star), but there's an extra one:

```
{
    "address": "0x3d1e15a1a55578f7c920884a9943b3b35d0d885b",
    "private_key": "0x385c546456b6a603a1cfcaa9ec9494ba4832da08dd6bcf4de9a71e4a01b74924"
}
```

## Docs

[Ethereum Rust L2 Docs](./docs/README.md)
