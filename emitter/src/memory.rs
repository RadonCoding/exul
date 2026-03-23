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
        let addr64 = match src_loc {
            Operand::Register(r) => get_gpr64(r).unwrap(),
            Operand::Stack(offset) => {
                let tmp = self.scratch(ctx)?;
                self.asm
                    .mov(get_gpr64(tmp).unwrap(), qword_ptr(rbp - offset))?;
                ctx.registers.track(tmp, src);
                get_gpr64(tmp).unwrap()
            }
            _ => unreachable!(),
        };

        match size {
            Memory::Byte => self.asm.movzx(dst64, byte_ptr(addr64))?,
            Memory::Word => self.asm.movzx(dst64, word_ptr(addr64))?,
            Memory::Dword => self.asm.mov(self.to_reg32(dst_reg), dword_ptr(addr64))?,
            Memory::Qword => self.asm.mov(dst64, qword_ptr(addr64))?,
        }

        ctx.registers.track(dst_reg, Value::Symbol(dst));

        Ok(())
    }

    pub(crate) fn compile_store(
        &mut self,
        ctx: &mut FunctionContext,
        size: Memory,
        dst: Value,
        src: Value,
    ) -> Result<(), Box<dyn Error>> {
        let addr_reg = self.scratch(ctx)?;
        self.load_to_register(ctx, dst, addr_reg)?;

        let data_reg = self.scratch(ctx)?;
        self.load_to_register(ctx, src, data_reg)?;

        let addr64 = get_gpr64(addr_reg).unwrap();

        match size {
            Memory::Byte => self.asm.mov(byte_ptr(addr64), self.to_reg8(data_reg))?,
            Memory::Word => self.asm.mov(word_ptr(addr64), self.to_reg16(data_reg))?,
            Memory::Dword => self.asm.mov(dword_ptr(addr64), self.to_reg32(data_reg))?,
            Memory::Qword => self
                .asm
                .mov(qword_ptr(addr64), get_gpr64(data_reg).unwrap())?,
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

        ctx.registers.track(dst_reg, Value::Symbol(dst));

        Ok(())
    }
}
