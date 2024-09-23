.PHONY: build lint test clean run_image build_image download-test-vectors clean-vectors \
	setup-hive test-pattern-default run-hive run-hive-debug clean-hive-logs

default: stop-localnet-silent ethereum-package checkout-ethereum-package localnet
	docker logs -f $$(docker ps -q --filter ancestor=ethereum_rust)

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

# Only rebuild the image if the source files have changed
STAMP_FILE := .docker_build_stamp
$(STAMP_FILE): $(shell find crates cmd -type f -name '*.rs') Cargo.toml Dockerfile
	docker build -t ethereum_rust .
	touch $(STAMP_FILE)

build_image: $(STAMP_FILE)

SPECTEST_VERSION := v3.0.0
SPECTEST_ARTIFACT := tests_$(SPECTEST_VERSION).tar.gz
SPECTEST_VECTORS_DIR := cmd/ef_tests/vectors

$(SPECTEST_ARTIFACT):
	rm -f tests_*.tar.gz # Delete older versions
	curl -L -o $(SPECTEST_ARTIFACT) "https://github.com/ethereum/execution-spec-tests/releases/download/$(SPECTEST_VERSION)/fixtures_stable.tar.gz"

$(SPECTEST_VECTORS_DIR): $(SPECTEST_ARTIFACT)
	mkdir -p $(SPECTEST_VECTORS_DIR) tmp
	tar -xzf $(SPECTEST_ARTIFACT) -C tmp
	mv tmp/fixtures/blockchain_tests/* $(SPECTEST_VECTORS_DIR)

download-test-vectors: $(SPECTEST_VECTORS_DIR)

clean-vectors:
	rm -rf $(SPECTEST_VECTORS_DIR)

ETHEREUM_PACKAGE_REVISION := c7952d75d72159d03aec423b46797df2ded11f99
# Shallow clones can't specify a single revision, but at least we avoid working
# the whole history by making it shallow since a given date (one day before our
# target revision).
ethereum-package:
	git clone --single-branch --branch ethereum-rust-integration https://github.com/lambdaclass/ethereum-package

checkout-ethereum-package: ethereum-package
	cd ethereum-package && \
		git fetch && \
		git checkout $(ETHEREUM_PACKAGE_REVISION)

localnet: build_image stop-localnet-silent
	kurtosis run --enclave lambdanet ethereum-package --args-file test_data/network_params.yaml

stop-localnet:
	kurtosis enclave stop lambdanet ; kurtosis enclave rm lambdanet --force

stop-localnet-silent:
	@echo "Double checking local net is not already started..."
	@kurtosis enclave stop lambdanet >/dev/null 2>&1 || true
	@kurtosis enclave rm lambdanet >/dev/null 2>&1 || true

HIVE_REVISION := efcd74daee8edc6b5792fafbb1653ea665a02453
# Shallow clones can't specify a single revision, but at least we avoid working
# the whole history by making it shallow since a given date (one day before our
# target revision).
HIVE_SHALLOW_SINCE := 2024-09-02
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

run-hive-debug: build_image setup-hive
	cd hive && ./hive --sim $(SIMULATION) --client ethereumrust --sim.limit "$(TEST_PATTERN)" --docker.output

clean-hive-logs:
	rm -rf ./hive/workspace/logs
