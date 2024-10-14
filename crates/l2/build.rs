use sp1_helper::build_program_with_args;
use std::fs;

fn main() {
    if fs::metadata("elf").is_err() {
        // If the 'elf' directory does not exist, run the build process
        build_program_with_args("./prover/sp1/execution_program", Default::default());
        build_program_with_args("./prover/sp1/verification_program", Default::default());
    } else {
        println!("Skipping build: 'elf' directory exists.");
    }
}
