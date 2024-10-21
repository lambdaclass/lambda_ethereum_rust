use sp1_helper::build_program_with_args;

// Builds the zkVM's program
// The L1 docker container is building the prover's program
// Using this variable to avoid compiling it, is it ok? 
fn main() {
    if std::env::var("BUILD_ZKVM").is_ok() {
        println!("Building ZKVM");
        build_program_with_args("./program", Default::default());
    } else {
        println!("Not Building ZKVM");
    }
}
