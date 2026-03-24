use crate::{context::FunctionContext, convention::Convention, emitter::Emitter, macros::r64};
use iced_x86::code_asm::{qword_ptr, rsp};
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
                let cond64 = r64!(reg);
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
        self.spill_volatiles(ctx)?;

        let shadow = self.convention.shadow_space() as i32;
        let argument_registers = self.convention.argument_registers().len();

        for (i, &arg) in args.iter().enumerate() {
            if let Some(reg) = self.convention.argument_reg(i) {
                self.load_to_register(ctx, arg, reg)?;
            } else {
                let reg = self.scratch(ctx)?;
                self.load_to_register(ctx, arg, reg)?;
                let offset = shadow + ((i - argument_registers) as i32 * 8);
                self.asm.mov(qword_ptr(rsp + offset), r64!(reg))?;
            }
        }

        if let Some(&slot) = self.imports.get(&callee) {
            self.asm.call(qword_ptr(slot))?;
        } else {
            self.asm.call(self.functions[&callee])?;
        }

        ctx.registers.invalidate_volatiles(&self.convention);

        ctx.registers
            .track(self.convention.return_register(), Value::Symbol(dst));

        Ok(())
    }

    pub(crate) fn compile_ret(
        &mut self,
        ctx: &mut FunctionContext,
        val: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.load_to_register(ctx, val, self.convention.return_register())?;

        if ctx.cursor < ctx.instructions.len() - 1 {
            self.asm.jmp(ctx.epilogue)?;
        }

        Ok(())
    }
}
