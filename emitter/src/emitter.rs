use crate::convention::Convention;
use iced_x86::{
    Register,
    code_asm::{
        AsmRegister8, AsmRegister32, AsmRegister64, CodeAssembler, CodeLabel, get_gpr8, get_gpr32,
        get_gpr64, rbp, rsp,
    },
};
use intermediate::{Function, Instruction, InstructionKind, LabelId, Module, SymbolId, Value};
use std::{collections::HashMap, error::Error};

pub struct Emitter<C: Convention> {
    pub(crate) asm: CodeAssembler,
    pub(crate) convention: C,
}

pub(crate) struct FunctionContext {
    pub(crate) allocs: HashMap<SymbolId, Register>,
    pub(crate) labels: HashMap<LabelId, CodeLabel>,
    pub(crate) pending: Vec<LabelId>,
    pub(crate) epilogue: CodeLabel,
    pub(crate) cursor: usize,
    pub(crate) instructions: Vec<Instruction>,
}

impl<C: Convention> Emitter<C> {
    pub fn new(convention: C) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            asm: CodeAssembler::new(64)?,
            convention,
        })
    }

    pub(crate) fn reg(&self, ctx: &FunctionContext, id: SymbolId) -> AsmRegister64 {
        get_gpr64(ctx.allocs[&id]).unwrap()
    }

    pub(crate) fn reg32(&self, ctx: &FunctionContext, id: SymbolId) -> AsmRegister32 {
        let r = ctx.allocs[&id];
        get_gpr32(r.full_register32()).unwrap()
    }

    pub(crate) fn reg8(&self, ctx: &FunctionContext, id: SymbolId) -> AsmRegister8 {
        let r = ctx.allocs[&id];
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

    pub(crate) fn ret(&self) -> AsmRegister64 {
        get_gpr64(self.convention.return_reg()).unwrap()
    }

    pub(crate) fn ret32(&self) -> AsmRegister32 {
        let r = self.convention.return_reg();
        get_gpr32(r.full_register32()).unwrap()
    }

    pub(crate) fn vol(&self) -> AsmRegister64 {
        get_gpr64(self.convention.volatile_regs()[0]).unwrap()
    }

    pub(crate) fn vol32(&self) -> AsmRegister32 {
        let r = self.convention.volatile_regs()[0];
        get_gpr32(r.full_register32()).unwrap()
    }

    pub(crate) fn get_label(&mut self, ctx: &mut FunctionContext, id: LabelId) -> CodeLabel {
        *ctx.labels
            .entry(id)
            .or_insert_with(|| self.asm.create_label())
    }

    fn bind_pending(&mut self, ctx: &mut FunctionContext) -> Result<(), Box<dyn Error>> {
        if ctx.pending.is_empty() {
            return Ok(());
        }

        let mut master = None;

        for &id in &ctx.pending {
            if let Some(&existing) = ctx.labels.get(&id) {
                master = Some(existing);
                break;
            }
        }

        let mut master = master.unwrap_or_else(|| self.asm.create_label());
        self.asm.set_label(&mut master)?;

        for id in ctx.pending.drain(..) {
            ctx.labels.insert(id, master);
        }

        Ok(())
    }

    pub(crate) fn mov_val(
        &mut self,
        ctx: &FunctionContext,
        dst: SymbolId,
        val: Value,
    ) -> Result<(), Box<dyn Error>> {
        let d = self.reg(ctx, dst);
        let d32 = self.reg32(ctx, dst);

        match val {
            Value::Symbol(s) => {
                let r = self.reg(ctx, s);
                if d != r {
                    self.asm.mov(d, r)?;
                }
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
        Ok(())
    }

    pub fn emit(&mut self, ip: u64, module: Module) -> Result<Vec<u8>, Box<dyn Error>> {
        for func in module.functions {
            self.emit_function(func)?;
        }
        Ok(self.asm.assemble(ip)?)
    }

    fn emit_function(&mut self, function: Function) -> Result<(), Box<dyn Error>> {
        self.asm.push(rbp)?;
        self.asm.mov(rbp, rsp)?;

        let stack = (self.convention.shadow_space() + 15) & !15;
        let epilogue = self.asm.create_label();

        let mut ctx = FunctionContext {
            allocs: HashMap::new(),
            labels: HashMap::new(),
            pending: Vec::new(),
            epilogue,
            cursor: 0,
            instructions: function.instructions,
        };

        self.run_allocator(&mut ctx);

        if stack > 0 {
            self.asm.sub(rsp, stack as i32)?;
        }

        for i in 0..ctx.instructions.len() {
            ctx.cursor = i;
            let instruction = ctx.instructions[i].clone();

            match instruction.kind {
                InstructionKind::Label(id) => ctx.pending.push(id),
                _ => {
                    self.bind_pending(&mut ctx)?;
                    self.compile_instruction(&mut ctx, instruction.kind)?;
                }
            }
        }

        if !ctx.pending.is_empty() {
            self.bind_pending(&mut ctx)?;
        }

        self.asm.set_label(&mut ctx.epilogue)?;

        if stack > 0 {
            self.asm.add(rsp, stack as i32)?;
        }
        self.asm.pop(rbp)?;
        self.asm.ret()?;

        Ok(())
    }

    fn compile_instruction(
        &mut self,
        ctx: &mut FunctionContext,
        kind: InstructionKind,
    ) -> Result<(), Box<dyn Error>> {
        match kind {
            InstructionKind::Add { .. } => self.compile_add(ctx, kind),
            InstructionKind::Eq { .. } => self.compile_eq(ctx, kind),
            InstructionKind::Assign { dst, src } => self.mov_val(ctx, dst, src),
            InstructionKind::Return(val) => self.compile_ret(ctx, val),
            InstructionKind::JumpIfFalse { .. }
            | InstructionKind::JumpIfNotEq { .. }
            | InstructionKind::Jump { .. } => self.compile_branch(ctx, kind),
            _ => unreachable!(),
        }
    }
}
