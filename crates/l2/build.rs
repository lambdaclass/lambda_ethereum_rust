use sp1_helper::build_program_with_args;
use std::{env, fs};

fn main() {
    if env::var("BUILD_ZKVM").is_ok() {
        println!("Checking if zkVM's elf exists.");

        // Check if the 'elf' directory exists
        if fs::metadata("elf").is_err() {
            // If the 'elf' directory does not exist, run the build process
            build_program_with_args("./prover/sp1/execution_program", Default::default());
            build_program_with_args("./prover/sp1/verification_program", Default::default());
        } else {
            println!("Skipping build: 'elf' directory exists.");
        }
    } else {
        println!("BUILD_ZKVM env variable not set. Not building the zkVM.");
    }
}
