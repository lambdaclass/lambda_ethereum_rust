#![cfg(feature = "ethereum_foundation_tests")]
mod ef_tests_executor;
use ef_tests_executor::test_utils::run_test;

use std::{collections::HashSet, path::Path};

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
        "stEIP1153-transientStorage".into(),
        "eip1153_tstore".into(),
        "eip3651_warm_coinbase".into(),
        "stEIP3651-warmcoinbase".into(),
        "stEIP3860-limitmeterinitcode".into(),
        "eip3860_initcode".into(),
        "stInitCodeTest".into(),
        "stArgsZeroOneBalance".into(),
        "stCallDelegateCodesHomestead".into(),
        "stDelegatecallTestHomestead".into(),
        "stCodeSizeLimit".into(),
        "stCreate2".into(),
        "stCreateTest".into(),
        "stRecursiveCreate".into(),
        "stCallCreateCallCodeTest".into(),
        "stCallCodes".into(),
        "stEIP158Specific".into(),
        "stEIP4844-blobtransactions".into(),
        "eip4844_blobs".into(),
        "stEIP5656-MCOPY".into(),
        "eip5656_mcopy".into(),
        "stEIP2930".into(),
        "stRandom".into(),
        "stRandom2".into(),
        "stRefundTest".into(),
        "stSStoreTest".into(),
        "stStaticFlagEnabled".into(),
        "stStaticCall".into(),
        "stRevertTest".into(),
        "stTimeConsuming".into(), // this will be tested with the time_consuming_test binary
        "eip3855_push0".into(),
        "stEIP3855-push0".into(),
        "stSystemOperationsTest".into(),
        "stReturnDataTest".into(),
        "stHomesteadSpecific".into(),
        "stStackTests".into(),
        "eip6780_selfdestruct".into(),
        "stPreCompiledContracts".into(),
        "stPreCompiledContracts2".into(),
        "eip198_modexp_precompile".into(),
        "stZeroKnowledge".into(),
        "stZeroKnowledge2".into(),
        "stEIP150singleCodeGasPrices".into(),
        "stEIP150Specific".into(),
        "stMemExpandingEIP150Calls".into(),
        "stSpecialTest".into(),
        "stExtCodeHash".into(),
        "stMemoryStressTest".into(),
        "stMemoryTest".into(),
        "vmTests".into(),
        "vmArithmeticTest".into(),
        "vmLogTest".into(),
        "vmPerformance".into(),
        "vmIOandFlowOperations".into(),
        "stLogTests".into(),
        "stBugs".into(),
        "stEIP1559".into(),
        "stTransactionTest".into(),
        "stCodeCopyTest".into(),
        "stNonZeroCallsTest".into(),
        "stZeroCallsTest".into(),
        "stZeroCallsRevert".into(),
        "stBadOpcode".into(),
        "stSolidityTest".into(),
        "yul".into(),
        "stEIP3607".into(),
        "stAttackTest".into(),
        "stExample".into(),
        "stQuadraticComplexityTest".into(),
        "stSelfBalance".into(),
        "stWalletTest".into(),
        "stTransitionTest".into(),
    ])
}

// ls -1 | wc -l -> count number of files in dir

// Current not ignored groups:
// - stShift
// 41 tests
// - eip7516_blobgasfee
// 3 tests
// - Pyspecs/frontier/opcodes
// 2 tests
// - eip2930_access_list
// 1 test
// - eip1344_chainid
// 1 test
// - stChainId
// 2 tests
// - vmBitwiseLogicOperation
// 11 tests
// - stCallDelegateCodesCallCodeHomestead
// 58 tests
// - stSLoadTest
// 1 test
// Total: 120 tests

fn get_ignored_suites() -> HashSet<String> {
    HashSet::from([
        "ValueOverflow".into(),      // TODO: parse bigint tx value
        "ValueOverflowParis".into(), // TODO: parse bigint tx value
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
