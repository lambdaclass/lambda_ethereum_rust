mod ef_tests_executor;
use ef_tests_executor::test_utils::run_test;

datatest_stable::harness!(
    run_test,
    "ethtests/GeneralStateTests/stTimeConsuming/",
    r"^.*/*.json",
);
