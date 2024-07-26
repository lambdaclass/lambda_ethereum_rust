use std::path::Path;

use ef_tests::test_runner::{execute_test, parse_test_file, validate_test};

fn parse_and_execute(path: &Path) -> datatest_stable::Result<()> {
    let tests = parse_test_file(path);

    for (_k, test) in tests {
        validate_test(&test);
        execute_test(&test)
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

datatest_stable::harness!(
    // Uncomment to run all tests in ef_tests/vectors/cancun:
    //parse_and_execute,
    //"vectors/cancun/",
    //r"eip1153_tstore/.*/.*\.json",
    parse_and_execute,
    "vectors/cancun/",
    r"eip4788_beacon_root/beacon_root_contract/tx_to_beacon_root_contract\.json",
    //parse_and_execute,
    //"vectors/cancun/",
    //r"eip4844_blobs/point_evaluation_precompile/valid_precompile_calls.json",
    //parse_and_execute,
    //"vectors/cancun/",
    //r"eip5656_mcopy/.*/.*\.json",
    //parse_and_execute,
    //"vectors/cancun/",
    //r"eip6780_selfdestruct/.*/.*\.json",
    //parse_and_validate,
    //"vectors/cancun/",
    //r"eip7516_blobgasfee/.*/.*\.json" // we ignore `create_selfdestruct_same_tx.json` because it has some errors in the encoding
    //r"^(?!.*create_selfdestruct_same_tx.json)(.*.json)",
);
