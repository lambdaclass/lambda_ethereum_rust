mod deserialize;
mod report;
mod runner;
mod test;

#[test]
fn testito() {
    let report = runner::run_ef_tests().unwrap();
    println!("{report}");
}
