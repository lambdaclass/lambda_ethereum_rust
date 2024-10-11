# VM

The Ethereum Virtual Machine (EVM) is a virtual machine that is used to execute smart contracts on the Ethereum network.

More information can be found [here](https://ethereum.org/en/developers/docs/evm/).

Currently, we're working on two implementations of the EVM:
- `levm`: A EVM interpreter written in Rust. See more [here](./levm).
- `levm_mlir`: An EVM-bytecode to machine-bytecode compiler that uses MLIR and LLVM. See more [here](./levm_mlir).

Both efforts are a work in progress and have not been integrated into the client yet.
