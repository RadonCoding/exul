use crate::{
    context::FunctionContext,
    convention::Convention,
    emitter::Emitter,
    macros::{r8, r16, r32, r64},
    registers::Operand,
};
use iced_x86::code_asm::{byte_ptr, dword_ptr, ptr, qword_ptr, rbp, word_ptr};
use intermediate::{Memory, Segment, SymbolId, Value};
use std::error::Error;

impl<C: Convention> Emitter<C> {
    pub(crate) fn compile_load(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        size: Memory,
        src: Value,
    ) -> Result<(), Box<dyn Error>> {
        let dreg = self.scratch(ctx)?;

        // Resolve the address directly to avoid clobbering live values with an intermediate move.
        let src_loc = ctx.registers.locate(src, &ctx.slots, &self.data_labels);
        let src64 = match src_loc {
            Operand::Register(r) => r64!(r),
            Operand::Stack(offset) => {
                let tmp_reg = self.scratch(ctx)?;
                self.asm.mov(r64!(tmp_reg), qword_ptr(rbp - offset))?;
                ctx.registers.track(tmp_reg, src);
                r64!(tmp_reg)
            }
            _ => unreachable!(),
        };

        match size {
            Memory::Byte => self.asm.movzx(r64!(dreg), byte_ptr(src64))?,
            Memory::Word => self.asm.movzx(r64!(dreg), word_ptr(src64))?,
            Memory::Dword => self.asm.mov(r32!(dreg), dword_ptr(src64))?,
            Memory::Qword => self.asm.mov(r64!(dreg), qword_ptr(src64))?,
        }

        self.spill_and_track(ctx, dst, dreg)?;

        Ok(())
    }

    pub(crate) fn compile_store(
        &mut self,
        ctx: &mut FunctionContext,
        size: Memory,
        dst: Value,
        src: Value,
    ) -> Result<(), Box<dyn Error>> {
        let dreg = self.scratch(ctx)?;
        self.load_to_register(ctx, dst, dreg)?;

        let sreg = self.scratch(ctx)?;
        self.load_to_register(ctx, src, sreg)?;

        let dst64 = r64!(dreg);

        match size {
            Memory::Byte => self.asm.mov(byte_ptr(dst64), r8!(sreg))?,
            Memory::Word => self.asm.mov(word_ptr(dst64), r16!(sreg))?,
            Memory::Dword => self.asm.mov(dword_ptr(dst64), r32!(sreg))?,
            Memory::Qword => self.asm.mov(qword_ptr(dst64), r64!(sreg))?,
        }

        Ok(())
    }

    pub(crate) fn compile_segment(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        segment: Segment,
        offset: Value,
    ) -> Result<(), Box<dyn Error>> {
        let dreg = self.scratch(ctx)?;

        match (segment, offset) {
            (Segment::Gs, Value::Constant(imm)) => {
                self.asm.mov(r64!(dreg), ptr(imm as u32).gs())?;
            }
            (Segment::Fs, Value::Constant(imm)) => {
                self.asm.mov(r64!(dreg), ptr(imm as u32).fs())?;
            }
            (Segment::Gs, _) => {
                let reg = self.scratch(ctx)?;
                self.load_to_register(ctx, offset, reg)?;
                self.asm.mov(r64!(dreg), ptr(r64!(reg)).gs())?;
            }
            (Segment::Fs, _) => {
                let reg = self.scratch(ctx)?;
                self.load_to_register(ctx, offset, reg)?;
                self.asm.mov(r64!(dreg), ptr(r64!(reg)).fs())?;
            }
        }

        self.spill_and_track(ctx, dst, dreg)?;

        Ok(())
    }
}
