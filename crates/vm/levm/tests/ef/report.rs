// Note: I use this to do not affect the EF tests logic with this side effects
// The cost to add this would be to return a Result<(), InternalError> in EFTestsReport methods
#![allow(clippy::arithmetic_side_effects)]

use colored::Colorize;
use std::{collections::HashMap, fmt};

#[derive(Debug, Default)]
pub struct EFTestsReport {
    total_passed: u64,
    total_failed: u64,
    total_run: u64,
    test_reports: HashMap<String, EFTestReport>,
    passed_tests: Vec<String>,
    failed_tests: Vec<(String, (usize, usize, usize), String)>,
}

#[derive(Debug, Default, Clone)]
pub struct EFTestReport {
    passed: u64,
    failed: u64,
    run: u64,
    // passed_tests: Vec<String>,
    failed_tests: Vec<((usize, usize, usize), String)>,
}

impl EFTestsReport {
    pub fn register_pass(&mut self, test_name: &str) {
        self.total_passed += 1;
        self.passed_tests.push(test_name.to_string());
        self.total_run += 1;

        let report = self.test_reports.entry(test_name.to_string()).or_default();
        report.passed += 1;
        //report.passed_tests.push(tx_indexes);
        report.run += 1;
    }

    pub fn register_fail(
        &mut self,
        tx_indexes: (usize, usize, usize),
        test_name: &str,
        reason: &str,
    ) {
        self.total_failed += 1;
        self.failed_tests
            .push((test_name.to_owned(), tx_indexes, reason.to_owned()));
        self.total_run += 1;

        let report = self.test_reports.entry(test_name.to_string()).or_default();
        report.failed += 1;
        report.failed_tests.push((tx_indexes, reason.to_owned()));
        report.run += 1;
    }

    pub fn progress(&self) -> String {
        format!(
            "{}: {} {} {}",
            "Ethereum Foundation Tests Run".bold(),
            format!("{} passed", self.total_passed).green().bold(),
            format!("{} failed", self.total_failed).red().bold(),
            format!("{} total run", self.total_run).blue().bold(),
        )
    }
}

impl fmt::Display for EFTestsReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Report:")?;
        writeln!(f, "Total results: {}", self.progress())?;
        for (test_name, report) in self.test_reports.clone() {
            if report.failed == 0 {
                continue;
            }
            writeln!(f)?;
            writeln!(
                f,
                "Test results for {}: {} {} {}",
                test_name,
                format!("{} passed", report.passed).green().bold(),
                format!("{} failed", report.failed).red().bold(),
                format!("{} run", report.run).blue().bold(),
            )?;
            for failing_test in report.failed_tests.clone() {
                writeln!(
                    f,
                    "(data_index: {}, gas_limit_index: {}, value_index: {}). Err: {}",
                    failing_test.0 .0,
                    failing_test.0 .1,
                    failing_test.0 .2,
                    failing_test.1.bright_red().bold()
                )?;
            }
        }

        Ok(())
    }
}
