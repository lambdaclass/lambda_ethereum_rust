use crate::{report::EFTestsReport, types::EFTest, utils};
use ethereum_rust_core::{
    types::{code_hash, AccountInfo},
    H256,
};
use ethereum_rust_levm::{
    errors::{TransactionReport, VMError},
    vm::VM,
};
use ethereum_rust_storage::AccountUpdate;
use keccak_hash::keccak;
use spinoff::{spinners::Dots, Color, Spinner};
use std::{collections::HashMap, error::Error};

pub fn run_ef_tests() -> Result<EFTestsReport, Box<dyn Error>> {
    let mut report = EFTestsReport::default();
    let cargo_manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ef_general_state_tests_path = cargo_manifest_dir.join("vectors/GeneralStateTests");
    let mut spinner = Spinner::new(Dots, report.to_string(), Color::Cyan);
    for test_dir in std::fs::read_dir(ef_general_state_tests_path)?.flatten() {
        for test in std::fs::read_dir(test_dir.path())?
            .flatten()
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "json")
                    .unwrap_or(false)
            })
        {
            // TODO: Figure out what to do with overflowed value: 0x10000000000000000000000000000000000000000000000000000000000000001.
            // Deserialization fails because the value is too big for U256.
            if test
                .path()
                .file_name()
                .is_some_and(|name| name == "ValueOverflowParis.json")
            {
                continue;
            }
            let test_result = run_ef_test(
                serde_json::from_reader(std::fs::File::open(test.path())?)?,
                &mut report,
            );
            if test_result.is_err() {
                continue;
            }
        }
        spinner.update_text(report.progress());
    }
    spinner.success(&report.progress());
    let mut spinner = Spinner::new(Dots, "Loading report...".to_owned(), Color::Cyan);
    spinner.success(&report.to_string());
    Ok(report)
}

pub fn run_ef_test(test: EFTest, report: &mut EFTestsReport) -> Result<(), Box<dyn Error>> {
    let mut evm = prepare_vm(&test, report)?;
    ensure_pre_state(&evm, &test, report)?;
    let execution_result = evm.transact();
    ensure_post_state(execution_result, &test, report)?;
    Ok(())
}

pub fn prepare_vm(test: &EFTest, report: &mut EFTestsReport) -> Result<VM, Box<dyn Error>> {
    match VM::try_from(test) {
        Ok(vm) => Ok(vm),
        Err(err) => {
            let error_reason = format!("VM initialization failed: {err:?}");
            report.register_fail(&test.name, &error_reason);
            Err(error_reason.into())
        }
    }
}

pub fn ensure_pre_state(
    evm: &VM,
    test: &EFTest,
    report: &mut EFTestsReport,
) -> Result<(), Box<dyn Error>> {
    let world_state = &evm.db;
    for (address, pre_value) in &test.pre.0 {
        let account = world_state.get_account_info(*address);
        ensure_pre_state_condition(
            account.nonce == pre_value.nonce.as_u64(),
            format!(
                "Nonce mismatch for account {:#x}: expected {}, got {}",
                address, pre_value.nonce, account.nonce
            ),
            test,
            report,
        )?;
        ensure_pre_state_condition(
            account.balance == pre_value.balance,
            format!(
                "Balance mismatch for account {:#x}: expected {}, got {}",
                address, pre_value.balance, account.balance
            ),
            test,
            report,
        )?;
        for (k, v) in &pre_value.storage {
            let mut key_bytes = [0u8; 32];
            k.to_big_endian(&mut key_bytes);
            let storage_slot = world_state.get_storage_slot(*address, H256::from_slice(&key_bytes));
            ensure_pre_state_condition(
                &storage_slot == v,
                format!(
                    "Storage slot mismatch for account {:#x} at key {:?}: expected {}, got {}",
                    address, k, v, storage_slot
                ),
                test,
                report,
            )?;
        }
        ensure_pre_state_condition(
            keccak(account.bytecode.clone()) == keccak(pre_value.code.as_ref()),
            format!(
                "Code hash mismatch for account {:#x}: expected {}, got {}",
                address,
                keccak(pre_value.code.as_ref()),
                keccak(account.bytecode)
            ),
            test,
            report,
        )?;
    }
    Ok(())
}

