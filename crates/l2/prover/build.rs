use sp1_helper::build_program_with_args;

// Builds the zkVM's program if BUILD_ZKVM env variable is defined.
fn main() {
    if std::env::var("BUILD_ZKVM").is_ok() {
        println!("Building ZKVM");
        build_program_with_args("./program", Default::default());
    } else {
        println!("Not Building ZKVM");
    }
}
