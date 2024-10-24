RISC0_DEV_MODE?=1
perf_test_proving:
	@echo "Using RISC0_DEV_MODE: ${RISC0_DEV_MODE}"
	RISC0_DEV_MODE=${RISC0_DEV_MODE} RUST_LOG="debug" cargo test --test perf_zkvm --features build_zkvm -- --show-output
.PHONY: perf_test_proving
