use melior::{
    dialect::llvm::{self, attributes::Linkage},
    ir::{
        attribute::{FlatSymbolRefAttribute, StringAttribute, TypeAttribute},
        operation::OperationBuilder,
        Identifier, Location, Region,
    },
    Context as MeliorContext,
};

pub fn global<'c>(
    context: &'c MeliorContext,
    name: &str,
    global_type: melior::ir::Type<'c>,
    linkage: Linkage,
    location: Location<'c>,
) -> melior::ir::Operation<'c> {
    // TODO: use ODS
    OperationBuilder::new("llvm.mlir.global", location)
        .add_regions([Region::new()])
        .add_attributes(&[
            (
                Identifier::new(context, "sym_name"),
                StringAttribute::new(context, name).into(),
            ),
            (
                Identifier::new(context, "global_type"),
                TypeAttribute::new(global_type).into(),
            ),
            (
                Identifier::new(context, "linkage"),
                llvm::attributes::linkage(context, linkage),
            ),
        ])
        .build()
        .expect("valid operation")
}

pub fn addressof<'c>(
    context: &'c MeliorContext,
    name: &str,
    result_type: melior::ir::Type<'c>,
    location: Location<'c>,
) -> melior::ir::Operation<'c> {
    // TODO: use ODS
    OperationBuilder::new("llvm.mlir.addressof", location)
        .add_attributes(&[(
            Identifier::new(context, "global_name"),
            FlatSymbolRefAttribute::new(context, name).into(),
        )])
        .add_results(&[result_type])
        .build()
        .expect("valid operation")
}
