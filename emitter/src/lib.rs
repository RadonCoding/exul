use std::{collections::HashMap, error::Error};

use crate::{
    allocator::Allocator,
    convention::Convention,
    registers::{Operand, Registers},
};
use iced_x86::{
    BlockEncoderOptions, Register,
    code_asm::{
        AsmRegister8, AsmRegister16, AsmRegister32, AsmRegister64, CodeAssembler, CodeLabel,
        get_gpr8, get_gpr16, get_gpr32, get_gpr64, ptr, qword_ptr, rbp, rsp,
    },
};
use intermediate::{
    Function, FunctionId, Instruction, InstructionKind, LabelId, Module, SymbolId, Value,
};

mod allocator;
mod arithmetic;
mod branches;
pub mod convention;
mod memory;
mod peephole;
mod registers;

pub struct Assembly {
    pub bytes: Vec<u8>,
    pub blobs: Vec<Blob>,
    pub functions: HashMap<FunctionId, usize>,
}

pub struct Blob {
    pub offset: usize,
    pub len: usize,
    pub content: Vec<u8>,
}

pub fn emit<C: Convention>(ip: u64, module: &mut Module) -> Result<Assembly, Box<dyn Error>> {
    let mut emitter = Emitter::new(C::default())?;

    for f in &mut module.functions {
        peephole::optimize(f);
    }

    emitter.emit(ip, module)
}

pub struct Emitter<C: Convention> {
    pub(crate) asm: CodeAssembler,
    pub(crate) convention: C,
    pub(crate) data_labels: Vec<CodeLabel>,
    pub(crate) data_bytes: Vec<Vec<u8>>,
    pub(crate) imports: HashMap<FunctionId, CodeLabel>,
    pub(crate) functions: HashMap<FunctionId, CodeLabel>,
    pub(crate) labels: Vec<CodeLabel>,
}

