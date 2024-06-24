build:
    cargo build --all

test:
    cargo test --all

clean:
    cargo clean

build_image:
    docker build -t ethrex .
