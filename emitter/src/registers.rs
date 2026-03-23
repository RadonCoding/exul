use crate::convention::Convention;
use iced_x86::{Register, code_asm::CodeLabel};
use intermediate::{SymbolId, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    Register(Register),
    Stack(i32),
    Immediate(i64),
    String(CodeLabel),
}

pub struct Registers {
    /// Tracks which value currently resides in which register.
    tracked: HashMap<Register, Value>,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            tracked: HashMap::new(),
        }
    }

    pub fn track(&mut self, reg: Register, val: Value) {
        if let Value::Symbol(s) = val {
            self.tracked.retain(|r, v| {
                if let Value::Symbol(id) = v {
                    *r == reg || *id != s
                } else {
                    true
                }
            });
        }
        self.tracked.insert(reg, val);
    }

    pub fn invalidate(&mut self) {
        self.tracked.clear();
    }

    /// Drops a single symbol from tracking when it is overwritten or dies.
    pub fn invalidate_symbol(&mut self, sym: SymbolId) {
        self.tracked.retain(|_, v| {
            if let Value::Symbol(s) = v {
                *s != sym
            } else {
                true
            }
        });
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
    pub fn evict(
        &mut self,
        volatiles: &[Register],
        slots: &HashMap<SymbolId, i32>,
    ) -> (Register, i32) {
        let (reg, slot) = self
            .tracked
            .iter()
            .find_map(|(r, v)| {
                if !volatiles.contains(r) {
                    return None;
                }

                if let Value::Symbol(s) = v {
                    slots.get(s).map(|slot| (*r, *slot))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| unreachable!());

        self.tracked.remove(&reg);

        (reg, slot)
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
        slots: &HashMap<SymbolId, i32>,
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
                    return Operand::Stack(offset);
                }
                unreachable!()
            }
            _ => unreachable!(),
        }
    }
}
