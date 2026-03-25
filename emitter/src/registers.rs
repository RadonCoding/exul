use crate::convention::Convention;
use iced_x86::{Register, code_asm::CodeLabel};
use intermediate::{SymbolId, Value};
use std::collections::{
    BTreeMap, HashSet,
    btree_map::{self},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    Register(Register),
    Stack(i32),
    Immediate(i64),
    String(CodeLabel),
}

pub struct Registers {
    /// Tracks which value currently resides in which register.
    tracked: BTreeMap<Register, Value>,
    /// Tracks which stack slots have been written at least once.
    written: HashSet<SymbolId>,
    /// Tracks registers whose current value is coherent with their stack slot.
    clean: HashSet<Register>,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            tracked: BTreeMap::new(),
            written: HashSet::new(),
            clean: HashSet::new(),
        }
    }

    pub fn iter(&self) -> btree_map::Iter<'_, Register, Value> {
        self.tracked.iter()
    }

    pub fn is_written(&self, sym: SymbolId) -> bool {
        self.written.contains(&sym)
    }

    pub fn set_written(&mut self, sym: SymbolId) {
        self.written.insert(sym);
    }

    pub fn is_clean(&self, reg: Register) -> bool {
        self.clean.contains(&reg)
    }

    pub fn set_clean(&mut self, reg: Register) {
        self.clean.insert(reg);
    }

    /// Tracks a value in a register, replacing any previous location of the same symbol.
    pub fn track(&mut self, reg: Register, val: Value) {
        if let Some(existing) = self.tracked.get(&reg) {
            panic!(
                "register '{:?}' already occupied by '{:?}', must be spilled before tracking '{:?}'",
                reg, existing, val
            );
        }
        if let Value::Symbol(s) = val {
            for (r, v) in &self.tracked {
                if *v == Value::Symbol(s) {
                    panic!(
                        "symbol '{:?}' is already tracked in '{:?}', must be untracked before tracking in '{:?}'",
                        s, r, reg
                    );
                }
            }
        }
        self.tracked.insert(reg, val);
        self.clean.remove(&reg);
    }

    /// Drops a specific register from tracking when it is about to be clobbered.
    pub fn untrack(&mut self, reg: Register) {
        self.tracked.remove(&reg);
        self.clean.remove(&reg);
    }

    pub fn invalidate(&mut self) {
        self.tracked.clear();
        self.clean.clear();
    }

    /// Drops all caller-saved registers from tracking to reflect callee clobbering.
    pub fn invalidate_volatiles<C: Convention>(&mut self, convention: &C) {
        let volatiles = convention.volatile_registers();
        self.tracked.retain(|reg, _| !volatiles.contains(reg));
        self.clean.retain(|reg| !volatiles.contains(reg));
    }

    /// Returns the first volatile register not currently holding a tracked value.
    pub fn free<'a>(&self, volatiles: impl Iterator<Item = &'a Register>) -> Option<Register> {
        volatiles.copied().find(|r| !self.tracked.contains_key(r))
    }

    /// Returns the first tracked volatile symbol matching the predicate, or the first volatile symbol if no match is found.
    pub fn evictable(
        &self,
        volatiles: &[Register],
        predicate: impl Fn(SymbolId) -> bool,
    ) -> (Register, SymbolId) {
        let candidates = self
            .tracked
            .iter()
            .filter_map(|(&reg, &val)| match val {
                Value::Symbol(sym) if volatiles.contains(&reg) => Some((reg, sym)),
                _ => None,
            })
            .collect::<Vec<(Register, SymbolId)>>();

        candidates
            .iter()
            .copied()
            .find(|&(_, sym)| predicate(sym))
            .or_else(|| candidates.first().copied())
            .unwrap()
    }

    pub fn tracked_value(&self, reg: Register) -> Option<Value> {
        self.tracked.get(&reg).copied()
    }

    pub fn tracked_register(&self, val: Value) -> Option<Register> {
        self.tracked
            .iter()
            .find(|(_, v)| **v == val)
            .map(|(&r, _)| r)
    }

    /// Resolves where a value currently lives, preferring live register state over the stack.
    pub fn locate(
        &self,
        val: Value,
        blobs: &Vec<CodeLabel>,
        slots: &BTreeMap<SymbolId, i32>,
    ) -> Operand {
        match val {
            Value::Constant(c) => Operand::Immediate(c),
            Value::String(id) => Operand::String(blobs[id]),
            Value::Symbol(s) => {
                if let Some(reg) = self.tracked_register(Value::Symbol(s)) {
                    return Operand::Register(reg);
                }
                if let Some(&offset) = slots.get(&s) {
                    assert!(
                        self.is_written(s),
                        "attempted to load symbol '{:?}' from stack slot before it was ever stored",
                        s
                    );
                    return Operand::Stack(offset);
                }
                panic!("symbol '{:?}' has no register nor a stack slot", s)
            }
            _ => unreachable!(),
        }
    }
}
