use std::{collections::HashSet, path::Path};
mod ef_tests_executor;
use ef_tests_executor::test_utils::run_test;

fn get_group_name_from_path(path: &Path) -> String {
    // Gets the parent directory's name.
    // Example: ethtests/GeneralStateTests/stArgsZeroOneBalance/addmodNonConst.json
    // -> stArgsZeroOneBalance
    path.ancestors()
        .into_iter()
        .nth(1)
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

fn get_suite_name_from_path(path: &Path) -> String {
    // Example: ethtests/GeneralStateTests/stArgsZeroOneBalance/addmodNonConst.json
    // -> addmodNonConst
    path.file_stem().unwrap().to_str().unwrap().to_string()
}

fn get_ignored_groups() -> HashSet<String> {
    HashSet::from([
        "stEIP4844-blobtransactions".into(),
        "stEIP5656-MCOPY".into(),
        "stTimeConsuming".into(), // this will be tested with the time_consuming_test binary
        "stRevertTest".into(),
        "eip3855_push0".into(),
        "eip4844_blobs".into(),
        "stSystemOperationsTest".into(),
        "stReturnDataTest".into(),
        "stHomesteadSpecific".into(),
        "stStackTests".into(),
        "eip5656_mcopy".into(),
        "eip6780_selfdestruct".into(),
        "stCallCreateCallCodeTest".into(),
        "stZeroKnowledge2".into(),
        "stDelegatecallTestHomestead".into(),
        "stEIP150singleCodeGasPrices".into(),
        "stSpecialTest".into(),
        "vmIOandFlowOperations".into(),
        "stEIP150Specific".into(),
        "stMemoryStressTest".into(),
        "vmTests".into(),
        "stZeroKnowledge".into(),
        "stLogTests".into(),
        "stBugs".into(),
        "stEIP1559".into(),
        "stMemExpandingEIP150Calls".into(),
        "stTransactionTest".into(),
        "stCodeCopyTest".into(),
        "stNonZeroCallsTest".into(),
        "stMemoryTest".into(),
        "stBadOpcode".into(),
        "eip1153_tstore".into(),
        "stEIP3607".into(),
        "stZeroCallsTest".into(),
        "stAttackTest".into(),
        "stExample".into(),
        "vmArithmeticTest".into(),
        "stQuadraticComplexityTest".into(),
        "stSelfBalance".into(),
        "stEIP3855-push0".into(),
        "stWalletTest".into(),
        "vmLogTest".into(),
    ])
}

fn get_ignored_suites() -> HashSet<String> {
    HashSet::from([
        "ValueOverflow".into(),      // TODO: parse bigint tx value
        "ValueOverflowParis".into(), // TODO: parse bigint tx value
        "blake2B".into(), // this is tested in the blake2B binary because takes a lot of time
    ])
}

fn run_ef_test(path: &Path, contents: String) -> datatest_stable::Result<()> {
    let group_name = get_group_name_from_path(path);
    if get_ignored_groups().contains(&group_name) {
        return Ok(());
    }

    let suite_name = get_suite_name_from_path(path);
    if get_ignored_suites().contains(&suite_name) {
        return Ok(());
    }

    run_test(path, contents)
}

datatest_stable::harness!(run_ef_test, "ethtests/GeneralStateTests/", r"^.*/*.json",);
