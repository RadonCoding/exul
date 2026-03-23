use crate::{
    convention::Convention,
    {Emitter, FunctionContext},
};
use iced_x86::code_asm::{get_gpr64, qword_ptr, rsp};
use intermediate::{FunctionId, InstructionKind, SymbolId, Value};
use std::error::Error;

impl<C: Convention> Emitter<C> {
    pub(crate) fn compile_branch(
        &mut self,
        ctx: &mut FunctionContext,
        kind: InstructionKind,
    ) -> Result<(), Box<dyn Error>> {
        match kind {
            InstructionKind::JumpIfFalse { cond, dst } => {
                let reg = self.scratch(ctx)?;
                self.load_to_register(ctx, cond, reg)?;
                let cond64 = get_gpr64(reg).unwrap();
                self.asm.test(cond64, cond64)?;
                let l = self.get_label(ctx, dst);
                self.asm.jz(l)?;
            }
            InstructionKind::JumpIfEq { left, right, dst } => {
                self.emit_compare_flags(ctx, left, right)?;
                let l = self.get_label(ctx, dst);
                self.asm.je(l)?;
            }
            InstructionKind::JumpIfNotEq { left, right, dst } => {
                self.emit_compare_flags(ctx, left, right)?;
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
        callee: FunctionId,
        args: Vec<Value>,
    ) -> Result<(), Box<dyn Error>> {
        ctx.registers.invalidate_symbol(dst);

        let shadow = self.convention.shadow_space() as i32;
        let argument_registers = self.convention.argument_registers().len();

        for (i, &arg) in args.iter().enumerate() {
            if let Some(reg) = self.convention.argument_reg(i) {
                self.load_to_register(ctx, arg, reg)?;
            } else {
                let reg = self.scratch(ctx)?;
                self.load_to_register(ctx, arg, reg)?;
                let offset = shadow + ((i - argument_registers) as i32 * 8);
                self.asm
                    .mov(qword_ptr(rsp + offset), get_gpr64(reg).unwrap())?;
            }
        }

        if let Some(&slot) = self.imports.get(&callee) {
            self.asm.call(qword_ptr(slot))?;
        } else {
            self.asm.call(self.functions[&callee])?;
        }

        ctx.registers.invalidate_volatiles(&self.convention);

        if self.is_live(ctx, dst) {
            self.spill(ctx, dst, self.ret())?;
        } else {
            ctx.registers.track(self.ret(), Value::Symbol(dst));
        }

        Ok(())
    }

    pub(crate) fn compile_ret(
        &mut self,
        ctx: &mut FunctionContext,
        val: Value,
    ) -> Result<(), Box<dyn Error>> {
        let ret = self.ret();
        self.load_to_register(ctx, val, ret)?;

        if ctx.cursor < ctx.instructions.len() - 1 {
            self.asm.jmp(ctx.epilogue)?;
        }

        Ok(())
    }
}
