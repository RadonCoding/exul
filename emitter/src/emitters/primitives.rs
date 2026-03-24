use crate::{context::FunctionContext, convention::Convention, emitter::Emitter, macros::r64};
use iced_x86::code_asm::ptr;
use intermediate::{FunctionId, SymbolId, Value};
use std::error::Error;

impl<C: Convention> Emitter<C> {
    pub(crate) fn compile_assign(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        src: Value,
    ) -> Result<(), Box<dyn Error>> {
        let reg = self.scratch(ctx)?;
        self.load_to_register(ctx, src, reg)?;
        self.spill_and_track(ctx, dst, reg)?;
        Ok(())
    }

    pub(crate) fn compile_import(
        &mut self,
        ctx: &mut FunctionContext,
        import: FunctionId,
        src: Value,
    ) -> Result<(), Box<dyn Error>> {
        let reg = self.scratch(ctx)?;
        self.load_to_register(ctx, src, reg)?;
        let slot = self.imports[&import];
        self.asm.mov(ptr(slot), r64!(reg))?;
        Ok(())
    }
}
