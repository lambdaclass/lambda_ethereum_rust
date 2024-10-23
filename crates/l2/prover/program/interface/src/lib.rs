#[cfg(not(clippy))]
pub mod methods {
    include!(concat!(env!("OUT_DIR"), "/methods.rs"));
}
