use crate::convention::Convention;
use crate::emitter::Emitter;
use intermediate::{Function, InstructionKind, Value};
use std::collections::HashMap;

impl<C: Convention> Emitter<C> {
    pub(crate) fn run_allocator(&mut self, func: &Function) {
        self.allocs.clear();
        let mut deaths = HashMap::new();
        let volatiles = self.convention.volatile_regs();
        let mut next = 0;

        for (i, instr) in func.instructions.iter().enumerate() {
            match &instr.kind {
                InstructionKind::Add { left, right, .. }
                | InstructionKind::Eq { left, right, .. } => {
                    if let Value::Symbol(s) = left {
                        deaths.insert(*s, i);
                    }
                    if let Value::Symbol(s) = right {
                        deaths.insert(*s, i);
                    }
                }
                InstructionKind::Assign { src, .. }
                | InstructionKind::JumpIfFalse { cond: src, .. } => {
                    if let Value::Symbol(s) = src {
                        deaths.insert(*s, i);
                    }
                }
                InstructionKind::Return(Value::Symbol(s)) => {
                    deaths.insert(*s, i);
                }
                _ => {}
            }
        }

        for (i, instruction) in func.instructions.iter().enumerate() {
            let (dst, left) = match &instruction.kind {
                InstructionKind::Add { dst, left, .. } | InstructionKind::Eq { dst, left, .. } => {
                    (Some(*dst), Some(left))
                }
                InstructionKind::Assign { dst, src } => (Some(*dst), Some(src)),
                _ => (None, None),
            };

            if let Some(d) = dst {
                if let Some(Value::Symbol(s)) = left {
                    if deaths.get(&s) == Some(&i) {
                        let r = self.allocs[&s];
                        self.allocs.insert(d, r);
                        continue;
                    }
                }
                self.allocs.insert(d, volatiles[next % volatiles.len()]);
                next += 1;
            }
        }
    }
}
