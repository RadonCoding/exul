use std::error::Error;

use ir::{InstructionKind, Value};

use crate::{convention::Convention, emitter::Emitter};

impl<C: Convention> Emitter<C> {
    pub(crate) fn compile_add(&mut self, kind: InstructionKind) -> Result<(), Box<dyn Error>> {
        if let InstructionKind::Add { dst, left, right } = kind {
            self.mov_val(dst, left)?;
            let d = self.reg(dst);
            match right {
                Value::Symbol(s) => self.asm.add(d, self.reg(s))?,
                Value::Constant(c) => self.asm.add(d, c as i32)?,
            }
        }
        Ok(())
    }

    pub(crate) fn compile_eq(&mut self, kind: InstructionKind) -> Result<(), Box<dyn Error>> {
        if let InstructionKind::Eq { dst, left, right } = kind {
            self.mov_val(dst, left)?;
            let d = self.reg(dst);
            match right {
                Value::Symbol(s) => self.asm.cmp(d, self.reg(s))?,
                Value::Constant(c) => self.asm.cmp(d, c as i32)?,
            }
            let byte_reg = self.low8(dst);
            self.asm.sete(byte_reg)?;
            self.asm.movzx(d, byte_reg)?;
        }
        Ok(())
    }
}
