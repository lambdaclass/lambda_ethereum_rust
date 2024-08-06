use std::path::Path;

use ef_tests::test_runner::{execute_test, parse_test_file, validate_test};

fn parse_and_execute(path: &Path) -> datatest_stable::Result<()> {
    let tests = parse_test_file(path);

    for (test_key, test) in tests {
        validate_test(&test);
        execute_test(&test_key, &test);
    }
    Ok(())
}

datatest_stable::harness!(
    parse_and_execute,
    "vectors/shanghai/eip3855_push0/",
    r"^.*/*",
    //parse_and_execute,
    //"vectors/shanghai/eip3651_warm_coinbase/",
    //r"^.*/*",
    // parse_and_execute,
    // "vectors/shanghai/eip3860_initcode/",
    // r"^.*/*",
    parse_and_execute,
    "vectors/shanghai/eip4895_withdrawals/",
    r"^.*/*",
);
