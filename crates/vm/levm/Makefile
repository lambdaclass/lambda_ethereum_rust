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

ethtests: # 📥 Downloads Ethereum Foundation tests.
	@echo "Downloading Ethereum Foundation tests..."
	@echo "Not implemented yet"

test-eth: # 💎 Runs Ethereum Foundation tests.
	@echo "Running Ethereum Foundation tests..."
	@echo "Not implemented yet"
