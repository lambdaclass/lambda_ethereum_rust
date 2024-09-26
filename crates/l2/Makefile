.PHONY: up down clean up-local-l1 down-local-l1 clean-local-l1 up-l2 down-l2 deploy-l1 deploy-block-executor deploy-inbox

L2_GENESIS_FILE_PATH=../../test_data/genesis-l2.json

up: up-local-l1 up-l2

down: down-local-l1 down-l2

clean: clean-local-l1

# Local L1

up-local-l1:
	mkdir -p volumes/ volumes/reth volumes/reth/data
	docker compose -f docker-compose-l2.yml up -d
	make deploy-l1

down-local-l1:
	docker compose -f docker-compose-l2.yml down

clean-local-l1:
	rm -rf volumes/

# L2

up-l2:
	cargo run --release --manifest-path ../../Cargo.toml --bin ethereum_rust --features l2 -- --network ${L2_GENESIS_FILE_PATH} --http.port 1729 

down-l2:
	pkill -f ethereum_rust || exit 0


# L1 Contract Deployment

L1_RPC_URL=http://localhost:8545
L1_PRIVATE_KEY=0x385c546456b6a603a1cfcaa9ec9494ba4832da08dd6bcf4de9a71e4a01b74924

deploy-l1: deploy-block-executor deploy-inbox

deploy-block-executor:
	forge create crates/l2/contracts/src/l1/BlockExecutor.sol:BlockExecutor --rpc-url ${L1_RPC_URL} --private-key ${L1_PRIVATE_KEY}

deploy-inbox:
	forge create crates/l2/contracts/src/l1/Inbox.sol:Inbox --rpc-url ${L1_RPC_URL} --private-key ${L1_PRIVATE_KEY}
