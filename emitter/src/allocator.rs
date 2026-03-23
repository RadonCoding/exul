use crate::convention::Convention;
use intermediate::{Instruction, SymbolId};
use std::collections::HashMap;

pub struct Allocator<'a, C: Convention> {
    instructions: &'a [Instruction],
    convention: &'a C,
    offset: i32,
}

impl<'a, C: Convention> Allocator<'a, C> {
    pub fn new(instructions: &'a [Instruction], convention: &'a C) -> Self {
        Self {
            instructions,
            convention,
            offset: convention.shadow_space() as i32,
        }
    }
    pub fn allocate_parameters(&mut self, slots: &mut HashMap<SymbolId, i32>, params: &[SymbolId]) {
        for &sym in params {
            slots.insert(sym, self.offset);
            self.offset += 8;
        }
    }

    pub fn allocate_symbols(&mut self, slots: &mut HashMap<SymbolId, i32>) {
        for instruction in self.instructions {
            for dst in instruction.kind.written_symbols() {
                if !slots.contains_key(&dst) {
                    slots.insert(dst, self.offset);
                    self.offset += 8;
                }
            }
        }
    }
}
