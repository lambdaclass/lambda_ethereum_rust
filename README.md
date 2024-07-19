# Lambda Ethereum Rust Execution Client

[![Telegram Chat][tg-badge]][tg-url]
[![rust](https://github.com/lambdaclass/ethereum_rust/actions/workflows/ci.yaml/badge.svg)](https://github.com/lambdaclass/ethereum_rust/actions/workflows/ci.yaml)
[![license](https://img.shields.io/github/license/lambdaclass/ethereum_rust)](/LICENSE)

[tg-badge]: https://img.shields.io/endpoint?url=https%3A%2F%2Ftg.sumanjay.workers.dev%2Frust_ethereum%2F&logo=telegram&label=chat&color=neon
[tg-url]: https://t.me/rust_ethereum

## Philosophy
Many long-established clients accumulate bloat over time. This often occurs due to the need to support legacy features for existing users or through attempts to implement overly ambitious software. The result is often complex, difficult-to-maintain, and error-prone systems.

In contrast, our philosophy is rooted in simplicity:

1. Write minimal code
2. Prioritize clarity
3. Embrace simplicity in design

We believe this approach is the best way to build a client that is both fast and resilient. By adhering to these principles, we will be able to iterate fast and explore next-generation features early, either from the Ethereum roadmap or from innovations from the L2s.

## Usage

### Build

To build the main executable and its crates, run:
```
just build
```

### Test
To run the tests from a crate, run:
```
just test <crate>
```

Or just run all the tests:
```
just test-all
```

### Run

To run a localnet, we can use a fork of [Ethereum Package](https://github.com/ethpandaops/ethereum-package), specifically [this branch](https://github.com/lambdaclass/ethereum-package/tree/ethereum-rust-integration) that adds support to our client.
```
# Make sure to have docker installed

# Kurtosis cli
brew install kurtosis-tech/tap/kurtosis-cli

# Lambdaclass fork of the kurtosis ethereum package.
git clone https://github.com/lambdaclass/ethereum-package.git
cd ethereum-package

# We're now working in the ethereum-rust-integration branch.
git checkout ethereum-rust-integration
```

Create a `network_params.yaml` to set up the localnet
```
participants:
  - el_type: geth
    cl_type: lighthouse
    count: 2
  - el_type: ethereumrust
    cl_type: lighthouse
    vc_count: 0
    validator_count: 0
    count: 1
```

Run the localnet
```
# Make sure we build our docker image with latest changes
docker build -t ethereum_rust .

# Assuming both repos are in the same directory and we're in the rust_ethereum directory:
kurtosis run --enclave lambdanet ../ethereum-package --args-file network_params.yaml
```

## Roadmap

### Milestone 1: RPC Node
Add support to participate in a Cancun localnet as a read-only node.
See issues and progress: https://github.com/lambdaclass/ethereum_rust/milestone/1

### Milestone 2: P2P Network
Implement DevP2P protocol, including RLPx and `eth` capability.
See issues and progress: https://github.com/lambdaclass/ethereum_rust/milestone/2

### Milestone 3: Snap Sync
TBD



