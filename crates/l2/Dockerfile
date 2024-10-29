FROM rust:1.80 AS chef

RUN apt-get update && apt-get install -y \
	build-essential \
	libclang-dev \
	libc6 \
	libssl-dev \
	ca-certificates \
	&& rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef

WORKDIR /ethereum_rust

FROM chef AS planner
COPY . .
# Determine the crates that need to be built from dependencies
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /ethereum_rust/recipe.json recipe.json
# Build dependencies only, these remained cached
RUN cargo chef cook --release --recipe-path recipe.json

ENV ETH_RPC_URL=http://ethereum_rust_dev:8545
ENV L1_WATCHER_BRIDGE_ADDRESS=${L1_WATCHER_BRIDGE_ADDRESS}
ENV L1_WATCHER_TOPICS=${L1_WATCHER_TOPICS}
ENV L1_WATCHER_CHECK_INTERVAL_MS=${L1_WATCHER_CHECK_INTERVAL_MS}
ENV L1_WATCHER_MAX_BLOCK_STEP=${L1_WATCHER_MAX_BLOCK_STEP}
ENV L1_WATCHER_L2_PROPOSER_PRIVATE_KEY=${L1_WATCHER_L2_PROPOSER_PRIVATE_KEY}
ENV ENGINE_API_RPC_URL=${ENGINE_API_RPC_URL}
ENV ENGINE_API_JWT_PATH=${ENGINE_API_JWT_PATH}
ENV PROVER_SERVER_LISTEN_IP=${PROVER_SERVER_LISTEN_IP}
ENV PROVER_SERVER_LISTEN_PORT=${PROVER_SERVER_LISTEN_PORT}
ENV PROVER_PROVER_SERVER_ENDPOINT=${PROVER_PROVER_SERVER_ENDPOINT}
ENV PROVER_ELF_PATH=${PROVER_ELF_PATH}
ENV PROPOSER_ON_CHAIN_PROPOSER_ADDRESS=${PROPOSER_ON_CHAIN_PROPOSER_ADDRESS}
ENV PROPOSER_L1_ADDRESS=${PROPOSER_L1_ADDRESS}
ENV PROPOSER_L1_PRIVATE_KEY=${PROPOSER_L1_PRIVATE_KEY}
ENV PROPOSER_INTERVAL_MS=${PROPOSER_INTERVAL_MS}
ENV DEPLOYER_ADDRESS=${DEPLOYER_ADDRESS}
ENV DEPLOYER_PRIVATE_KEY=${DEPLOYER_PRIVATE_KEY}

COPY . .
RUN cargo build --release --features l2

FROM ubuntu:24.04
WORKDIR /usr/local/bin

COPY --from=builder ethereum_rust/target/release/ethereum_rust .
EXPOSE 1729
ENTRYPOINT [ "./ethereum_rust" ]
