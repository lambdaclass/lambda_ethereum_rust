use ::ef_tests::{evm::execute_transaction, types::TestUnit};

fn execute_test(test: TestUnit) {
    // TODO: Add support for multiple blocks and multiple transactions per block.
    let transaction = test
        .blocks
        .first()
        .unwrap()
        .transactions
        .as_ref()
        .unwrap()
        .first()
        .unwrap();
    execute_transaction(&test.genesis_block_header, transaction, test.pre);
}

#[cfg(test)]
mod ef_tests {
    use std::collections::HashMap;

    use ef_tests::types::TestUnit;

    use crate::execute_test;

    #[test]
    fn add11_test() {
        let s: String =
            std::fs::read_to_string("./vectors/add11.json").expect("Unable to read file");
        let tests: HashMap<String, TestUnit> =
            serde_json::from_str(&s).expect("Unable to parse JSON");

        for (_k, test) in tests {
            execute_test(test)
        }
    }
}
