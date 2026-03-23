use crate::{
    convention::Convention,
    registers::Operand,
    {Emitter, FunctionContext},
};
use iced_x86::code_asm::{get_gpr64, qword_ptr, rbp};
use intermediate::{SymbolId, Value};
use std::error::Error;

impl<C: Convention> Emitter<C> {
    pub(crate) fn compile_eq(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.emit_compare_value(ctx, dst, left, right, |s, cmp64| {
            let res8 = s.to_reg8(cmp64.into());
            s.asm.sete(res8)?;
            s.asm.movzx(cmp64, res8)?;
            Ok(())
        })
    }

    pub(crate) fn compile_not_eq(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.emit_compare_value(ctx, dst, left, right, |s, cmp64| {
            let res8 = s.to_reg8(cmp64.into());
            s.asm.setne(res8)?;
            s.asm.movzx(cmp64, res8)?;
            Ok(())
        })
    }

    pub(crate) fn compile_lte(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.emit_compare_value(ctx, dst, left, right, |s: &mut Emitter<C>, cmp64| {
            let res8 = s.to_reg8(cmp64.into());
            s.asm.setle(res8)?;
            s.asm.movzx(cmp64, res8)?;
            Ok(())
        })
    }

    pub(crate) fn compile_gte(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.emit_compare_value(ctx, dst, left, right, |s, cmp64| {
            let res8 = s.to_reg8(cmp64.into());
            s.asm.setge(res8)?;
            s.asm.movzx(cmp64, res8)?;
            Ok(())
        })
    }

    pub(crate) fn compile_lt(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.emit_compare_value(ctx, dst, left, right, |s, cmp64| {
            let res8 = s.to_reg8(cmp64.into());
            s.asm.setl(res8)?;
            s.asm.movzx(cmp64, res8)?;
            Ok(())
        })
    }

    pub(crate) fn compile_gt(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.emit_compare_value(ctx, dst, left, right, |s, cmp64| {
            let res8 = s.to_reg8(cmp64.into());
            s.asm.setg(res8)?;
            s.asm.movzx(cmp64, res8)?;
            Ok(())
        })
    }

    pub(crate) fn compile_add(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.emit_binary(ctx, dst, left, right, |s, dst64, right_loc| {
            match right_loc {
                Operand::Register(r) => s.asm.add(dst64, get_gpr64(r).unwrap())?,
                Operand::Stack(offset) => s.asm.add(dst64, qword_ptr(rbp - offset))?,
                Operand::Immediate(imm) => s.asm.add(dst64, imm as i32)?,
                _ => unreachable!(),
            }
            Ok(())
        })
    }

    pub(crate) fn compile_sub(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.emit_binary(ctx, dst, left, right, |s, dst64, right_loc| {
            match right_loc {
                Operand::Register(r) => s.asm.sub(dst64, get_gpr64(r).unwrap())?,
                Operand::Stack(offset) => s.asm.sub(dst64, qword_ptr(rbp - offset))?,
                Operand::Immediate(imm) => s.asm.sub(dst64, imm as i32)?,
                _ => unreachable!(),
            }
            Ok(())
        })
    }

    pub(crate) fn compile_mul(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.emit_binary(ctx, dst, left, right, |s, dst64, right_loc| {
            match right_loc {
                Operand::Register(r) => s.asm.imul_2(dst64, get_gpr64(r).unwrap())?,
                Operand::Stack(offset) => s.asm.imul_2(dst64, qword_ptr(rbp - offset))?,
                Operand::Immediate(imm) => s.asm.imul_3(dst64, dst64, imm as i32)?,
                _ => unreachable!(),
            }
            Ok(())
        })
    }
}
