use crate::convention::Convention;
use iced_x86::{
    Register,
    code_asm::{
        AsmRegister8, AsmRegister32, AsmRegister64, CodeAssembler, CodeLabel, get_gpr8, get_gpr32,
        get_gpr64, rbp, rsp,
    },
};
use intermediate::{Function, InstructionKind, LabelId, Module, SymbolId, Value};
use std::{collections::HashMap, error::Error};

pub struct Emitter<C: Convention> {
    pub(crate) asm: CodeAssembler,
    pub(crate) convention: C,
    pub(crate) allocs: HashMap<SymbolId, Register>,
    pub(crate) labels: HashMap<LabelId, CodeLabel>,
    pub(crate) pending: Vec<LabelId>,
}

impl<C: Convention> Emitter<C> {
    pub fn new(convention: C) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            asm: CodeAssembler::new(64)?,
            convention,
            allocs: HashMap::new(),
            labels: HashMap::new(),
            pending: Vec::new(),
        })
    }

    pub(crate) fn reg(&self, id: SymbolId) -> AsmRegister64 {
        get_gpr64(self.allocs[&id]).unwrap()
    }

    pub(crate) fn reg32(&self, id: SymbolId) -> AsmRegister32 {
        let r = self.allocs[&id];
        get_gpr32(r.full_register32()).unwrap()
    }

    pub(crate) fn low8(&self, id: SymbolId) -> AsmRegister8 {
        let r = self.allocs[&id];
        let reg8 = match r {
            Register::RAX => Register::AL,
            Register::RCX => Register::CL,
            Register::RDX => Register::DL,
            Register::RBX => Register::BL,
            Register::RSP => Register::SPL,
            Register::RBP => Register::BPL,
            Register::RSI => Register::SIL,
            Register::RDI => Register::DIL,
            Register::R8 => Register::R8L,
            Register::R9 => Register::R9L,
            Register::R10 => Register::R10L,
            Register::R11 => Register::R11L,
            Register::R12 => Register::R12L,
            Register::R13 => Register::R13L,
            Register::R14 => Register::R14L,
            Register::R15 => Register::R15L,
            _ => unreachable!(),
        };
        get_gpr8(reg8).unwrap()
    }

    pub(crate) fn get_label(&mut self, id: LabelId) -> CodeLabel {
        *self
            .labels
            .entry(id)
            .or_insert_with(|| self.asm.create_label())
    }

    pub(crate) fn bind_pending(&mut self) -> Result<(), Box<dyn Error>> {
        if self.pending.is_empty() {
            return Ok(());
        }

        let mut master = None;

        for &id in &self.pending {
            if let Some(&existing) = self.labels.get(&id) {
                master = Some(existing);
                break;
            }
        }

        let mut master = master.unwrap_or_else(|| self.asm.create_label());
        self.asm.set_label(&mut master)?;

        for id in self.pending.drain(..) {
            self.labels.insert(id, master);
        }

        Ok(())
    }

    pub(crate) fn mov_val(&mut self, dst: SymbolId, val: Value) -> Result<(), Box<dyn Error>> {
        let d = self.reg(dst);
        let d32 = self.reg32(dst);
        match val {
            Value::Symbol(s) => {
                let r = self.reg(s);
                if d != r {
                    self.asm.mov(d, r)?;
                }
            }
            Value::Constant(c) => {
                if c >= 0 && c <= u32::MAX as i64 {
                    self.asm.mov(d32, c as u32)?;
                } else {
                    self.asm.mov(d, c as u64)?;
                }
            }
        }
        Ok(())
    }

    pub fn emit(&mut self, ip: u64, module: Module) -> Result<Vec<u8>, Box<dyn Error>> {
        for func in module.functions {
            self.labels.clear();
            self.pending.clear();
            self.run_allocator(&func);
            self.emit_function(func)?;
        }
        Ok(self.asm.assemble(ip)?)
    }

    fn emit_function(&mut self, func: Function) -> Result<(), Box<dyn Error>> {
        self.asm.push(rbp)?;
        self.asm.mov(rbp, rsp)?;
        let stack = (self.convention.shadow_space() + 15) & !15;
        if stack > 0 {
            self.asm.sub(rsp, stack as i32)?;
        }

        for instruction in func.instructions {
            match instruction.kind {
                InstructionKind::Label(id) => self.pending.push(id),
                _ => {
                    self.bind_pending()?;
                    self.compile_instruction(instruction.kind, stack)?;
                }
            }
        }

        if !self.pending.is_empty() {
            self.bind_pending()?;
            self.asm.ret()?;
        }
        Ok(())
    }

    fn compile_instruction(
        &mut self,
        kind: InstructionKind,
        stack_size: usize,
    ) -> Result<(), Box<dyn Error>> {
        match kind {
            InstructionKind::Add { .. } => self.compile_add(kind),
            InstructionKind::Eq { .. } => self.compile_eq(kind),
            InstructionKind::Assign { dst, src } => self.mov_val(dst, src),
            InstructionKind::Return(val) => self.compile_ret(val, stack_size),
            InstructionKind::JumpIfFalse { .. }
            | InstructionKind::JumpIfNotEq { .. }
            | InstructionKind::Jump { .. } => self.compile_branch(kind),
            _ => unreachable!(),
        }
    }
}
