.PHONY: check-deps deps lint fmt test usage check-ethtests revm-comparison build-revm-comparison

#
# Environment detection.
#

UNAME := $(shell uname)

usage:
	@echo "Usage:"
	@echo "    deps:         	Installs the necessary dependencies."
	@echo "    test:         	Runs all tests except Ethereum tests."
	@echo "    test-eth:        Runs only the Ethereum tests."
	@echo "    fmt:          	Formats all files."
	@echo "    lint:         	Checks format and runs lints."
	@echo "    revm-comparison:	Runs benchmarks."

check-deps:
	ifeq (, $(shell which cargo))
		$(error "The cargo command could not be found in your PATH, please install Rust: https://www.rust-lang.org/tools/install")
	endif
	ifndef LLVM_SYS_180_PREFIX
		$(error Could not find a suitable LLVM 18 toolchain, please set LLVM_SYS_180_PREFIX env pointing to the LLVM 18 dir)
	endif
	ifndef MLIR_SYS_180_PREFIX
		$(error Could not find a suitable LLVM 18 toolchain (mlir), please set MLIR_SYS_180_PREFIX env pointing to the LLVM 18 dir)
	endif
	ifndef TABLEGEN_180_PREFIX
		$(error Could not find a suitable LLVM 18 toolchain (tablegen), please set TABLEGEN_180_PREFIX env pointing to the LLVM 18 dir)
	endif
		@echo "[make] LLVM is correctly set at $(MLIR_SYS_180_PREFIX)."

deps: install-nextest
ifeq ($(UNAME), Linux)
deps:
endif
ifeq ($(UNAME), Darwin)
deps: deps-macos
endif

install-nextest:
	@if ! command -v cargo-nextest > /dev/null; then \
		cargo install cargo-nextest; \
	fi

deps-macos:
	-brew install llvm@18 --quiet
	-brew install hyperfine --quiet
	@echo "You need to run source scripts/env-macos.sh to setup the environment."

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-features --benches --examples --tests -- -D warnings

fmt:
	cargo fmt --all

test:
	cargo nextest run --workspace --all-features --no-capture -E 'all() - binary(time_consuming) - binary(normal) - binary(blake2B)'

test-eth: check-ethtests
	cargo nextest run --workspace --all-features --no-capture --test normal

test-eth-slow: check-ethtests
	cargo nextest run --workspace --all-features --no-capture --test time_consuming
	
test-eth-blake2B: check-ethtests
	cargo nextest run --workspace --all-features --no-capture --test blake2B

revm-comparison:
	$(MAKE) build-revm-comparison
	@echo
	@printf "%s" "evm_mlir_factorial result: "
	@target/release/evm_mlir_factorial 1 1000
	@printf "%s" "revm_factorial result: "
	@target/release/revm_factorial 1 1000
	hyperfine -w 5 -r 10 -N \
		-n "evm_mlir_factorial" "target/release/evm_mlir_factorial 100000 1000" \
		-n "revm_factorial" "target/release/revm_factorial 100000 1000"
	@echo
	@printf "%s" "evm_mlir_fibonacci result: "
	@target/release/evm_mlir_fibonacci 1 1000
	@printf "%s" "revm_fibonacci result: "
	@target/release/revm_fibonacci 1 1000
	hyperfine -w 5 -r 10 -N \
		-n "evm_mlir_fibonacci" "target/release/evm_mlir_fibonacci 100000 1000" \
		-n "revm_fibonacci" "target/release/revm_fibonacci 100000 1000"
	@echo

build-revm-comparison:
	cd bench/revm_comparison && \
		cargo build --release \
		--bin evm_mlir_factorial \
		--bin revm_factorial \
		--bin evm_mlir_fibonacci \
		--bin revm_fibonacci

###### Ethereum tests ######

ETHTEST_SHASUM := ".ethtest_shasum"
ETHTEST_VERSION := $(shell cat .ethtest_version)
ETHTEST_TAR := "ethtests-${ETHTEST_VERSION}.tar.gz"

${ETHTEST_TAR}: .ethtest_version
	curl -Lo ${ETHTEST_TAR} https://github.com/ethereum/tests/archive/refs/tags/${ETHTEST_VERSION}.tar.gz

ethtests: ${ETHTEST_TAR}
	mkdir -p "$@"
	tar -xzmf "$<" --strip-components=1 -C "$@"
	@cat ${ETHTEST_SHASUM}
	sha256sum -c ${ETHTEST_SHASUM}

install-solc:
	brew install solidity

compile-solidity-test-examples:
	solc --bin sources/fibonacci.sol -o ./programs; solc --bin sources/factorial.sol -o ./programs; solc --bin sources/recursive_fib.sol -o ./programs
