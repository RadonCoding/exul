use crate::{
    convention::Convention,
    registers::{self, LoadAction, Registers, ValueContext, ValueLocation},
};
use iced_x86::{
    Register,
    code_asm::{AsmRegister8, CodeAssembler, CodeLabel, get_gpr8, get_gpr64, qword_ptr, rbp, rsp},
};
use intermediate::{Function, Instruction, InstructionKind, LabelId, Module, SymbolId, Value};
use std::{collections::HashMap, error::Error};

pub struct Emitter<C: Convention> {
    pub(crate) asm: CodeAssembler,
    pub(crate) convention: C,
    pub(crate) functions: HashMap<SymbolId, CodeLabel>,
}

pub(crate) struct FunctionContext<'a> {
    pub(crate) allocs: HashMap<SymbolId, Register>,
    pub(crate) slots: HashMap<SymbolId, i32>,
    pub(crate) labels: HashMap<LabelId, CodeLabel>,
    pub(crate) pending: Vec<LabelId>,
    pub(crate) epilogue: CodeLabel,
    pub(crate) cursor: usize,
    pub(crate) instructions: &'a [Instruction],
    pub(crate) registers: Registers,
}

impl<C: Convention> Emitter<C> {
    pub fn new(convention: C) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            asm: CodeAssembler::new(64)?,
            convention,
            functions: HashMap::new(),
        })
    }

    pub(crate) fn to_reg8(&self, r: Register) -> AsmRegister8 {
        let reg8 = match r.full_register() {
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

    pub(super) fn ret(&self) -> Register {
        self.convention.return_reg()
    }

    pub(super) fn vol(&self) -> Register {
        self.convention.volatile_regs()[0]
    }

    pub(super) fn value_context<'a>(&'a self, ctx: &'a FunctionContext) -> ValueContext<'a> {
        ValueContext {
            allocs: &ctx.allocs,
            slots: &ctx.slots,
            registers: &ctx.registers,
        }
    }

    pub(super) fn load_to_register(
        &mut self,
        ctx: &mut FunctionContext,
        val: Value,
        target: Register,
    ) -> Result<bool, Box<dyn Error>> {
        let vctx = self.value_context(ctx);
        let action = registers::plan_load(val, target, &vctx);

        let target64 = get_gpr64(target).unwrap();
        let did_load = !matches!(action, LoadAction::None);

        registers::execute_load(&mut self.asm, action, target64)?;

        if did_load {
            ctx.registers.track(target, val);
        }

        Ok(did_load)
    }

    pub(super) fn ensure_in_register(
        &mut self,
        ctx: &mut FunctionContext,
        val: Value,
        hint: Register,
    ) -> Result<Register, Box<dyn Error>> {
        let vctx = self.value_context(ctx);
        let location = vctx.locate(val);

        match location {
            ValueLocation::Register(r) => Ok(r),
            _ => {
                self.load_to_register(ctx, val, hint)?;
                Ok(hint)
            }
        }
    }

    pub(super) fn store_symbol(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        src_reg: Register,
    ) -> Result<(), Box<dyn Error>> {
        ctx.registers.invalidate_symbol(dst);

        if let Some(&offset) = ctx.slots.get(&dst) {
            if ctx.registers.find_value(Value::Symbol(dst)) != Some(src_reg) {
                let src64 = get_gpr64(src_reg).unwrap();
                self.asm.mov(qword_ptr(rbp - offset), src64)?;
            }
            ctx.registers.track(src_reg, Value::Symbol(dst));
        } else if let Some(&dst_reg) = ctx.allocs.get(&dst) {
            if src_reg != dst_reg {
                let dst64 = get_gpr64(dst_reg).unwrap();
                let src64 = get_gpr64(src_reg).unwrap();
                self.asm.mov(dst64, src64)?;
            }
            ctx.registers.track(dst_reg, Value::Symbol(dst));
        } else {
            unreachable!()
        }

        Ok(())
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

    pub fn emit(&mut self, ip: u64, module: &Module) -> Result<Vec<u8>, Box<dyn Error>> {
        for function in &module.functions {
            self.functions.insert(function.id, self.asm.create_label());
        }

        for function in &module.functions {
            self.emit_function(function)?;
        }
        Ok(self.asm.assemble(ip)?)
    }

    fn emit_function(&mut self, function: &Function) -> Result<(), Box<dyn Error>> {
        self.asm
            .set_label(self.functions.get_mut(&function.id).unwrap())?;

        self.asm.push(rbp)?;
        self.asm.mov(rbp, rsp)?;

        let epilogue = self.asm.create_label();

        let mut ctx = FunctionContext {
            allocs: HashMap::new(),
            slots: HashMap::new(),
            labels: HashMap::new(),
            pending: Vec::new(),
            epilogue,
            cursor: 0,
            instructions: &function.instructions,
            registers: Registers::new(),
        };

        self.run_allocator(&mut ctx, function.params);

        for i in 0..function.params {
            let sym = SymbolId(i);
            let d = ctx.allocs[&sym];
            let d64 = get_gpr64(d).unwrap();

            if let Some(s) = self.convention.argument_reg(i) {
                if s != d {
                    self.asm.mov(d64, get_gpr64(s).unwrap())?;
                }
            } else {
                let offset = 16
                    + self.convention.shadow_space()
                    + ((i - self.convention.argument_regs().len()) * 8);
                self.asm.mov(d64, qword_ptr(rbp + offset as i32))?;
            }
            ctx.registers.track(d, Value::Symbol(sym));
        }

        let shadow = self.convention.shadow_space() as i32;
        let space = ctx
            .slots
            .values()
            .copied()
            .max()
            .map(|max| max + 8)
            .unwrap_or(shadow);
        let stack = ((space + 15) & !15) as i32;

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
            InstructionKind::Add { dst, left, right } => self.compile_add(ctx, dst, left, right),
            InstructionKind::Eq { dst, left, right } => self.compile_eq(ctx, dst, left, right),
            InstructionKind::Assign { dst, src } => self.compile_assign(ctx, dst, src),
            InstructionKind::Call { dst, callee, args } => {
                self.compile_call(ctx, dst, callee, args)
            }
            InstructionKind::Return(val) => self.compile_ret(ctx, val),
            InstructionKind::JumpIfFalse { .. }
            | InstructionKind::JumpIfNotEq { .. }
            | InstructionKind::Jump { .. } => self.compile_branch(ctx, kind),
            _ => unreachable!(),
        }
    }

    fn compile_assign(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        src: Value,
    ) -> Result<(), Box<dyn Error>> {
        let target_reg = if let Some(&r) = ctx.allocs.get(&dst) {
            r
        } else {
            self.ret()
        };
        self.load_to_register(ctx, src, target_reg)?;
        self.store_symbol(ctx, dst, target_reg)?;
        Ok(())
    }
}
