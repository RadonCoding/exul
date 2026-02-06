use std::error::Error;

use iced_x86::code_asm::{get_gpr32, get_gpr64, rbp, rsp};
use ir::{InstructionKind, Value};

use crate::{convention::Convention, emitter::Emitter};

impl<C: Convention> Emitter<C> {
    pub(crate) fn compile_branch(&mut self, kind: InstructionKind) -> Result<(), Box<dyn Error>> {
        match kind {
            InstructionKind::JumpIfFalse { cond, dst } => {
                let l = self.get_label(dst);

                match cond {
                    Value::Symbol(s) => {
                        let r = self.reg(s);
                        self.asm.test(r, r)?;
                    }
                    Value::Constant(c) => {
                        let tmp = get_gpr64(self.convention.volatile_regs()[0]).unwrap();
                        self.asm.mov(tmp, c as u64)?;
                        self.asm.test(tmp, tmp)?;
                    }
                }
                self.asm.jz(l)?;
            }
            InstructionKind::JumpIfNotEq { left, right, dst } => {
                let tmp = get_gpr64(self.convention.volatile_regs()[0]).unwrap();
                let tmp32 =
                    get_gpr32(self.convention.volatile_regs()[0].full_register32()).unwrap();

                match left {
                    Value::Symbol(s) => {
                        let r = self.reg(s);
                        if tmp != r {
                            self.asm.mov(tmp, r)?;
                        }
                    }
                    Value::Constant(c) => {
                        if c >= 0 && c <= u32::MAX as i64 {
                            self.asm.mov(tmp32, c as u32)?;
                        } else {
                            self.asm.mov(tmp, c as u64)?;
                        }
                    }
                }

                match right {
                    Value::Symbol(s) => {
                        let r = self.reg(s);
                        self.asm.cmp(tmp, r)?;
                    }
                    Value::Constant(c) => {
                        self.asm.cmp(tmp, c as i32)?;
                    }
                }

                let l = self.get_label(dst);
                self.asm.jne(l)?;
            }
            InstructionKind::Jump(id) => {
                let l = self.get_label(id);
                self.asm.jmp(l)?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    pub(crate) fn compile_ret(
        &mut self,
        val: Value,
        stack_size: usize,
    ) -> Result<(), Box<dyn Error>> {
        let r_raw = self.convention.return_reg();
        let r = get_gpr64(r_raw).unwrap();
        let r32 = get_gpr32(r_raw.info().full_register32()).unwrap();

        match val {
            Value::Symbol(s) => {
                let sr = self.reg(s);
                if r != sr {
                    self.asm.mov(r, sr)?;
                }
            }
            Value::Constant(c) => {
                if c >= 0 && c <= u32::MAX as i64 {
                    self.asm.mov(r32, c as u32)?;
                } else {
                    self.asm.mov(r, c as u64)?;
                }
            }
        }

        if stack_size > 0 {
            self.asm.add(rsp, stack_size as i32)?;
        }
        self.asm.pop(rbp)?;
        self.asm.ret()?;
        Ok(())
    }
}
