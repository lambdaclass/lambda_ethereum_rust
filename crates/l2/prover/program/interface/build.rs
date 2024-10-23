fn main() {
    #[cfg(not(clippy))]
    {
        risc0_build::embed_methods();
    }
}
