FROM rust:1.76 as builder

WORKDIR /usr/src/ethrex

COPY . .

RUN cargo build --release

FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
  build-essential \
  libc6 \
  libssl-dev \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/ethrex/target/release/ethrex .
