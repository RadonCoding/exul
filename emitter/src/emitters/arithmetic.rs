use crate::{
    context::FunctionContext,
    convention::Convention,
    emitter::Emitter,
    macros::{r8, r64},
    registers::Operand,
};
use iced_x86::{
    Register,
    code_asm::{cl, qword_ptr, rbp},
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
        self.emit_compare_value(ctx, dst, left, right, |s, reg| {
            s.asm.sete(r8!(reg))?;
            s.asm.movzx(r64!(reg), r8!(reg))?;
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
        self.emit_compare_value(ctx, dst, left, right, |s, reg| {
            s.asm.setne(r8!(reg))?;
            s.asm.movzx(r64!(reg), r8!(reg))?;
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
        self.emit_compare_value(ctx, dst, left, right, |s, reg| {
            s.asm.setle(r8!(reg))?;
            s.asm.movzx(r64!(reg), r8!(reg))?;
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
        self.emit_compare_value(ctx, dst, left, right, |s, reg| {
            s.asm.setge(r8!(reg))?;
            s.asm.movzx(r64!(reg), r8!(reg))?;
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
        self.emit_compare_value(ctx, dst, left, right, |s, reg| {
            s.asm.setl(r8!(reg))?;
            s.asm.movzx(r64!(reg), r8!(reg))?;
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
        self.emit_compare_value(ctx, dst, left, right, |s, reg| {
            s.asm.setg(r8!(reg))?;
            s.asm.movzx(r64!(reg), r8!(reg))?;
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
        self.emit_binary(ctx, dst, left, right, |s, dreg, rloc| {
            match rloc {
                Operand::Register(r) => s.asm.add(r64!(dreg), r64!(r))?,
                Operand::Stack(offset) => s.asm.add(r64!(dreg), qword_ptr(rbp - offset))?,
                Operand::Immediate(imm) => s.asm.add(r64!(dreg), imm as i32)?,
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
        self.emit_binary(ctx, dst, left, right, |s, dreg, rloc| {
            match rloc {
                Operand::Register(r) => s.asm.sub(r64!(dreg), r64!(r))?,
                Operand::Stack(offset) => s.asm.sub(r64!(dreg), qword_ptr(rbp - offset))?,
                Operand::Immediate(imm) => s.asm.sub(r64!(dreg), imm as i32)?,
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
        self.emit_binary(ctx, dst, left, right, |s, dreg, rloc| {
            match rloc {
                Operand::Register(r) => s.asm.imul_2(r64!(dreg), r64!(r))?,
                Operand::Stack(offset) => s.asm.imul_2(r64!(dreg), qword_ptr(rbp - offset))?,
                Operand::Immediate(imm) => s.asm.imul_3(r64!(dreg), r64!(dreg), imm as i32)?,
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
        self.emit_binary(ctx, dst, left, right, |s, dreg, rloc| {
            match rloc {
                Operand::Register(r) => s.asm.and(r64!(dreg), r64!(r))?,
                Operand::Stack(offset) => s.asm.and(r64!(dreg), qword_ptr(rbp - offset))?,
                Operand::Immediate(imm) => s.asm.and(r64!(dreg), imm as i32)?,
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
        self.emit_binary(ctx, dst, left, right, |s, dreg, rloc| {
            match rloc {
                Operand::Register(r) => s.asm.or(r64!(dreg), r64!(r))?,
                Operand::Stack(offset) => s.asm.or(r64!(dreg), qword_ptr(rbp - offset))?,
                Operand::Immediate(imm) => s.asm.or(r64!(dreg), imm as i32)?,
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
        self.emit_binary(ctx, dst, left, right, |s, dreg, rloc| {
            match rloc {
                Operand::Register(r) => s.asm.xor(r64!(dreg), r64!(r))?,
                Operand::Stack(offset) => s.asm.xor(r64!(dreg), qword_ptr(rbp - offset))?,
                Operand::Immediate(imm) => s.asm.xor(r64!(dreg), imm as i32)?,
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
        let dreg = self.scratch(ctx)?;
        let rloc = self.binary_operands(ctx, left, right, dreg)?;

        match rloc {
            Operand::Immediate(imm) => self.asm.shl(r64!(dreg), imm as u32)?,
            Operand::Register(r) => {
                ctx.registers.invalidate_register(Register::RCX);
                self.asm.mov(r64!(Register::RCX), r64!(r))?;
                self.asm.shl(r64!(dreg), cl)?;
            }
            Operand::Stack(offset) => {
                ctx.registers.invalidate_register(Register::RCX);
                self.asm.mov(r64!(Register::RCX), qword_ptr(rbp - offset))?;
                self.asm.shl(r64!(dreg), cl)?;
            }
            _ => unreachable!(),
        }

        self.spill_and_track(ctx, dst, dreg)?;
        Ok(())
    }

    pub(crate) fn compile_shr(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        let dreg = self.scratch(ctx)?;
        let rloc = self.binary_operands(ctx, left, right, dreg)?;

        match rloc {
            Operand::Immediate(imm) => self.asm.shr(r64!(dreg), imm as u32)?,
            Operand::Register(r) => {
                ctx.registers.invalidate_register(Register::RCX);
                self.asm.mov(r64!(Register::RCX), r64!(r))?;
                self.asm.shr(r64!(dreg), cl)?;
            }
            Operand::Stack(offset) => {
                ctx.registers.invalidate_register(Register::RCX);
                self.asm.mov(r64!(Register::RCX), qword_ptr(rbp - offset))?;
                self.asm.shr(r64!(dreg), cl)?;
            }
            _ => unreachable!(),
        }

        self.spill_and_track(ctx, dst, dreg)?;

        Ok(())
    }
}
