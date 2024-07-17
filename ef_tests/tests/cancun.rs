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
    "vectors/cancun/eip1153_tstore/",
    r"^.*gas_usage.json",
);
