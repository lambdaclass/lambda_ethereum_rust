build:
    cargo build --workspace

test-all:
    cargo test --workspace

test crate:
    cargo test -p {{crate}}

clean:
    cargo clean

build_image:
    docker build -t ethrex .
