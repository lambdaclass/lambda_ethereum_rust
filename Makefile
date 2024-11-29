.PHONY: build lint test clean run-image build-image download-test-vectors clean-vectors \
	setup-hive test-pattern-default run-hive run-hive-debug clean-hive-logs

help: ## 📚 Show help for each of the Makefile recipes
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

build: ## 🔨 Build the client
	cargo build --workspace

lint: ## 🧹 Linter check
	cargo clippy --all-targets --all-features --workspace --exclude ethrex-prover -- -D warnings

SPECTEST_VERSION := v3.0.0
SPECTEST_ARTIFACT := tests_$(SPECTEST_VERSION).tar.gz
SPECTEST_VECTORS_DIR := cmd/ef_tests/ethrex/vectors

CRATE ?= *
test: $(SPECTEST_VECTORS_DIR) ## 🧪 Run each crate's tests
	cargo test -p '$(CRATE)' --workspace --exclude ethrex-prover --exclude ethrex-levm --exclude ef_tests-levm -- --skip test_contract_compilation --skip testito

clean: clean-vectors ## 🧹 Remove build artifacts
	cargo clean
	rm -rf hive

STAMP_FILE := .docker_build_stamp
$(STAMP_FILE): $(shell find crates cmd -type f -name '*.rs') Cargo.toml Dockerfile
	docker build -t ethrex .
	touch $(STAMP_FILE)

build-image: $(STAMP_FILE) ## 🐳 Build the Docker image

run-image: build-image ## 🏃 Run the Docker image
	docker run --rm -p 127.0.0.1:8545:8545 ethrex --http.addr 0.0.0.0

$(SPECTEST_ARTIFACT):
	rm -f tests_*.tar.gz # Delete older versions
	curl -L -o $(SPECTEST_ARTIFACT) "https://github.com/ethereum/execution-spec-tests/releases/download/$(SPECTEST_VERSION)/fixtures_stable.tar.gz"

