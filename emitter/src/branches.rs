use crate::{
    convention::Convention,
    emitter::{Emitter, FunctionContext},
    registers::ValueLocation,
};
use iced_x86::code_asm::{get_gpr64, qword_ptr, rbp, rsp};
use intermediate::{InstructionKind, SymbolId, Value};
use std::error::Error;

impl<C: Convention> Emitter<C> {
    pub(crate) fn compile_branch(
        &mut self,
        ctx: &mut FunctionContext,
        kind: InstructionKind,
    ) -> Result<(), Box<dyn Error>> {
        match kind {
            InstructionKind::JumpIfFalse { cond, dst } => {
                let cond_reg = self.ensure_in_register(ctx, cond, self.vol())?;
                let cond64 = get_gpr64(cond_reg).unwrap();

                self.asm.test(cond64, cond64)?;
                let l = self.get_label(ctx, dst);
                self.asm.jz(l)?;
            }
            InstructionKind::JumpIfNotEq { left, right, dst } => {
                let left_reg = self.ensure_in_register(ctx, left, self.vol())?;
                let left64 = get_gpr64(left_reg).unwrap();

                let vctx = self.value_context(ctx);
                let right_loc = vctx.locate(right);

                match right_loc {
                    ValueLocation::Register(r) => {
                        let r64 = get_gpr64(r).unwrap();
                        self.asm.cmp(left64, r64)?;
                    }
                    ValueLocation::Stack(offset) => {
                        self.asm.cmp(left64, qword_ptr(rbp - offset))?;
                    }
                    ValueLocation::Immediate(imm) => {
                        self.asm.cmp(left64, imm as i32)?;
                    }
                }

                let l = self.get_label(ctx, dst);
                self.asm.jne(l)?;
            }
            InstructionKind::Jump(id) => {
                let l = self.get_label(ctx, id);
                self.asm.jmp(l)?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    pub(crate) fn compile_call(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        callee: SymbolId,
        args: Vec<Value>,
    ) -> Result<(), Box<dyn Error>> {
        ctx.registers.invalidate_symbol(dst);

        let mut stacked = 0;

        for (i, &arg) in args.iter().enumerate() {
            if let Some(reg) = self.convention.argument_reg(i) {
                self.load_to_register(ctx, arg, reg)?;
            } else {
                let temp_reg = self.vol();
                self.load_to_register(ctx, arg, temp_reg)?;
                let temp64 = get_gpr64(temp_reg).unwrap();
                self.asm.push(temp64)?;
                stacked += 1;
            }
        }

        self.asm.call(self.functions[&callee])?;

        if stacked > 0 {
            self.asm.add(rsp, (stacked * 8) as i32)?;
        }

        ctx.registers.invalidate_volatile(&self.convention);

        self.store_symbol(ctx, dst, self.ret())?;

        Ok(())
    }

    pub(crate) fn compile_ret(
        &mut self,
        ctx: &mut FunctionContext,
        val: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.load_to_register(ctx, val, self.ret())?;

        if ctx.cursor < ctx.instructions.len() - 1 {
            self.asm.jmp(ctx.epilogue)?;
        }

        Ok(())
    }
}
