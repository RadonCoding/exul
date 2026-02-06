use std::error::Error;

use intermediate::{InstructionKind, Value};

use crate::{
    convention::Convention,
    emitter::{Emitter, FunctionContext},
};

impl<C: Convention> Emitter<C> {
    pub(crate) fn compile_add(
        &mut self,
        ctx: &FunctionContext,
        kind: InstructionKind,
    ) -> Result<(), Box<dyn Error>> {
        if let InstructionKind::Add { dst, left, right } = kind {
            self.mov_val(ctx, dst, left)?;

            let d = self.reg(ctx, dst);

            match right {
                Value::Symbol(s) => self.asm.add(d, self.reg(ctx, s))?,
                Value::Constant(c) => self.asm.add(d, c as i32)?,
            }
        }
        Ok(())
    }

    pub(crate) fn compile_eq(
        &mut self,
        ctx: &FunctionContext,
        kind: InstructionKind,
    ) -> Result<(), Box<dyn Error>> {
        if let InstructionKind::Eq { dst, left, right } = kind {
            self.mov_val(ctx, dst, left)?;

            let d = self.reg(ctx, dst);

            match right {
                Value::Symbol(s) => self.asm.cmp(d, self.reg(ctx, s))?,
                Value::Constant(c) => self.asm.cmp(d, c as i32)?,
            }

            let d8 = self.reg8(ctx, dst);
            self.asm.sete(d8)?;
            self.asm.movzx(d, d8)?;
        }
        Ok(())
    }
}
