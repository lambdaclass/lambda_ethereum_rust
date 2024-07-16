use std::path::Path;

mod common;

fn cancun_tests(path: &Path) -> datatest_stable::Result<()> {
    common::parse_and_execute_test_file(path);
    Ok(())
}

fn parse_test(path: &Path) -> datatest_stable::Result<()> {
    common::parse_test_file(path);
    Ok(())
}

datatest_stable::harness!(
    cancun_tests,
    "vectors/cancun/",
    r"^.*/point_evaluation_precompile/valid_precompile_calls.json",
    cancun_tests,
    "vectors/cancun/",
    r"^.*beacon_root_contract_calls.json",
    cancun_tests,
    "vectors/cancun/",
    r"^.*mcopy_huge_memory_expansion.json",
    cancun_tests,
    "vectors/cancun/",
    r"^.*invalid_beacon_root_calldata_value.json",
    parse_test,
    "vectors/cancun/",
    r"^.*.json",
);
