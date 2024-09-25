use std::collections::HashSet;

use bytes::Bytes;

use crate::operations::Operation;

pub struct Program {
    pub operations: Vec<Operation>,
    pub jumptable: HashSet<usize>,
}

impl Program {
    pub fn from_operations(operations: Vec<Operation>) -> Self {
        Self {
            jumptable: Self::populate_jumptable(&operations),
            operations,
        }
    }

    pub fn to_bytecode(&self) -> Bytes {
        self.operations
            .iter()
            .flat_map(Operation::to_bytecode)
            .collect::<Bytes>()
    }

    fn populate_jumptable(operations: &[Operation]) -> HashSet<usize> {
        let mut jumptable = HashSet::new();
        let mut counter = 0;
        for operation in operations.iter() {
            match operation {
                Operation::Jumpdest => {
                    counter += 1;
                    jumptable.insert(counter);
                }
                Operation::Push32(_) => {
                    counter += 32;
                }
                _ => {
                    counter += 1;
                }
            }
        }

        jumptable
    }
}
