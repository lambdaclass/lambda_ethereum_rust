# Ethereum Rust L2 CLI

## Table of Contents

- [Introduction](#introduction)
- [How to install](#how-to-install)
- [Commands](#commands)
  - [`config`](#config)
  - [`stack`](#stack)
  - [`wallet`](#wallet)
  - [`autocomplete`](#autocomplete)
- [Examples](#examples)
    - [`config`](#config)
        - [Adding a configuration](#adding-a-configuration)
        - [Editing exiting configuration interactively](#editing-exiting-configuration-interactively)
        - [Deleting existing configuration interactively](#deleting-existing-configuration-interactively)
        - [Setting a configuration interactively](#setting-a-configuration-interactively)
    - [`stack`](#stack)
        - [Initializing the stack](#initializing-the-stack)
        - [Restarting the stack](#restarting-the-stack)

## How to install

Running the command below will install the `ethereum_rust_l2` binary in your system.

```
cargo install --path .
```

## Commands

```
Usage: ethereum_rust_l2 <COMMAND>

Commands:
  stack         Stack related commands.
  wallet        Wallet interaction commands. The configured wallet could operate both with the L1 and L2 networks. [aliases: w]
  config        CLI config commands.
  autocomplete  Generate shell completion scripts.
  help          Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

> [!IMPORTANT]  
> Most of the CLI interaction needs a configuration to be set. You can set a configuration with the `config` command.

### `config`

```
CLI config commands.

Usage: ethereum_rust_l2 config <COMMAND>

Commands:
  edit     Edit an existing config.
  create   Create a new config.
  set      Set the config to use.
  display  Display a config.
  list     List all configs.
  delete   Delete a config.
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `stack`

```
Stack related commands.

Usage: ethereum_rust_l2 stack <COMMAND>

Commands:
  init      Initializes the L2 network in the provided L1. [aliases: i]
  shutdown  Shutdown the stack.
  start     Starts the stack.
  purge     Cleans up the stack. Prompts for confirmation.
  restart   Re-initializes the stack. Prompts for confirmation.
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `wallet`

> [!NOTE]
> This command is a work in progress. It requires basic L2 functionality to be implemented.

```
Wallet interaction commands. The configured wallet could operate both with the L1 and L2 networks.

Usage: ethereum_rust_l2 wallet <COMMAND>

Commands:
  balance            Get the balance of the wallet.
  deposit            Deposit funds into some wallet.
  finalize-withdraw  Finalize a pending withdrawal.
  transfer           Transfer funds to another wallet.
  withdraw           Withdraw funds from the wallet.
  address            Get the wallet address.
  private-key        Get the wallet private key.
  help               Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `autocomplete`

```
Generate shell completion scripts.

Usage: ethereum_rust_l2 autocomplete <COMMAND>

Commands:
  generate  Generate autocomplete shell script.
  install   Generate and install autocomplete shell script.
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## Examples

### `config`

#### Adding a configuration

<script src="https://asciinema.org/a/B8LhxhYY3IhAXZPywB9t7TeOk.js" id="asciicast-B8LhxhYY3IhAXZPywB9t7TeOk" async="true"></script>

#### Editing exiting configuration interactively

<script src="https://asciinema.org/a/YWGVVJGLdK8EBLi3S5CppkkgW.js" id="asciicast-YWGVVJGLdK8EBLi3S5CppkkgW" async="true"></script>


#### Deleting existing configuration interactively

<script src="https://asciinema.org/a/y49T7RjKKSz0hNhbTfohnhbE3.js" id="asciicast-y49T7RjKKSz0hNhbTfohnhbE3" async="true"></script>

#### Setting a configuration interactively

<script src="https://asciinema.org/a/4XJ8OeVqD4QpMXowfxZDTWGLA.js" id="asciicast-4XJ8OeVqD4QpMXowfxZDTWGLA" async="true"></script>

### `stack`

#### Initializing the stack

<script src="https://asciinema.org/a/PTZuL01FDifYhrQpTxqKTUquy.js" id="asciicast-PTZuL01FDifYhrQpTxqKTUquy" async="true"></script>

#### Restarting the stack

<script src="https://asciinema.org/a/qvSigcYkieMUv5klCyqSQjXIj.js" id="asciicast-qvSigcYkieMUv5klCyqSQjXIj" async="true"></script>
