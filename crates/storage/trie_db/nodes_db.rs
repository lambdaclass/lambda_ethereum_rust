use libmdbx::orm::Database;

use crate::error::StoreError;

pub struct NodesDB(Database);

impl NodesDB {
    pub fn init(trie_dir: &str) -> Result<NodesDB, StoreError> {
        let tables = [].into_iter().collect();
        let path = [trie_dir, "/nodes"].concat().try_into().ok();
        Ok(NodesDB(
            Database::create(path, &tables).map_err(StoreError::LibmdbxError)?,
        ))
    }
}
