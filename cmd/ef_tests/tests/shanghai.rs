use std::path::Path;

use ef_tests::test_runner::{parse_test_file, test_add_block};

fn parse_and_execute(path: &Path) -> datatest_stable::Result<()> {
    let tests = parse_test_file(path);

    for (test_key, test) in tests {
        test_add_block(&test_key, &test);
    }
    Ok(())
}

datatest_stable::harness!(
    parse_and_execute,
    "vectors/shanghai/eip3855_push0/",
    r"^.*/*",
    parse_and_execute,
    "vectors/shanghai/eip3651_warm_coinbase/",
    r"^.*/*",
    parse_and_execute,
    "vectors/shanghai/eip3860_initcode/",
    r"^.*/*",
    parse_and_execute,
    "vectors/shanghai/eip4895_withdrawals/",
    r"^.*/*"
);
