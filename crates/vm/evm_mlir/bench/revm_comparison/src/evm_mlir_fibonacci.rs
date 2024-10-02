use revm_comparison::{run_with_evm_mlir, FIBONACCI_BYTECODE};
use std::env;

fn main() {
    let runs = env::args().nth(1).unwrap();
    let number_of_iterations = env::args().nth(2).unwrap();

    run_with_evm_mlir(
        FIBONACCI_BYTECODE,
        runs.parse().unwrap(),
        number_of_iterations.parse().unwrap(),
    );
}