pub(crate) struct FunctionContext<'a> {
    pub(crate) slots: HashMap<SymbolId, i32>,
    pub(crate) labels: HashMap<LabelId, usize>,
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
            data_labels: Vec::new(),
            data_bytes: Vec::new(),
            imports: HashMap::new(),
            functions: HashMap::new(),
            labels: Vec::new(),
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

    pub(crate) fn to_reg16(&self, r: Register) -> AsmRegister16 {
        let reg16 = match r.full_register() {
            Register::RAX => Register::AX,
            Register::RCX => Register::CX,
            Register::RDX => Register::DX,
            Register::RBX => Register::BX,
            Register::RSP => Register::SP,
            Register::RBP => Register::BP,
            Register::RSI => Register::SI,
            Register::RDI => Register::DI,
            Register::R8 => Register::R8W,
            Register::R9 => Register::R9W,
            Register::R10 => Register::R10W,
            Register::R11 => Register::R11W,
            Register::R12 => Register::R12W,
            Register::R13 => Register::R13W,
            Register::R14 => Register::R14W,
            Register::R15 => Register::R15W,
            _ => unreachable!(),
        };
        get_gpr16(reg16).unwrap()
    }

    pub(crate) fn to_reg32(&self, r: Register) -> AsmRegister32 {
        get_gpr32(r.full_register32()).unwrap()
    }

    pub(crate) fn ret(&self) -> Register {
        self.convention.return_reg()
    }

    /// Returns a free volatile register, evicting to the stack if all are occupied.
    pub(crate) fn scratch(
        &mut self,
        ctx: &mut FunctionContext,
    ) -> Result<Register, Box<dyn Error>> {
        let volatiles = self.convention.volatile_regs();

        if let Some(r) = ctx.registers.free(volatiles.iter()) {
            return Ok(r);
        }

        // No free registers, push a victim back to its slot to make room
        let (reg, offset) = ctx.registers.evict(&volatiles, &ctx.slots);
        self.asm
            .mov(qword_ptr(rbp - offset), get_gpr64(reg).unwrap())?;
        Ok(reg)
    }

    /// Loads a value into a specific register, emitting a move only when necessary.
    pub(crate) fn load_to_register(
        &mut self,
        ctx: &mut FunctionContext,
        val: Value,
        reg: Register,
    ) -> Result<(), Box<dyn Error>> {
        let loc = ctx.registers.locate(val, &ctx.slots, &self.data_labels);

        match loc {
            Operand::Register(r) if r == reg => return Ok(()),
            Operand::Register(r) => {
                self.asm
                    .mov(get_gpr64(reg).unwrap(), get_gpr64(r).unwrap())?;
            }
            Operand::Stack(offset) => {
                self.asm
                    .mov(get_gpr64(reg).unwrap(), qword_ptr(rbp - offset))?;
            }
            Operand::Immediate(imm) => {
                if imm == 0 {
                    let r32 = get_gpr32(reg.full_register32()).unwrap();
                    self.asm.xor(r32, r32)?;
                } else if imm >= 0 && imm <= u32::MAX as i64 {
                    self.asm
                        .mov(get_gpr32(reg.full_register32()).unwrap(), imm as u32)?;
                } else {
                    self.asm.mov(get_gpr64(reg).unwrap(), imm as u64)?;
                }
            }
            Operand::String(label) => {
                self.asm.lea(get_gpr64(reg).unwrap(), ptr(label))?;
            }
        }

        ctx.registers.track(reg, val);
        Ok(())
    }

    /// Resolves the right operand before clobbering its register with the left, copying to a scratch register if they alias.
    pub(crate) fn binary_operands(
        &mut self,
        ctx: &mut FunctionContext,
        left: Value,
        right: Value,
        reg: Register,
    ) -> Result<Operand, Box<dyn Error>> {
        let right_loc = ctx.registers.locate(right, &ctx.slots, &self.data_labels);

        let right_loc = match right_loc {
            Operand::Register(r) if r == reg => {
                let scratch = self
                    .convention
                    .volatile_regs()
                    .iter()
                    .find(|&&r| r != reg)
                    .copied()
                    .unwrap();
                self.load_to_register(ctx, right, scratch)?;
                Operand::Register(scratch)
            }
            loc => loc,
        };

        self.load_to_register(ctx, left, reg)?;

        Ok(right_loc)
    }

    /// Emits a binary arithmetic operation and writes the result to its spill slot.
    pub(crate) fn emit_binary(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
        op: impl FnOnce(&mut Self, AsmRegister64, Operand) -> Result<(), Box<dyn Error>>,
    ) -> Result<(), Box<dyn Error>> {
        let dst_reg = self.scratch(ctx)?;
        let dst64 = get_gpr64(dst_reg).unwrap();

        let right_loc = self.binary_operands(ctx, left, right, dst_reg)?;

        op(self, dst64, right_loc)?;
        self.spill(ctx, dst, dst_reg)?;
        Ok(())
    }

    /// Emits a comparison that materializes the boolean result into a register.
    pub(crate) fn emit_compare_value(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
        setcc: impl FnOnce(&mut Self, AsmRegister64) -> Result<(), Box<dyn Error>>,
    ) -> Result<(), Box<dyn Error>> {
        let dst_reg = self.scratch(ctx)?;
        let dst64 = get_gpr64(dst_reg).unwrap();

        let right_loc = self.binary_operands(ctx, left, right, dst_reg)?;

        match right_loc {
            Operand::Register(r) => self.asm.cmp(dst64, get_gpr64(r).unwrap())?,
            Operand::Stack(offset) => self.asm.cmp(dst64, qword_ptr(rbp - offset))?,
            Operand::Immediate(imm) => self.asm.cmp(dst64, imm as i32)?,
            _ => unreachable!(),
        }

        setcc(self, dst64)?;
        self.spill(ctx, dst, dst_reg)?;
        Ok(())
    }

    /// Emits a comparison that sets CPU flags without materializing the result.
    pub(crate) fn emit_compare_flags(
        &mut self,
        ctx: &mut FunctionContext,
        left: Value,
        right: Value,
    ) -> Result<(), Box<dyn Error>> {
        let reg = self.scratch(ctx)?;
        let right_loc = self.binary_operands(ctx, left, right, reg)?;
        let left64 = get_gpr64(reg).unwrap();

        match right_loc {
            Operand::Register(r) => self.asm.cmp(left64, get_gpr64(r).unwrap())?,
            Operand::Stack(offset) => self.asm.cmp(left64, qword_ptr(rbp - offset))?,
            Operand::Immediate(imm) => self.asm.cmp(left64, imm as i32)?,
            _ => unreachable!(),
        }

        Ok(())
    }

    /// Writes a register value back to its guaranteed spill slot and updates the register tracker.
    pub(crate) fn spill(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        reg: Register,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(&offset) = ctx.slots.get(&dst) {
            self.asm
                .mov(qword_ptr(rbp - offset), get_gpr64(reg).unwrap())?;
            ctx.registers.track(reg, Value::Symbol(dst));
        }

        Ok(())
    }

    pub(crate) fn get_label(&mut self, ctx: &mut FunctionContext, id: LabelId) -> CodeLabel {
        let index = *ctx.labels.entry(id).or_insert_with(|| {
            let i = self.labels.len();
            self.labels.push(self.asm.create_label());
            i
        });
        self.labels[index]
    }

    /// Consolidates pending virtual labels into a single physical code offset.
    fn bind_pending(&mut self, ctx: &mut FunctionContext) -> Result<(), Box<dyn Error>> {
        if ctx.pending.is_empty() {
            return Ok(());
        }

        let existing = ctx
            .pending
            .iter()
            .filter_map(|&id| ctx.labels.get(&id).copied())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<usize>>();

        let master = if !existing.is_empty() {
            existing[0]
        } else {
            let i = self.labels.len();
            self.labels.push(self.asm.create_label());
            i
        };

        self.asm.set_label(&mut self.labels[master])?;

        for &slot in existing.iter().skip(1) {
            self.asm.zero_bytes()?;
            self.asm.set_label(&mut self.labels[slot])?;
        }

        let ids = ctx.pending.drain(..).collect::<Vec<LabelId>>();

        for id in ids {
            ctx.labels.insert(id, master);
        }

        Ok(())
    }

    pub fn emit(&mut self, ip: u64, module: &Module) -> Result<Assembly, Box<dyn Error>> {
        for s in &module.strings {
            let label = self.asm.create_label();
            let bytes = s.bytes().chain([0]).collect::<Vec<u8>>();
            self.data_labels.push(label);
            self.data_bytes.push(bytes);
        }

        for import in &module.imports {
            let slot = self.asm.create_label();
            self.imports.insert(import.id, slot);
        }

        for function in &module.functions {
            self.functions.insert(function.id, self.asm.create_label());
        }

        // Assemble the entry point first.
        for function in module
            .functions
            .iter()
            .cycle()
            .skip(module.entry.unwrap_or(0))
            .take(module.functions.len())
        {
            self.emit_function(function)?;
        }

        for i in 0..self.data_labels.len() {
            self.asm.set_label(&mut self.data_labels[i])?;
            self.asm.db(&self.data_bytes[i])?;
        }

        for import in &module.imports {
            let mut slot = self.imports[&import.id];
            self.asm.set_label(&mut slot)?;
            self.asm.dq(&[0u64])?;
        }

        let result = self
            .asm
            .assemble_options(ip, BlockEncoderOptions::RETURN_NEW_INSTRUCTION_OFFSETS)?;

        let sections = self
            .data_labels
            .iter()
            .zip(self.data_bytes.iter())
            .map(|(label, bytes)| Blob {
                offset: (result.label_ip(label).unwrap() - ip) as usize,
                len: bytes.len(),
                content: bytes.clone(),
            })
            .collect();

        let functions = self
            .functions
            .iter()
            .map(|(&id, label)| (id, (result.label_ip(label).unwrap() - ip) as usize))
            .collect();

        Ok(Assembly {
            bytes: result.inner.code_buffer,
            blobs: sections,
            functions,
        })
    }

    fn allocate(&mut self, ctx: &mut FunctionContext, params: &[SymbolId]) {
        let mut allocator = Allocator::new(ctx.instructions, &self.convention);
        allocator.allocate_parameters(&mut ctx.slots, params);
        allocator.allocate_symbols(&mut ctx.slots);
    }

    fn emit_function(&mut self, function: &Function) -> Result<(), Box<dyn Error>> {
        self.asm
            .set_label(self.functions.get_mut(&function.id).unwrap())?;

        self.asm.push(rbp)?;
        self.asm.mov(rbp, rsp)?;

        let epilogue = self.asm.create_label();

        let mut ctx = FunctionContext {
            slots: HashMap::new(),
            labels: HashMap::new(),
            pending: Vec::new(),
            epilogue,
            cursor: 0,
            instructions: &function.instructions,
            registers: Registers::new(),
        };

        self.allocate(&mut ctx, &function.params);

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

        // Copy parameters from their ABI location into their spill slots.
        for (i, &sym) in function.params.iter().enumerate() {
            let slot = ctx.slots[&sym];

            if let Some(arg_reg) = self.convention.argument_reg(i) {
                self.asm
                    .mov(qword_ptr(rbp - slot), get_gpr64(arg_reg).unwrap())?;
            } else {
                let offset = 16
                    + self.convention.shadow_space()
                    + ((i - self.convention.argument_regs().len()) * 8);
                let reg = self.scratch(&mut ctx)?;
                self.asm
                    .mov(get_gpr64(reg).unwrap(), qword_ptr(rbp + offset as i32))?;
                self.asm
                    .mov(qword_ptr(rbp - slot), get_gpr64(reg).unwrap())?;
            }
        }

        for i in 0..ctx.instructions.len() {
            ctx.cursor = i;

            let instruction = ctx.instructions[i].clone();

            match instruction.kind {
                InstructionKind::Label(id) => {
                    ctx.registers.invalidate();
                    ctx.pending.push(id);
                }
                _ => {
                    self.bind_pending(&mut ctx)?;
                    self.compile_instruction(&mut ctx, instruction.kind)?;
                }
            }
        }

        self.bind_pending(&mut ctx)?;

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
            InstructionKind::Eq { dst, left, right } => self.compile_eq(ctx, dst, left, right),
            InstructionKind::NotEq { dst, left, right } => {
                self.compile_not_eq(ctx, dst, left, right)
            }
            InstructionKind::Lte { dst, left, right } => self.compile_lte(ctx, dst, left, right),
            InstructionKind::Gte { dst, left, right } => self.compile_gte(ctx, dst, left, right),
            InstructionKind::Lt { dst, left, right } => self.compile_lt(ctx, dst, left, right),
            InstructionKind::Gt { dst, left, right } => self.compile_gt(ctx, dst, left, right),
            InstructionKind::Add { dst, left, right } => self.compile_add(ctx, dst, left, right),
            InstructionKind::Sub { dst, left, right } => self.compile_sub(ctx, dst, left, right),
            InstructionKind::Mul { dst, left, right } => self.compile_mul(ctx, dst, left, right),
            InstructionKind::And { dst, left, right } => self.compile_and(ctx, dst, left, right),
            InstructionKind::Or { dst, left, right } => self.compile_or(ctx, dst, left, right),
            InstructionKind::Xor { dst, left, right } => self.compile_xor(ctx, dst, left, right),
            InstructionKind::Shl { dst, left, right } => self.compile_shl(ctx, dst, left, right),
            InstructionKind::Shr { dst, left, right } => self.compile_shr(ctx, dst, left, right),
            InstructionKind::Assign { dst, src } => self.compile_assign(ctx, dst, src),
            InstructionKind::Call { dst, callee, args } => {
                self.compile_call(ctx, dst, callee, args)
            }
            InstructionKind::Return(val) => self.compile_ret(ctx, val),
            InstructionKind::Load { dst, size, src } => self.compile_load(ctx, dst, size, src),
            InstructionKind::Store { size, dst, src } => self.compile_store(ctx, size, dst, src),
            InstructionKind::Import { import, src } => self.compile_import(ctx, import, src),
            InstructionKind::Segment { dst, seg, offset } => {
                self.compile_segment(ctx, dst, seg, offset)
            }
            InstructionKind::JumpIfFalse { .. }
            | InstructionKind::JumpIfEq { .. }
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
        let reg = self.scratch(ctx)?;
        self.load_to_register(ctx, src, reg)?;
        self.spill(ctx, dst, reg)?;
        Ok(())
    }

    fn compile_import(
        &mut self,
        ctx: &mut FunctionContext,
        import: FunctionId,
        src: Value,
    ) -> Result<(), Box<dyn Error>> {
        let ret = self.ret();
        self.load_to_register(ctx, src, ret)?;
        let src64 = get_gpr64(ret).unwrap();
        let slot = self.imports[&import];
        self.asm.mov(ptr(slot), src64)?;
        Ok(())
    }
}
