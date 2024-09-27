use std::fmt::Debug;

use melior::{ir::Module as MeliorModule, Context as MeliorContext};

pub struct MLIRModule<'m> {
    pub(crate) melior_module: MeliorModule<'m>,
}

impl<'m> MLIRModule<'m> {
    pub fn new(module: MeliorModule<'m>) -> Self {
        Self {
            melior_module: module,
        }
    }

    pub fn module(&self) -> &MeliorModule {
        &self.melior_module
    }

    pub fn parse(context: &MeliorContext, source: &str) -> Option<Self> {
        MeliorModule::parse(context, source).map(Self::new)
    }
}

impl Debug for MLIRModule<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.melior_module.as_operation().to_string())
    }
}
