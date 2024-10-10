#![allow(unused)]
use super::models::TestSuite;
use std::io::BufReader;
use std::path::PathBuf;
use std::{fs::File, path::Path};
use walkdir::{DirEntry, WalkDir};

// These fail to parse due to invalid JSON.
pub const INVALID_PATHS: [&str; 2] = [
    "ethtests/GeneralStateTests/stTransactionTest/ValueOverflowParis.json",
    "ethtests/GeneralStateTests/stTransactionTest/ValueOverflow.json",
];

fn filter_json(entry: DirEntry) -> Option<DirEntry> {
    match entry.path().extension() {
        Some(ext) if "json" == ext => Some(entry),
        _ => None,
    }
}

fn filter_not_valid(entry: DirEntry) -> Option<DirEntry> {
    match entry.path().to_str() {
        Some(path) => {
            let filtered = INVALID_PATHS.iter().any(|x| path.contains(*x));
            if filtered {
                None
            } else {
                Some(entry)
            }
        }
        _ => None,
    }
}

pub fn parse_test_suite(entry: DirEntry) -> (PathBuf, TestSuite) {
    let file = File::open(entry.path())
        .unwrap_or_else(|_| panic!("Failed to open file {}", entry.path().display()));
    let reader = BufReader::new(file);
    let test: TestSuite = serde_json::from_reader(reader)
        .unwrap_or_else(|_| panic!("Failed to parse JSON test {}", entry.path().display()));
    (PathBuf::from(entry.path()), test)
}

pub fn parse_tests(directory_path: impl AsRef<Path>) -> impl Iterator<Item = (PathBuf, TestSuite)> {
    WalkDir::new(directory_path)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(filter_not_valid)
        .filter_map(filter_json)
        .map(parse_test_suite)
}