fn ensure_pre_state_condition(
    condition: bool,
    error_reason: String,
    test: &EFTest,
    report: &mut EFTestsReport,
) -> Result<(), Box<dyn Error>> {
    if !condition {
        let error_reason = format!("Pre-state condition failed: {error_reason}");
        report.register_fail(&test.name, &error_reason);
        return Err(error_reason.into());
    }
    Ok(())
}

pub fn ensure_post_state(
    execution_result: Result<TransactionReport, VMError>,
    test: &EFTest,
    report: &mut EFTestsReport,
) -> Result<(), Box<dyn Error>> {
    match execution_result {
        Ok(execution_report) => {
            match test
                .post
                .clone()
                .values()
                .first()
                .map(|v| v.clone().expect_exception)
            {
                // Execution result was successful but an exception was expected.
                Some(Some(expected_exception)) => {
                    let error_reason = format!("Expected exception: {expected_exception}");
                    report.register_fail(&test.name, &error_reason);
                    return Err(format!("Post-state condition failed: {error_reason}").into());
                }
                // Execution result was successful and no exception was expected.
                // TODO: Check that the post-state matches the expected post-state.
                None | Some(None) => {
                    let pos_state_root = post_state_root(execution_report, test);
                    let expected_post_state_value = test.post.iter().next().cloned();
                    if let Some(expected_post_state_root_hash) = expected_post_state_value {
                        let expected_post_state_root_hash = expected_post_state_root_hash.hash;
                        if expected_post_state_root_hash != pos_state_root {
                            let error_reason = format!(
                                "Post-state root mismatch: expected {expected_post_state_root_hash:#x}, got {pos_state_root:#x}",
                            );
                            report.register_fail(&test.name, &error_reason);
                            return Err(
                                format!("Post-state condition failed: {error_reason}").into()
                            );
                        }
                    } else {
                        let error_reason = "No post-state root hash provided";
                        report.register_fail(&test.name, error_reason);
                        return Err(format!("Post-state condition failed: {error_reason}").into());
                    }
                }
            }
        }
        Err(err) => {
            match test
                .post
                .clone()
                .values()
                .first()
                .map(|v| v.clone().expect_exception)
            {
                // Execution result was unsuccessful and an exception was expected.
                // TODO: Check that the exception matches the expected exception.
                Some(Some(_expected_exception)) => {}
                // Execution result was unsuccessful but no exception was expected.
                None | Some(None) => {
                    let error_reason = format!("Unexpected exception: {err:?}");
                    report.register_fail(&test.name, &error_reason);
                    return Err(format!("Post-state condition failed: {error_reason}").into());
                }
            }
        }
    };
    report.register_pass(&test.name);
    Ok(())
}

pub fn post_state_root(execution_report: TransactionReport, test: &EFTest) -> H256 {
    let (initial_state, parent_hash) = utils::load_initial_state(test);

    let mut account_updates: Vec<AccountUpdate> = vec![];
    for (address, account) in execution_report.new_state {
        let mut added_storage = HashMap::new();

        for (key, value) in account.storage {
            added_storage.insert(key, value.current_value);
        }

        let code = if account.info.bytecode.is_empty() {
            None
        } else {
            Some(account.info.bytecode.clone())
        };

        let account_update = AccountUpdate {
            address,
            removed: false,
            info: Some(AccountInfo {
                code_hash: code_hash(&account.info.bytecode),
                balance: account.info.balance,
                nonce: account.info.nonce,
            }),
            code,
            added_storage,
        };

        account_updates.push(account_update);
    }

    initial_state
        .database()
        .unwrap()
        .apply_account_updates(parent_hash, &account_updates)
        .unwrap()
        .unwrap()
}
