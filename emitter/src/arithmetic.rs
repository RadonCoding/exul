use crate::{
    convention::Convention,
    emitter::{Emitter, FunctionContext},
    registers::ValueLocation,
};
use iced_x86::code_asm::{get_gpr64, qword_ptr, rbp};
use intermediate::{SymbolId, Value};
use std::error::Error;

impl<C: Convention> Emitter<C> {
    pub(crate) fn compile_add(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        let vctx = self.value_context(ctx);

        let left_loc = vctx.locate(left);
        let right_loc = vctx.locate(right);
        let dst_reg = ctx.allocs.get(&dst).copied().unwrap_or_else(|| self.ret());
        let dst64 = get_gpr64(dst_reg).unwrap();

        match (left_loc, right_loc) {
            (ValueLocation::Register(l), ValueLocation::Stack(off)) if l == dst_reg => {
                self.asm.add(dst64, qword_ptr(rbp - off))?;
            }
            (ValueLocation::Stack(off), ValueLocation::Register(r)) if r == dst_reg => {
                self.asm.add(dst64, qword_ptr(rbp - off))?;
            }
            _ => {
                self.load_to_register(ctx, left, dst_reg)?;

                match right_loc {
                    ValueLocation::Register(r) => {
                        self.asm.add(dst64, get_gpr64(r).unwrap())?;
                    }
                    ValueLocation::Stack(off) => {
                        self.asm.add(dst64, qword_ptr(rbp - off))?;
                    }
                    ValueLocation::Immediate(imm) => {
                        self.asm.add(dst64, imm as i32)?;
                    }
                }
            }
        }

        self.store_symbol(ctx, dst, dst_reg)?;

        Ok(())
    }

    pub(crate) fn compile_eq(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        let cmp_reg = self.vol();

        self.load_to_register(ctx, left, cmp_reg)?;
        let cmp64 = get_gpr64(cmp_reg).unwrap();

        let vctx = self.value_context(ctx);
        let right_loc = vctx.locate(right);

        match right_loc {
            ValueLocation::Register(r) => {
                let r64 = get_gpr64(r).unwrap();
                self.asm.cmp(cmp64, r64)?;
            }
            ValueLocation::Stack(offset) => {
                self.asm.cmp(cmp64, qword_ptr(rbp - offset))?;
            }
            ValueLocation::Immediate(imm) => {
                self.asm.cmp(cmp64, imm as i32)?;
            }
        }

        let res8 = self.to_reg8(cmp_reg);
        self.asm.sete(res8)?;
        self.asm.movzx(cmp64, res8)?;

        self.store_symbol(ctx, dst, cmp_reg)?;

        Ok(())
    }
}
