use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
};

use crate::{
    allocator::{self, Allocator},
    assembly::{Assembly, Blob},
    context::FunctionContext,
    convention::Convention,
    macros::{r32, r64},
    registers::{Operand, Registers},
};
use iced_x86::{
    BlockEncoderOptions, Register,
    code_asm::{CodeAssembler, CodeLabel, ptr, rbp, rsp},
};
use intermediate::{Function, FunctionId, InstructionKind, LabelId, Module, SymbolId, Value};

pub struct Emitter<C: Convention> {
    pub(crate) asm: CodeAssembler,
    pub(crate) convention: C,
    pub(crate) data_labels: Vec<CodeLabel>,
    pub(crate) data_bytes: Vec<Vec<u8>>,
    pub(crate) imports: BTreeMap<FunctionId, CodeLabel>,
    pub(crate) functions: BTreeMap<FunctionId, CodeLabel>,
    pub(crate) labels: Vec<CodeLabel>,
}

impl<C: Convention> Emitter<C> {
    pub fn new(convention: C) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            asm: CodeAssembler::new(64)?,
            convention,
            data_labels: Vec::new(),
            data_bytes: Vec::new(),
            imports: BTreeMap::new(),
            functions: BTreeMap::new(),
            labels: Vec::new(),
        })
    }

    /// Returns a free volatile register, evicting to the stack if all are occupied.
    pub(crate) fn scratch(
        &mut self,
        ctx: &mut FunctionContext,
    ) -> Result<Register, Box<dyn Error>> {
        let volatiles = self.convention.volatile_registers();

        if let Some(r) = ctx.registers.free(volatiles.iter()) {
            return Ok(r);
        }

        // No free registers, push a victim back to its slot to make room.
        let (reg, sym) = ctx.registers.evict(&volatiles);

        self.spill_symbol(ctx, sym, reg)?;

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
                self.asm.mov(r64!(reg), r64!(r))?;
            }
            Operand::Stack(offset) => {
                self.asm.mov(r64!(reg), ptr(rbp - offset))?;
            }
            Operand::Immediate(imm) => {
                if imm == 0 {
                    self.asm.xor(r32!(reg), r32!(reg))?;
                } else if imm >= 0 && imm <= u32::MAX as i64 {
                    self.asm.mov(r32!(reg), imm as u32)?;
                } else {
                    self.asm.mov(r64!(reg), imm as u64)?;
                }
            }
            Operand::String(label) => {
                self.asm.lea(r64!(reg), ptr(label))?;
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
        let rloc = ctx.registers.locate(right, &ctx.slots, &self.data_labels);

        let rloc = match rloc {
            Operand::Register(r) if r == reg => {
                let scratch = self
                    .convention
                    .volatile_registers()
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

        Ok(rloc)
    }

    /// Emits a binary arithmetic operation and writes the result to its spill slot.
    pub(crate) fn emit_binary(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
        op: impl FnOnce(&mut Self, Register, Operand) -> Result<(), Box<dyn Error>>,
    ) -> Result<(), Box<dyn Error>> {
        let dreg = self.scratch(ctx)?;
        let rloc = self.binary_operands(ctx, left, right, dreg)?;

        op(self, dreg, rloc)?;

        self.spill_and_track(ctx, dst, dreg)?;

        Ok(())
    }

    /// Emits a comparison that materializes the boolean result into a register.
    pub(crate) fn emit_compare_value(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        left: Value,
        right: Value,
        setcc: impl FnOnce(&mut Self, Register) -> Result<(), Box<dyn Error>>,
    ) -> Result<(), Box<dyn Error>> {
        let dreg = self.scratch(ctx)?;
        self.emit_compare_flags(ctx, left, right)?;

        setcc(self, dreg)?;

        self.spill_and_track(ctx, dst, dreg)?;

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
        let rloc = self.binary_operands(ctx, left, right, reg)?;

        match rloc {
            Operand::Register(r) => self.asm.cmp(r64!(reg), r64!(r))?,
            Operand::Stack(offset) => self.asm.cmp(r64!(reg), ptr(rbp - offset))?,
            Operand::Immediate(imm) => self.asm.cmp(r64!(reg), imm as i32)?,
            _ => unreachable!(),
        }

        Ok(())
    }

    fn store(
        &mut self,
        ctx: &mut FunctionContext,
        sym: SymbolId,
        reg: Register,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(&offset) = ctx.slots.get(&sym) {
            let is_live = ctx
                .liveness
                .get(&sym)
                .map_or(false, |range| range.end > ctx.cursor);

            if is_live {
                self.asm.mov(ptr(rbp - offset), r64!(reg))?;
                ctx.registers.set_dirty(sym);
            }
        }
        Ok(())
    }

    /// Synchronizes a symbol with its stack slot and marks it as dirty
    fn spill_symbol(
        &mut self,
        ctx: &mut FunctionContext,
        sym: SymbolId,
        reg: Register,
    ) -> Result<(), Box<dyn Error>> {
        if !ctx.registers.is_dirty(sym) {
            self.store(ctx, sym, reg)?;
        }
        Ok(())
    }

    /// Synchronizes a register with its stack slot and tracks the symbol.
    pub(crate) fn spill_and_track(
        &mut self,
        ctx: &mut FunctionContext,
        dst: SymbolId,
        reg: Register,
    ) -> Result<(), Box<dyn Error>> {
        self.store(ctx, dst, reg)?;
        ctx.registers.track(reg, Value::Symbol(dst));
        Ok(())
    }

    /// Spills all symbols currently tracked in volatile registers, then clears register tracking.
    pub(crate) fn spill_volatiles(
        &mut self,
        ctx: &mut FunctionContext,
    ) -> Result<(), Box<dyn Error>> {
        let volatiles = self.convention.volatile_registers();
        self.spill_registers(ctx, |reg, _val| volatiles.contains(reg))?;
        ctx.registers.invalidate_volatiles(&self.convention);
        Ok(())
    }

    /// Spills all tracked symbols.
    pub(crate) fn spill_everything(
        &mut self,
        ctx: &mut FunctionContext,
    ) -> Result<(), Box<dyn Error>> {
        self.spill_registers(ctx, |_reg, val| matches!(val, Value::Symbol(_)))?;
        ctx.registers.invalidate();
        Ok(())
    }

    /// Spills symbols for registers matching the provided predicate.
    fn spill_registers<F>(
        &mut self,
        ctx: &mut FunctionContext,
        predicate: F,
    ) -> Result<(), Box<dyn Error>>
    where
        F: Fn(&Register, &Value) -> bool,
    {
        let spillees = ctx
            .registers
            .iter()
            .filter(|(reg, val)| predicate(reg, val))
            .filter_map(|(&reg, val)| {
                if let Value::Symbol(sym) = *val {
                    Some((reg, sym))
                } else {
                    None
                }
            })
            .collect::<Vec<(Register, SymbolId)>>();

        for (reg, sym) in spillees {
            self.spill_symbol(ctx, sym, reg)?;
        }

        Ok(())
    }

    /// Gets or creates a label for a [`LabelId`]
    pub(crate) fn get_label(&mut self, ctx: &mut FunctionContext, id: LabelId) -> CodeLabel {
        let index = *ctx.labels.entry(id).or_insert_with(|| {
            let index = self.labels.len();
            self.labels.push(self.asm.create_label());
            index
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
        let mut allocator = Allocator::new(&self.convention);
        allocator.allocate_parameters(&mut ctx.slots, params);
        allocator.allocate_symbols(&mut ctx.slots, &ctx.instructions);
    }

    fn emit_function(&mut self, function: &Function) -> Result<(), Box<dyn Error>> {
        self.asm
            .set_label(self.functions.get_mut(&function.id).unwrap())?;

        self.asm.push(rbp)?;
        self.asm.mov(rbp, rsp)?;

        let epilogue = self.asm.create_label();

        let liveness = allocator::compute_live_ranges(&function.instructions);

        let mut ctx = FunctionContext {
            slots: BTreeMap::new(),
            labels: BTreeMap::new(),
            pending: Vec::new(),
            epilogue,
            cursor: 0,
            instructions: &function.instructions,
            registers: Registers::new(),
            liveness,
        };

        for (i, &sym) in function.params.iter().enumerate() {
            if let Some(reg) = self.convention.argument_reg(i) {
                ctx.registers.track(reg, Value::Symbol(sym));
            } else {
                ctx.registers.set_dirty(sym);
            }
        }

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

        for (i, &sym) in function.params.iter().enumerate() {
            if let Some(reg) = self.convention.argument_reg(i) {
                self.spill_symbol(&mut ctx, sym, reg)?;
            }
        }

        for i in 0..ctx.instructions.len() {
            ctx.cursor = i;

            let instruction = ctx.instructions[i].clone();

            match instruction.kind {
                InstructionKind::Label(id) => {
                    self.spill_everything(&mut ctx)?;
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
}
