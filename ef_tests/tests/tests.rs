#[cfg(test)]
mod ef_tests {
    use std::collections::HashMap;

    use ef_tests::{evm::execute_transaction, types::TestUnit};

    #[test]
    fn add11_test() {
        let s: String = std::fs::read_to_string("./add11.json").unwrap();
        let v: HashMap<String, TestUnit> = serde_json::from_str(&s).unwrap();

        for (_key, test) in v.iter() {
            let genesis = &test.genesis_block_header;

            let transaction = test
                .blocks
                .first()
                .unwrap()
                .transactions
                .as_ref()
                .unwrap()
                .first()
                .unwrap();
            execute_transaction(genesis, transaction, test.pre.clone());
        }
    }
}
