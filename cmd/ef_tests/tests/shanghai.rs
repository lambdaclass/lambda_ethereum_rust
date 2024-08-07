use ethereum_rust_evm::SpecId;
use std::path::Path;

use ef_tests::test_runner::{execute_test, parse_test_file, validate_test};

fn parse_and_execute(path: &Path) -> datatest_stable::Result<()> {
    let tests = parse_test_file(path);

    for (test_key, test) in tests {
        let spec = match &*test.network {
            "Shanghai" => SpecId::SHANGHAI,
            "Cancun" => SpecId::CANCUN,
            _ => continue,
        };
        validate_test(&test);
        execute_test(&test_key, &test, spec);
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
    // TODO: Get withdrawals/self_destructing_account.json test to pass
    parse_and_execute,
    "vectors/shanghai/eip4895_withdrawals/",
    r"^(?!.*withdrawals/self_destructing_account\.json$).*"
);
