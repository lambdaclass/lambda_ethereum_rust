# EVM MLIR

[![Telegram Chat][tg-badge]][tg-url]
[![rust](https://github.com/lambdaclass/ethereum_rust/actions/workflows/levm_mlir.yml/badge.svg)](https://github.com/lambdaclass/ethereum_rust/actions/workflows/levm_mlir.yml)
[![license](https://img.shields.io/github/license/lambdaclass/ethereum_rust)](/LICENSE)

[tg-badge]: https://img.shields.io/endpoint?url=https%3A%2F%2Ftg.sumanjay.workers.dev%2Frust_ethereum%2F&logo=telegram&label=chat&color=neon
[tg-url]: https://t.me/rust_ethereum

An EVM-bytecode to machine-bytecode compiler using MLIR and LLVM.

## Status

- Opcodes ✅
- Precompiles 🏗️
- Transaction validation 🏗️
- Pass all EF tests 🏗️

## Getting Started

### Dependencies

- Linux or macOS (aarch64 included) only for now
- LLVM 18 with MLIR: On debian you can use [apt.llvm.org](https://apt.llvm.org/), on macOS you can use brew
- Rust
- Git

### Setup

> This step applies to all operating systems.

Run the following make target to install the dependencies (**both Linux and macOS**):

```bash
make deps
```

#### Linux

Since Linux distributions change widely, you need to install LLVM 18 via your package manager, compile it or check if the current release has a Linux binary.

If you are on Debian/Ubuntu, check out the repository https://apt.llvm.org/
Then you can install with:

```bash
sudo apt-get install llvm-18 llvm-18-dev llvm-18-runtime clang-18 clang-tools-18 lld-18 libpolly-18-dev libmlir-18-dev mlir-18-tools
```

If you decide to build from source, here are some indications:

<details><summary>Install LLVM from source instructions</summary>

```bash
# Go to https://github.com/llvm/llvm-project/releases
# Download the latest LLVM 18 release:
# The blob to download is called llvm-project-18.x.x.src.tar.xz

# For example
wget https://github.com/llvm/llvm-project/releases/download/llvmorg-18.1.4/llvm-project-18.1.4.src.tar.xz
tar xf llvm-project-18.1.4.src.tar.xz

cd llvm-project-18.1.4.src
mkdir build
cd build

# The following cmake command configures the build to be installed to /opt/llvm-18
cmake -G Ninja ../llvm \
   -DLLVM_ENABLE_PROJECTS="mlir;clang;clang-tools-extra;lld;polly" \
   -DLLVM_BUILD_EXAMPLES=OFF \
   -DLLVM_TARGETS_TO_BUILD="Native" \
   -DCMAKE_INSTALL_PREFIX=/opt/llvm-18 \
   -DCMAKE_BUILD_TYPE=RelWithDebInfo \
   -DLLVM_PARALLEL_LINK_JOBS=4 \
   -DLLVM_ENABLE_BINDINGS=OFF \
   -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ -DLLVM_ENABLE_LLD=ON \
   -DLLVM_ENABLE_ASSERTIONS=OFF

ninja install
```

</details>

Setup an environment variable called `MLIR_SYS_180_PREFIX`, `LLVM_SYS_180_PREFIX` and `TABLEGEN_180_PREFIX` pointing to the llvm directory:

```bash
# For Debian/Ubuntu using the repository, the path will be /usr/lib/llvm-18
export MLIR_SYS_180_PREFIX=/usr/lib/llvm-18
export LLVM_SYS_180_PREFIX=/usr/lib/llvm-18
export TABLEGEN_180_PREFIX=/usr/lib/llvm-18
```

Run the deps target to install the other dependencies.

```bash
make deps
```

#### MacOS

The makefile `deps` target (which you should have ran before) installs LLVM 18 with brew for you, afterwards you need to execute the `env-macos.sh` script to setup the environment.

```bash
source scripts/env-macos.sh
```

### Running

To run the compiler, call `cargo run` while passing it a file with the EVM bytecode to compile.
There are some example files under `programs/`, for example:

```bash
cargo run programs/push32.bytecode
```

You can also specify the optimization level:

```bash
cargo run programs/push32.bytecode 3  # ranges from 0 to 3
```

### Testing

To run the project's tests, do `make test`.
To run the [Ethereum Foundation tests](https://github.com/ethereum/tests), use the following commands:

```bash
make ethtests   # downloads the tests
make test-eth
```

To run the solidity tests if you don't have the compiled binaries, you can run

```bash
make compile-solidity-test-examples
```

If you don't have the solc compiler, you should run if you have brew installed on macOs

```bash
make install-solc
```

Or if you are on Linux

```bash
sudo add-apt-repository ppa:ethereum/ethereum
sudo apt-get update
sudo apt-get install solc
```

## Debugging the compiler

### Compile a program

To generate the necessary artifacts, you need to run `cargo run <filepath>`, with `<filepath>` being the path to a file containing the EVM bytecode to compile.

Writing EVM bytecode directly can be a bit difficult, so you can edit [src/main.rs](../src/main.rs), modifying the `program` variable with the structure of your EVM program. After that you just run `cargo run`.

An example edit would look like this:

```rust
fn main() {
    let program = vec![
            Operation::Push0,
            Operation::PushN(BigUint::from(42_u8)),
            Operation::Add,
        ];
    // ...
}
```

### Inspecting the artifacts

The most useful ones to inspect are the MLIR-IR (`<name>.mlir`) and Assembly (`<name>.asm`) files. The first one has a one-to-one mapping with the operations added in the compiler, while the second one contains the instructions that are executed by your machine.

The other generated artifacts are:

- Semi-optimized MLIR-IR (`<name>.after-pass.mlir`)
- LLVM-IR (`<name>.ll`)
- Object file (`<name>.o`)
- Executable (`<name>`)

### Running with a debugger

> [!NOTE]  
> This may not be up-to-date since contracts are no longer compiled into an executable.

Once we have the executable, we can run it with a debugger (here we use `lldb`, but you can use others). To run with `lldb`, use `lldb <name>`.

To run until we reach our main function, we can use:

```lldb
br set -n main
run
```

#### Running a single step

`thread step-inst`

#### Reading registers

All registers: `register read`

The `x0` register: `register read x0`

#### Reading memory

To inspect the memory at `<address>`: `memory read <address>`

To inspect the memory at the address given by the register `x0`: `memory read $x0`

#### Reading the EVM stack

To pretty-print the EVM stack at address `X`: `memory read -s32 -fu -c4 X`

Reference:

- The `-s32` flag groups the bytes in 32-byte chunks.
- The `-fu` flag interprets the chunks as unsigned integers.
- The `-c4` flag includes 4 chunks: the one at the given address plus the three next chunks.

#### Restarting the program

To restart the program, just use `run` again.
