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
