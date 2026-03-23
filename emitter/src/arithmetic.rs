use crate::{
    convention::Convention,
    registers::Operand,
    {Emitter, FunctionContext},
};
use iced_x86::{
    Register,
    code_asm::{cl, get_gpr64, qword_ptr, rbp},
};
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

    pub(crate) fn compile_and(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.emit_binary(ctx, dst, left, right, |s, dst64, right_loc| {
            match right_loc {
                Operand::Register(r) => s.asm.and(dst64, get_gpr64(r).unwrap())?,
                Operand::Stack(offset) => s.asm.and(dst64, qword_ptr(rbp - offset))?,
                Operand::Immediate(imm) => s.asm.and(dst64, imm as i32)?,
                _ => unreachable!(),
            }
            Ok(())
        })
    }

    pub(crate) fn compile_or(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.emit_binary(ctx, dst, left, right, |s, dst64, right_loc| {
            match right_loc {
                Operand::Register(r) => s.asm.or(dst64, get_gpr64(r).unwrap())?,
                Operand::Stack(offset) => s.asm.or(dst64, qword_ptr(rbp - offset))?,
                Operand::Immediate(imm) => s.asm.or(dst64, imm as i32)?,
                _ => unreachable!(),
            }
            Ok(())
        })
    }

    pub(crate) fn compile_xor(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        self.emit_binary(ctx, dst, left, right, |s, dst64, right_loc| {
            match right_loc {
                Operand::Register(r) => s.asm.xor(dst64, get_gpr64(r).unwrap())?,
                Operand::Stack(offset) => s.asm.xor(dst64, qword_ptr(rbp - offset))?,
                Operand::Immediate(imm) => s.asm.xor(dst64, imm as i32)?,
                _ => unreachable!(),
            }
            Ok(())
        })
    }

    pub(crate) fn compile_shl(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        let dst_reg = self.scratch(ctx)?;
        let dst64 = get_gpr64(dst_reg).unwrap();
        let right_loc = self.binary_operands(ctx, left, right, dst_reg)?;

        match right_loc {
            Operand::Immediate(imm) => self.asm.shl(dst64, imm as u32)?,
            Operand::Register(r) => {
                ctx.registers.invalidate_register(Register::RCX);
                self.asm
                    .mov(get_gpr64(Register::RCX).unwrap(), get_gpr64(r).unwrap())?;
                self.asm.shl(dst64, cl)?;
            }
            Operand::Stack(offset) => {
                ctx.registers.invalidate_register(Register::RCX);
                self.asm
                    .mov(get_gpr64(Register::RCX).unwrap(), qword_ptr(rbp - offset))?;
                self.asm.shl(dst64, cl)?;
            }
            _ => unreachable!(),
        }

        self.spill(ctx, dst, dst_reg)?;
        Ok(())
    }

    pub(crate) fn compile_shr(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        let dst_reg = self.scratch(ctx)?;
        let dst64 = get_gpr64(dst_reg).unwrap();
        let right_loc = self.binary_operands(ctx, left, right, dst_reg)?;

        match right_loc {
            Operand::Immediate(imm) => self.asm.shr(dst64, imm as u32)?,
            Operand::Register(r) => {
                ctx.registers.invalidate_register(Register::RCX);
                self.asm
                    .mov(get_gpr64(Register::RCX).unwrap(), get_gpr64(r).unwrap())?;
                self.asm.shr(dst64, cl)?;
            }
            Operand::Stack(offset) => {
                ctx.registers.invalidate_register(Register::RCX);
                self.asm
                    .mov(get_gpr64(Register::RCX).unwrap(), qword_ptr(rbp - offset))?;
                self.asm.shr(dst64, cl)?;
            }
            _ => unreachable!(),
        }

        self.spill(ctx, dst, dst_reg)?;
        Ok(())
    }
}
