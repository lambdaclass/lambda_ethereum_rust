use crate::{db::Database, env::Env, Evm};

#[derive(Default)]
pub struct EvmBuilder<DB: Database> {
    db: DB,
    env: Env,
}

impl<DB: Database + Default> EvmBuilder<DB> {
    /// Sets the [`Database`] that will be used by [`Evm`].
    pub fn with_db(self, db: DB) -> EvmBuilder<DB> {
        EvmBuilder { db, env: self.env }
    }

    pub fn build(self) -> Evm<DB> {
        Evm::new(self.env, self.db)
    }
}
