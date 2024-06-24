build:
    cargo build --all

test-all:
    cargo test --all

test crate:
    cargo test -p {{crate}}

clean:
    cargo clean

build_image:
    docker build -t ethrex .
