.PHONY: build lint test clean run_image build_image download-vectors clean-vectors \
	setup-hive test-pattern-default run-hive run-hive-debug clean-hive-logs

build:
	cargo build --workspace

lint:
	cargo clippy --all-targets --all-features --workspace -- -D warnings

CRATE ?= *
test:
	cargo test -p '$(CRATE)'

clean:  clean-vectors
	cargo clean
	rm -rf hive

run_image: build_image
	docker run --rm -p 127.0.0.1:8545:8545 ethereum_rust --http.addr 0.0.0.0

build_image:
	docker build -t ethereum_rust .

SPECTEST_VERSION := v3.0.0
SPECTEST_ARTIFACT := tests_$(SPECTEST_VERSION).tar.gz
SPECTEST_VECTORS_DIR := cmd/ef_tests/vectors

$(SPECTEST_ARTIFACT):
	rm tests_*.tar.gz # Delete older versions
	curl -L -o $(SPECTEST_ARTIFACT) "https://github.com/ethereum/execution-spec-tests/releases/download/$(SPECTEST_VERSION)/fixtures_stable.tar.gz"

$(SPECTEST_VECTORS_DIR): $(SPECTEST_ARTIFACT)
	mkdir -p $(SPECTEST_VECTORS_DIR) tmp
	tar -xzf $(SPECTEST_ARTIFACT) -C tmp
	mv tmp/fixtures/blockchain_tests/* $(SPECTEST_VECTORS_DIR)

download-vectors: $(SPECTEST_VECTORS_DIR)

clean-vectors:
	rm -rf $(SPECTEST_VECTORS_DIR)

ETHEREUM_PACKAGE_REVISION := c7952d75d72159d03aec423b46797df2ded11f99
# Shallow clones can't specify a single revision, but at least we avoid working
# the whole history by making it shallow since a given date (one day before our
# target revision).
ETHEREUM_PACKAGE_SHALLOW_SINCE := 2024-08-23
ethereum-package:
	git clone --single-branch --branch ethereum-rust-integration --shallow-since=$(ETHEREUM_PACKAGE_SHALLOW_SINCE) https://github.com/lambdaclass/ethereum-package

checkout-ethereum-package: ethereum-package
	cd ethereum-package && \
		git fetch --shallow-since=$(ETHEREUM_PACKAGE_SHALLOW_SINCE) && \
		git checkout $(ETHEREUM_PACKAGE_REVISION)

HIVE_REVISION := 9bff4bbf4439336bd037a444560516dd49ff1c40
# Shallow clones can't specify a single revision, but at least we avoid working
# the whole history by making it shallow since a given date (one day before our
# target revision).
HIVE_SHALLOW_SINCE := 2024_09_02
hive:
	git clone --single-branch --branch master --shallow-since=$(HIVE_SHALLOW_SINCE) https://github.com/lambdaclass/hive

setup-hive: hive
	cd hive && \
		git fetch --shallow-since=$(HIVE_SHALLOW_SINCE) && \
		git checkout $(HIVE_REVISION) && go build .

TEST_PATTERN ?= /

# Runs a hive testing suite
# The endpoints tested may be limited by supplying a test pattern in the form "/endpoint_1|enpoint_2|..|enpoint_n"
# For example, to run the rpc-compat suites for eth_chainId & eth_blockNumber you should run:
# `make run-hive SIMULATION=ethereum/rpc-compat TEST_PATTERN="/eth_chainId|eth_blockNumber"`
run-hive: build_image setup-hive
	cd hive && ./hive --sim $(SIMULATION) --client ethereumrust --sim.limit "$(TEST_PATTERN)"

run-hive-debug: build_image
	cd hive && ./hive --sim $(SIMULATION) --client ethereumrust --sim.limit "$(TEST_PATTERN)" --docker.output

clean-hive-logs:
	rm -rf ./hive/workspace/logs
