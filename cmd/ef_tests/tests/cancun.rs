use std::path::Path;

use ef_tests::test_runner::{parse_and_execute_test_file, parse_test_file};

fn cancun_tests(path: &Path) -> datatest_stable::Result<()> {
    parse_and_execute_test_file(path);
    Ok(())
}

#[allow(unused)]
fn parse_test(path: &Path) -> datatest_stable::Result<()> {
    parse_test_file(path);
    Ok(())
}

datatest_stable::harness!(
    cancun_tests,
    "vectors/cancun/",
    // we ignore `create_selfdestruct_same_tx.json` because it has some errors in the encoding
    r"^(?!.*create_selfdestruct_same_tx.json)(.*.json)",
);
