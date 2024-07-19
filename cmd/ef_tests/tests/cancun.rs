use std::path::Path;

use ef_tests::test_runner::{parse_and_execute_test_file, parse_test_file};

fn cancun_tests(path: &Path) -> datatest_stable::Result<()> {
    parse_and_execute_test_file(path);
    Ok(())
}

fn parse_test(path: &Path) -> datatest_stable::Result<()> {
    parse_test_file(path);
    Ok(())
}

datatest_stable::harness!(
    cancun_tests,
    "vectors/cancun/eip4844_blobs/",
    r"^.*/valid_precompile_calls.json",
    cancun_tests,
    "vectors/cancun/eip4788_beacon_root/",
    r"^.*beacon_root_contract_calls.json",
    cancun_tests,
    "vectors/cancun/eip5656_mcopy/",
    r"^.*mcopy_huge_memory_expansion.json",
    cancun_tests,
    "vectors/cancun/eip4788_beacon_root/",
    r"^.*invalid_beacon_root_calldata_value.json",
    parse_test,
    "vectors/cancun/",
    r"^.*.json",
);
