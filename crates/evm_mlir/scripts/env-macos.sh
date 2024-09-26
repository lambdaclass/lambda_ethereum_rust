#!/usr/bin/env bash

# This script is only useful on macOS using brew.
# It sets the LLVM environment variables.
export LIBRARY_PATH=/opt/homebrew/lib
MLIR_SYS_180_PREFIX="$(brew --prefix llvm@18)"
LLVM_SYS_181_PREFIX="$(brew --prefix llvm@18)"
TABLEGEN_180_PREFIX="$(brew --prefix llvm@18)"

export MLIR_SYS_180_PREFIX
export LLVM_SYS_181_PREFIX
export TABLEGEN_180_PREFIX
