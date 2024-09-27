# Rust Ethereum L2

## ToC

- [How to run](#how-to-run)
    - [The command you're looking for](#the-command-youre-looking-for)
    - [The other command you will look for in the future](#the-other-command-you-will-look-for-in-the-future)
    - [Other useful commands](#other-useful-commands)
        - [General](#general)
        - [L1](#l1)
        - [L2](#l2)
- [Local L1 Rich Wallets](#local-l1-rich-wallets)

## How to run

### The command you're looking for

Running the below command will start both a local L1 (reth for the moment, but `ethereum_rust` in the future) and a local L2 (`ethereum_rust`).

```
make init
```

This command has three steps that can also be run individually:

- `make init-l1` - Starts the L1 (reth) node, creating the volumes necessary for running a docker compose file with reth's docker image.
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
