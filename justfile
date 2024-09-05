build:
    cargo build --workspace

lint:
    cargo clippy --all-targets --all-features --workspace -- -D warnings

test crate='*':
    cargo test -p '{{crate}}'

clean:  clean-vectors
    cargo clean
    rm -rf hive

run_image: build_image
    docker run --rm -p 127.0.0.1:8545:8545 ethereum_rust --http.addr 0.0.0.0

build_image:
    docker build -t ethereum_rust .

spectest_version := "v3.0.0"
spectest_artifact := "tests_" + spectest_version + ".tar.gz"
spectest_vectors_dir := "cmd/ef_tests/vectors"

download-vectors: clean-vectors
    curl -L -o {{spectest_artifact}} "https://github.com/ethereum/execution-spec-tests/releases/download/{{spectest_version}}/fixtures_stable.tar.gz"
    mkdir -p {{spectest_vectors_dir}} tmp
    tar -xzf {{spectest_artifact}} -C tmp
    mv tmp/fixtures/blockchain_tests/* {{spectest_vectors_dir}}
    rm -rf tmp {{spectest_artifact}}

clean-vectors:
    rm -rf {{spectest_vectors_dir}}

setup-hive:
    git submodule update --init hive
    cd hive && go build .

test-pattern-default := "/"

# Runs a hive testing suite
# The endpoints tested may be limited by supplying a test pattern in the form "/endpoint_1|enpoint_2|..|enpoint_n"
# For example, to run the rpc-compat suites for eth_chainId & eth_blockNumber you should run:
# `just run-hive ethereum/rpc-compat "/eth_chainId|eth_blockNumber"`
run-hive simulation test-pattern=test-pattern-default: build_image setup-hive
    cd hive && ./hive --sim {{simulation}} --client ethereumrust --sim.limit "{{test-pattern}}" --docker.output

run-hive-debug simulation test-pattern=test-pattern-default: build_image
    cd hive && ./hive --sim {{simulation}} --client ethereumrust --sim.limit "{{test-pattern}}" --docker.output

clean-hive-logs:
    rm -rf ./hive/workspace/logs
