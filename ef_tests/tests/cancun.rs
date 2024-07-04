use std::path::Path;

mod common;

fn eip4788_tests(path: &Path) -> datatest_stable::Result<()> {
    common::parse_and_execute_test_file(path);
    Ok(())
}

datatest_stable::harness!(
    eip4788_tests,
    "vectors/cancun/eip4788_beacon_root",
    r"^.*beacon_root_contract_calls.json"
);
