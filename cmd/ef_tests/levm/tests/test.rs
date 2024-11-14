use crate::report;

#[test]
fn testito() {
    let report = runner::run_ef_tests().unwrap();
    println!("{report}");
}
