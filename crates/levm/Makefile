.PHONY: all test clippy fmt usage lint eth-tests

all: test clippy fmt

usage:
	@echo "Usage:"
	@echo "    test:         	Runs all tests except Ethereum tests."
	@echo "    lint:         	Runs clippy and fmt."
	@echo "    fmt:          	Runs rustfmt."
	@echo "    eth-tests:    	Runs Ethereum Foundation tests."

test:
	cargo test -p 'levm'

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-features --benches --examples --tests -- -D warnings

fmt:
	cargo fmt --all

ethtests: # TODO.
	@echo "Downloads Ethereum Foundation tests"
	@echo "Not implemented yet"

test-eth: # TODO.
	@echo "Runs Ethereum Foundation tests"
	@echo "Not implemented yet"
