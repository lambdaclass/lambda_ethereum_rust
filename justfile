build:
    cargo build --workspace

lint:
    cargo clippy --all-targets --all-features --workspace -- -D warnings

test-all:
    cargo test --workspace

test crate:
    cargo test -p {{crate}}

clean:
    cargo clean

run_image: build_image
    docker run --rm -p 127.0.0.1:8545:8545 ethrex --http.addr 0.0.0.0

build_image:
    docker build -t ethrex .

spectest_version := "v2.1.1"
spectest_artifact := "tests_" + spectest_version + ".tar.gz"

download-vectors: clean-vectors
    curl -L -o {{spectest_artifact}} "https://github.com/ethereum/execution-spec-tests/releases/download/{{spectest_version}}/fixtures_develop.tar.gz"
    mkdir -p tmp
    tar -xzf {{spectest_artifact}} -C tmp fixtures/blockchain_tests
    mv tmp/fixtures/blockchain_tests/* ef_tests/vectors/
    rm -rf tmp {{spectest_artifact}}

clean-vectors:
    rm -rf ef_tests/vectors/*
