use crate::{
    context::FunctionContext,
    convention::Convention,
    emitter::Emitter,
    macros::{r8, r16, r32, r64},
};
use iced_x86::code_asm::{byte_ptr, dword_ptr, ptr, qword_ptr, rbp, word_ptr};
use intermediate::{Memory, Segment, SymbolId, Value};
use std::error::Error;

impl<C: Convention> Emitter<C> {
    fn resolve_address(
        &mut self,
        ctx: &mut FunctionContext,
        base: Option<SymbolId>,
        index: Option<(SymbolId, i32)>,
        d: i32,
    ) -> Result<iced_x86::code_asm::AsmMemoryOperand, Box<dyn Error>> {
        let base = if let Some(id) = base {
            let reg = self.scratch(ctx)?;
            self.load_to_register(ctx, Value::Symbol(id), reg)?;
            Some(r64!(reg))
        } else {
            None
        };
        let index = if let Some((id, scale)) = index {
            let reg = self.scratch(ctx)?;
            self.load_to_register(ctx, Value::Symbol(id), reg)?;
            Some((r64!(reg), scale))
        } else {
            None
        };
        match (base, index) {
            (Some(b), Some((i, s))) => Ok(ptr(b + i * s + d)),
            (Some(b), None) => Ok(ptr(b + d)),
            (None, Some((i, s))) => Ok(ptr(i * s + d)),
            (None, None) => Ok(ptr(d as u64)),
        }
    }

    pub(crate) fn compile_load(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        size: Memory,
        src: Value,
    ) -> Result<(), Box<dyn Error>> {
        let dreg = self.scratch(ctx)?;

        let smem = match src {
            Value::Address {
                base,
                index,
                displacement,
            } => self.resolve_address(ctx, base, index, displacement)?,
            _ => {
                let sreg = self.scratch(ctx)?;
                self.load_to_register(ctx, src, sreg)?;
                ptr(r64!(sreg))
            }
        };

        match size {
            Memory::Byte => self.asm.movzx(r64!(dreg), byte_ptr(smem))?,
            Memory::Word => self.asm.movzx(r64!(dreg), word_ptr(smem))?,
            Memory::Dword => self.asm.mov(r32!(dreg), dword_ptr(smem))?,
            Memory::Qword => self.asm.mov(r64!(dreg), qword_ptr(smem))?,
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
        let sreg = self.scratch(ctx)?;
        self.load_to_register(ctx, src, sreg)?;

        let dmem = match dst {
            Value::Address {
                base,
                index,
                displacement,
            } => self.resolve_address(ctx, base, index, displacement)?,
            _ => {
                let dreg = self.scratch(ctx)?;
                self.load_to_register(ctx, dst, dreg)?;
                ptr(r64!(dreg))
            }
        };

        match size {
            Memory::Byte => self.asm.mov(byte_ptr(dmem), r8!(sreg))?,
            Memory::Word => self.asm.mov(word_ptr(dmem), r16!(sreg))?,
            Memory::Dword => self.asm.mov(dword_ptr(dmem), r32!(sreg))?,
            Memory::Qword => self.asm.mov(qword_ptr(dmem), r64!(sreg))?,
        }

        Ok(())
    }

    pub(crate) fn compile_segment(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        seg: Segment,
        offset: Value,
    ) -> Result<(), Box<dyn Error>> {
        let dreg = self.scratch(ctx)?;

        match (seg, offset) {
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
