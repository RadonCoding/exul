use std::error::Error;

use intermediate::{InstructionKind, Value};

use crate::{
    convention::Convention,
    emitter::{Emitter, FunctionContext},
};

impl<C: Convention> Emitter<C> {
    pub(crate) fn compile_branch(
        &mut self,
        ctx: &mut FunctionContext,
        kind: InstructionKind,
    ) -> Result<(), Box<dyn Error>> {
        match kind {
            InstructionKind::JumpIfFalse { cond, dst } => {
                let l = self.get_label(ctx, dst);

                match cond {
                    Value::Symbol(s) => {
                        let r = self.reg(ctx, s);
                        self.asm.test(r, r)?;
                    }
                    Value::Constant(c) => {
                        let tmp = self.vol();
                        self.asm.mov(tmp, c as u64)?;
                        self.asm.test(tmp, tmp)?;
                    }
                }
                self.asm.jz(l)?;
            }
            InstructionKind::JumpIfNotEq { left, right, dst } => {
                let tmp = self.vol();
                let tmp32 = self.vol32();

                match left {
                    Value::Symbol(s) => {
                        let r = self.reg(ctx, s);
                        if tmp != r {
                            self.asm.mov(tmp, r)?;
                        }
                    }
                    Value::Constant(c) => {
                        if c == 0 {
                            self.asm.xor(tmp32, tmp32)?;
                        } else if c >= 0 && c <= u32::MAX as i64 {
                            self.asm.mov(tmp32, c as u32)?;
                        } else {
                            self.asm.mov(tmp, c as u64)?;
                        }
                    }
                }

                match right {
                    Value::Symbol(s) => {
                        let r = self.reg(ctx, s);
                        self.asm.cmp(tmp, r)?;
                    }
                    Value::Constant(c) => {
                        self.asm.cmp(tmp, c as i32)?;
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

    pub(crate) fn compile_ret(
        &mut self,
        ctx: &mut FunctionContext,
        val: Value,
    ) -> Result<(), Box<dyn Error>> {
        let d = self.ret();
        let d32 = self.ret32();

        match val {
            Value::Symbol(s) => {
                let s = self.reg(ctx, s);
                self.asm.mov(d, s)?;
            }
            Value::Constant(c) => {
                if c == 0 {
                    self.asm.xor(d32, d32)?;
                } else if c >= 0 && c <= u32::MAX as i64 {
                    self.asm.mov(d32, c as u32)?;
                } else {
                    self.asm.mov(d, c as u64)?;
                }
            }
        }

        if ctx.cursor < ctx.instructions.len() - 1 {
            self.asm.jmp(ctx.epilogue)?;
        }

        Ok(())
    }
}
