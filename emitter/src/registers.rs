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
    dirty: HashSet<SymbolId>,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            tracked: BTreeMap::new(),
            dirty: HashSet::new(),
        }
    }

    pub fn iter(&self) -> btree_map::Iter<'_, Register, Value> {
        self.tracked.iter()
    }

    pub fn is_dirty(&self, sym: SymbolId) -> bool {
        self.dirty.contains(&sym)
    }

    /// Records that [`SymbolId`]'s stack slot has been physically written to.
    pub fn set_dirty(&mut self, sym: SymbolId) {
        self.dirty.insert(sym);
    }

    /// Tracks a value in a register, replacing any previous location of the same symbol.
    pub fn track(&mut self, reg: Register, val: Value) {
        if let Value::Symbol(s) = val {
            self.tracked.retain(|_, v| *v != Value::Symbol(s));
        }
        self.tracked.insert(reg, val);
    }

    pub fn invalidate(&mut self) {
        self.tracked.clear();
    }

    /// Drops a specific register from tracking when it is about to be clobbered.
    pub fn invalidate_register(&mut self, reg: Register) {
        self.tracked.remove(&reg);
    }

    /// Drops all caller-saved registers from tracking to reflect callee clobbering.
    pub fn invalidate_volatiles<C: Convention>(&mut self, convention: &C) {
        let volatiles = convention.volatile_registers();
        self.tracked.retain(|reg, _| !volatiles.contains(reg));
    }

    /// Returns the first volatile register not currently holding a tracked value.
    pub fn free<'a>(&self, volatiles: impl Iterator<Item = &'a Register>) -> Option<Register> {
        volatiles.copied().find(|r| !self.tracked.contains_key(r))
    }

    /// Picks a volatile symbol to evict, removes it from tracking, and returns its register and stack offset.
    pub fn evict(&mut self, volatiles: &[Register]) -> (Register, SymbolId) {
        let (reg, sym) = self
            .tracked
            .iter()
            .find_map(|(r, v)| {
                if !volatiles.contains(r) {
                    return None;
                }
                if let Value::Symbol(s) = v {
                    Some((*r, *s))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| unreachable!());

        self.tracked.remove(&reg);

        (reg, sym)
    }

    pub fn get_register_for_value(&self, val: Value) -> Option<Register> {
        self.tracked
            .iter()
            .find(|(_, v)| **v == val)
            .map(|(&r, _)| r)
    }

    /// Resolves where a value currently lives, preferring live register state over the stack.
    pub fn locate(
        &self,
        val: Value,
        slots: &BTreeMap<SymbolId, i32>,
        data: &[CodeLabel],
    ) -> Operand {
        match val {
            Value::Constant(c) => Operand::Immediate(c),
            Value::String(id) => Operand::String(data[id]),
            Value::Symbol(s) => {
                if let Some(reg) = self.get_register_for_value(Value::Symbol(s)) {
                    return Operand::Register(reg);
                }
                if let Some(&offset) = slots.get(&s) {
                    assert!(
                        self.is_dirty(s),
                        "attempted to load '{:?}' from stack slot before it was ever stored",
                        s
                    );
                    return Operand::Stack(offset);
                }
                unreachable!("symbol {:?} has no register nor a stack slot", s)
            }
            _ => unreachable!(),
        }
    }
}
