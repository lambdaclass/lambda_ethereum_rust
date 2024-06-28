FROM rust:1.76 as builder

RUN apt-get update && apt-get install -y \
  build-essential \
  libclang-dev \
  libc6 \
  libssl-dev \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/ethrex

COPY . .

RUN cargo build --release

FROM ubuntu:24.04

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/ethrex/target/release/ethrex .

EXPOSE 8545

ENTRYPOINT [ "./ethrex" ]