$(SPECTEST_VECTORS_DIR): $(SPECTEST_ARTIFACT)
	mkdir -p $(SPECTEST_VECTORS_DIR) tmp
	tar -xzf $(SPECTEST_ARTIFACT) -C tmp
	mv tmp/fixtures/blockchain_tests/* $(SPECTEST_VECTORS_DIR)

download-test-vectors: $(SPECTEST_VECTORS_DIR) ## 📥 Download test vectors

clean-vectors: ## 🗑️  Clean test vectors
	rm -rf $(SPECTEST_VECTORS_DIR)

ETHEREUM_PACKAGE_REVISION := 5b49d02ee556232a73ea1e28000ec5b3fca1073f
# Shallow clones can't specify a single revision, but at least we avoid working
# the whole history by making it shallow since a given date (one day before our
# target revision).
ethereum-package:
	git clone --single-branch --branch ethrex-integration https://github.com/lambdaclass/ethereum-package

checkout-ethereum-package: ethereum-package ## 📦 Checkout specific Ethereum package revision
	cd ethereum-package && \
		git fetch && \
		git checkout $(ETHEREUM_PACKAGE_REVISION)

localnet: stop-localnet-silent build-image checkout-ethereum-package ## 🌐 Start local network
	kurtosis run --enclave lambdanet ethereum-package --args-file test_data/network_params.yaml
	docker logs -f $$(docker ps -q --filter ancestor=ethrex)

stop-localnet: ## 🛑 Stop local network
	kurtosis enclave stop lambdanet
	kurtosis enclave rm lambdanet --force

stop-localnet-silent:
	@echo "Double checking local net is not already started..."
	@kurtosis enclave stop lambdanet >/dev/null 2>&1 || true
	@kurtosis enclave rm lambdanet --force >/dev/null 2>&1 || true

HIVE_REVISION := f220e0c55fb222aaaffdf17d66aa0537cd16a67a
# Shallow clones can't specify a single revision, but at least we avoid working
# the whole history by making it shallow since a given date (one day before our
# target revision).
HIVE_SHALLOW_SINCE := 2024-09-02
hive:
	git clone --single-branch --branch master --shallow-since=$(HIVE_SHALLOW_SINCE) https://github.com/lambdaclass/hive
	cd hive && git checkout --detach $(HIVE_REVISION) && go build .

setup-hive: hive ## 🐝 Set up Hive testing framework
	if [ "$$(cd hive && git rev-parse HEAD)" != "$(HIVE_REVISION)" ]; then \
		cd hive && \
		git checkout master && \
		git fetch --shallow-since=$(HIVE_SHALLOW_SINCE) && \
		git checkout --detach $(HIVE_REVISION) && go build . ;\
	fi

TEST_PATTERN ?= /

# Runs a hive testing suite
# The endpoints tested may be limited by supplying a test pattern in the form "/endpoint_1|enpoint_2|..|enpoint_n"
# For example, to run the rpc-compat suites for eth_chainId & eth_blockNumber you should run:
# `make run-hive SIMULATION=ethereum/rpc-compat TEST_PATTERN="/eth_chainId|eth_blockNumber"`
run-hive: build-image setup-hive ## 🧪 Run Hive testing suite
	cd hive && ./hive --client ethrex --sim $(SIMULATION) --sim.limit "$(TEST_PATTERN)"

run-hive-debug: build-image setup-hive ## 🐞 Run Hive testing suite in debug mode
	cd hive && ./hive --sim $(SIMULATION) --client ethrex --sim.limit "$(TEST_PATTERN)" --docker.output

clean-hive-logs: ## 🧹 Clean Hive logs
	rm -rf ./hive/workspace/logs

loc:
	cargo run -p loc

hive-stats:
	git clone --quiet --single-branch --branch master --shallow-since=$(HIVE_SHALLOW_SINCE) https://github.com/lambdaclass/hive || true
	cd hive && git checkout --quiet --detach $(HIVE_REVISION) && go build .
	if [ "$$(cd hive && git rev-parse HEAD)" != "$(HIVE_REVISION)" ]; then \
		cd hive && \
		git checkout --quiet master && \
		git fetch --quiet --shallow-since=$(HIVE_SHALLOW_SINCE) && \
		git checkout --quiet --detach $(HIVE_REVISION) && go build . ;\
	fi
# make xxx FILE_NAM
E=rpc-compact SIMULATION=ethereum/rpc-compat TEST_PATTERN="/eth_chainId|eth_getTransactionByBlockHashAndIndex|eth_getTransactionByBlockNumberAndIndex|eth_getCode|eth_getStorageAt|eth_call|eth_getTransactionByHash|eth_getBlockByHash|eth_getBlockByNumber|eth_createAccessList|eth_getBlockTransactionCountByNumber|eth_getBlockTransactionCountByHash|eth_getBlockReceipts|eth_getTransactionReceipt|eth_blobGasPrice|eth_blockNumber|ethGetTransactionCount|debug_getRawHeader|debug_getRawBlock|debug_getRawTransaction|debug_getRawReceipts|eth_estimateGas|eth_getBalance|eth_sendRawTransaction|eth_getProof|eth_getLogs"
# make xxx FILE_NAME=devp2p SIMULATION=devp2p TEST_PATTERN="discv4"
	make run-hive SIMULATION=devp2p TEST_PATTERN="/AccountRange|StorageRanges|ByteCodes|TrieNodes"
	make run-hive SIMULATION=devp2p TEST_PATTERN="eth/Status|GetBlockHeaders|SimultaneousRequests|SameRequestID|ZeroRequestID|GetBlockBodies|MaliciousHandshake|MaliciousStatus|Transaction|InvalidTxs"
	make run-hive SIMULATION=ethereum/engine TEST_PATTERN="engine-(auth|exchange-capabilities)/"
	make run-hive SIMULATION=ethereum/sync TEST_PATTERN="engine-cancun/Blob Transactions On Block 1|Blob Transaction Ordering, Single|Blob Transaction Ordering, Multiple Accounts|Replace Blob Transactions|Parallel Blob Transactions|ForkchoiceUpdatedV3 Modifies Payload ID on Different Beacon Root|NewPayloadV3 After Cancun|NewPayloadV3 Versioned Hashes|Incorrect BlobGasUsed|Bad Hash|ParentHash equals BlockHash|RPC:|in ForkchoiceState|Unknown|Invalid PayloadAttributes|Unique|ForkchoiceUpdated Version on Payload Request|Re-Execute Payload|In-Order Consecutive Payload|Multiple New Payloads|Valid NewPayload->|NewPayload with|Payload Build after|Build Payload with|Invalid Missing Ancestor ReOrg, StateRoot|Re-Org Back to|Re-org to Previously|Safe Re-Org to Side Chain|Transaction Re-Org, Re-Org Back In|Re-Org Back into Canonical Chain, Depth=5|Suggested Fee Recipient Test|PrevRandao Opcode|Invalid NewPayload, [^R][^e]|Fork ID Genesis=0, Cancun=0|Fork ID Genesis=0, Cancun=1|Fork ID Genesis=1, Cancun=0|Fork ID Genesis=1, Cancun=2, Shanghai=2"


# xxx:
# 	rm -rf hive/workspace $(FILE_NAME)_logs
# 	make run-hive SIMULATION="$(SIMULATION)" TEST_PATTERN="$(TEST_PATTERN)"
# 	mkdir -p $(FILE_NAME)_logs
# 	mv hive/workspace/logs/*-*.json $(FILE_NAME)_logs

stats: 
	cargo run --quiet --release -p loc -- --summary && echo
	cargo test --quiet -p ef_tests-levm --test ef_tests_levm -- --summary && echo
	make hive-stats
	cargo run --quiet --release -p hive_report
