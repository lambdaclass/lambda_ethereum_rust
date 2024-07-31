use std::path::Path;

use ef_tests::test_runner::{execute_test, parse_test_file, validate_test};

fn parse_and_execute(path: &Path) -> datatest_stable::Result<()> {
    let tests = parse_test_file(path);

    for (test_key, test) in tests {
        validate_test(&test);
        // TODO: Enable post state check
        execute_test(&test_key, &test, false)
    }
    Ok(())
}

#[allow(unused)]
fn parse_and_validate(path: &Path) -> datatest_stable::Result<()> {
    let tests = parse_test_file(path);

    for (_k, test) in tests {
        validate_test(&test);
    }
    Ok(())
}

//TODO: eip4844_blobs tests are not passing because they expect exceptions.
datatest_stable::harness!(
    parse_and_execute,
    "vectors/cancun/",
    r"eip1153_tstore/.*/.*\.json",
    parse_and_execute,
    "vectors/cancun/",
    r"eip4788_beacon_root/.*/.*\.json",
    parse_and_execute,
    "vectors/cancun/",
    r"eip5656_mcopy/.*/.*\.json",
    parse_and_execute,
    "vectors/cancun/",
    r"eip7516_blobgasfee/.*/.*\.json",
    parse_and_execute,
    "vectors/cancun/",
    r"eip6780_selfdestruct/.*/.*\.json"
);
