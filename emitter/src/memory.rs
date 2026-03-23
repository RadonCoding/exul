use crate::{
    convention::Convention,
    registers::Operand,
    {Emitter, FunctionContext},
};
use iced_x86::code_asm::{byte_ptr, dword_ptr, get_gpr64, ptr, qword_ptr, rbp, word_ptr};
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
        let dst_reg = self.scratch(ctx)?;
        let dst64 = get_gpr64(dst_reg).unwrap();

        // Resolve the address directly to avoid clobbering live values with an intermediate move.
        let src_loc = ctx.registers.locate(src, &ctx.slots, &self.data_labels);
        let src64 = match src_loc {
            Operand::Register(r) => get_gpr64(r).unwrap(),
            Operand::Stack(offset) => {
                let tmp_reg = self.scratch(ctx)?;
                let tmp64 = get_gpr64(tmp_reg).unwrap();
                self.asm.mov(tmp64, qword_ptr(rbp - offset))?;
                ctx.registers.track(tmp_reg, src);
                tmp64
            }
            _ => unreachable!(),
        };

        match size {
            Memory::Byte => self.asm.movzx(dst64, byte_ptr(src64))?,
            Memory::Word => self.asm.movzx(dst64, word_ptr(src64))?,
            Memory::Dword => self.asm.mov(self.to_reg32(dst_reg), dword_ptr(src64))?,
            Memory::Qword => self.asm.mov(dst64, qword_ptr(src64))?,
        }

        self.spill(ctx, dst, dst_reg)?;

        Ok(())
    }

    pub(crate) fn compile_store(
        &mut self,
        ctx: &mut FunctionContext,
        size: Memory,
        dst: Value,
        src: Value,
    ) -> Result<(), Box<dyn Error>> {
        let dst_reg = self.scratch(ctx)?;
        self.load_to_register(ctx, dst, dst_reg)?;

        let src_reg = self.scratch(ctx)?;
        self.load_to_register(ctx, src, src_reg)?;

        let dst64 = get_gpr64(dst_reg).unwrap();

        match size {
            Memory::Byte => self.asm.mov(byte_ptr(dst64), self.to_reg8(src_reg))?,
            Memory::Word => self.asm.mov(word_ptr(dst64), self.to_reg16(src_reg))?,
            Memory::Dword => self.asm.mov(dword_ptr(dst64), self.to_reg32(src_reg))?,
            Memory::Qword => self
                .asm
                .mov(qword_ptr(dst64), get_gpr64(src_reg).unwrap())?,
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
        let dst_reg = self.scratch(ctx)?;
        let dst64 = get_gpr64(dst_reg).unwrap();

        match (segment, offset) {
            (Segment::Gs, Value::Constant(imm)) => {
                self.asm.mov(dst64, ptr(imm as u32).gs())?;
            }
            (Segment::Fs, Value::Constant(imm)) => {
                self.asm.mov(dst64, ptr(imm as u32).fs())?;
            }
            (Segment::Gs, _) => {
                let reg = self.scratch(ctx)?;
                self.load_to_register(ctx, offset, reg)?;
                self.asm.mov(dst64, ptr(get_gpr64(reg).unwrap()).gs())?;
            }
            (Segment::Fs, _) => {
                let reg = self.scratch(ctx)?;
                self.load_to_register(ctx, offset, reg)?;
                self.asm.mov(dst64, ptr(get_gpr64(reg).unwrap()).fs())?;
            }
        }

        self.spill(ctx, dst, dst_reg)?;

        Ok(())
    }
}
