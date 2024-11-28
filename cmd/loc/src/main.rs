use std::path::PathBuf;
use tokei::{Config, LanguageType, Languages};

const CARGO_MANIFEST_DIR: &str = std::env!("CARGO_MANIFEST_DIR");

fn main() {
    let ethrex = PathBuf::from(CARGO_MANIFEST_DIR).join("../../");
    let levm = PathBuf::from(CARGO_MANIFEST_DIR).join("../../crates/vm");
    let ethrex_l2 = PathBuf::from(CARGO_MANIFEST_DIR).join("../../crates/l2");

    let config = Config::default();

    let mut languages = Languages::new();
    languages.get_statistics(&[ethrex.clone()], &["tests"], &config);
    let ethrex_loc = &languages.get(&LanguageType::Rust).unwrap();

    let mut languages = Languages::new();
    languages.get_statistics(&[levm], &["tests"], &config);
    let levm_loc = &languages.get(&LanguageType::Rust).unwrap();

    let mut languages = Languages::new();
    languages.get_statistics(&[ethrex_l2], &["tests"], &config);
    let ethrex_l2_loc = &languages.get(&LanguageType::Rust).unwrap();

    let report = format!(
        r#"*ethrex L1:* {}\n*ethrex L2:* {}\n*levm:* {}\n*ethrex (total):* {}"#,
        ethrex_loc.code - ethrex_l2_loc.code - levm_loc.code,
        ethrex_l2_loc.code,
        levm_loc.code,
        ethrex_loc.code,
    );

    std::fs::write("loc_report.txt", report).unwrap();
}
