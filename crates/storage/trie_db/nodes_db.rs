use crate::{error::StoreError, rlp::Rlp};
use libmdbx::{
    orm::{table, Database},
    table_info,
};

use super::node::{Node, NodeHash};
pub struct NodesDB(Database);

pub type NodeHashRLP = Rlp<NodeHash>;
pub type NodeRLP = Rlp<Node>;

table!(
    /// Node Hash to Nodes table
    ( Nodes ) NodeHashRLP => NodeRLP
);

impl NodesDB {
    pub fn init(trie_dir: &str) -> Result<NodesDB, StoreError> {
        let tables = [table_info!(Nodes)].into_iter().collect();
        let path = [trie_dir, "/nodes"].concat().try_into().ok();
        Ok(NodesDB(
            Database::create(path, &tables).map_err(StoreError::LibmdbxError)?,
        ))
    }

    pub fn get(&self, node_hash: NodeHash) -> Result<Option<Node>, StoreError> {
        let txn = self.0.begin_read().map_err(StoreError::LibmdbxError)?;
        Ok(txn
            .get::<Nodes>(node_hash.into())
            .map_err(StoreError::LibmdbxError)?
            .map(|n| n.to()))
    }

    pub fn insert(&self, node_hash: NodeHash, node: Node) -> Result<(), StoreError> {
        let txn = self.0.begin_readwrite().map_err(StoreError::LibmdbxError)?;
        txn.upsert::<Nodes>(node_hash.into(), node.into())
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }

    pub fn remove(&self, node_hash: NodeHash) -> Result<(), StoreError> {
        let txn = self.0.begin_readwrite().map_err(StoreError::LibmdbxError)?;
        txn.delete::<Nodes>(node_hash.into(), None)
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }
}
