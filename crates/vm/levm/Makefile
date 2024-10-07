.PHONY: all test clippy fmt usage lint eth-tests

all: test clippy fmt ## 🚀 Runs all tests, linter and formatter

help: ## 📚 Show help for each of the Makefile recipes
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

test: ## 🧪 Runs all tests except Ethereum tests
	cargo test -p 'levm'

lint: ## 🧹 Linter check
	cargo clippy --all-targets --all-features --workspace -- -D warnings

fmt: ## 📄 Runs rustfmt
	cargo fmt --all


###### Ethereum tests ######

ETHTEST_SHASUM := ".ethtest_shasum"
ETHTEST_VERSION := $(shell cat .ethtest_version)
ETHTEST_TAR := "ethtests-${ETHTEST_VERSION}.tar.gz"

${ETHTEST_TAR}: .ethtest_version
	curl -Lo ${ETHTEST_TAR} https://github.com/ethereum/tests/archive/refs/tags/${ETHTEST_VERSION}.tar.gz

ethtests: ${ETHTEST_TAR}  # 📥 Downloads Ethereum Foundation tests.
	mkdir -p "$@"
	tar -xzmf "$<" --strip-components=1 -C "$@"
	@cat ${ETHTEST_SHASUM}
	sha256sum -c ${ETHTEST_SHASUM}

test-eth:  # 💎 Runs Ethereum Foundation tests.
	cargo nextest run --workspace --all-features --no-capture --test normal

test-eth-slow: check-ethtests
	cargo nextest run --workspace --all-features --no-capture --test time_consuming
