use std::collections::HashMap;

use crate::emitter::Emitter;
use crate::{convention::Convention, emitter::FunctionContext};
use intermediate::{InstructionKind, SymbolId};

impl<C: Convention> Emitter<C> {
    pub(crate) fn run_allocator(&mut self, ctx: &mut FunctionContext, params: usize) {
        let mut last_use = HashMap::new();

        let volatiles = self.convention.volatile_regs();

        // Perform liveness analysis to determine the last instruction index where each symbol is used.
        for (i, instruction) in ctx.instructions.iter().enumerate() {
            for s in instruction.kind.read_symbols() {
                last_use.insert(s, i);
            }
        }

        let mut next_slot = self.convention.shadow_space() as i32;
        let mut next_reg = 0;

        for i in 0..params {
            let sym = SymbolId(i);
            let death = *last_use.get(&sym).unwrap_or(&0);

            // Spill to stack if the symbol range crosses a function call.
            let survivor = ctx.instructions[0..=death]
                .iter()
                .any(|inst| matches!(inst.kind, InstructionKind::Call { .. }));

            if survivor {
                ctx.slots.insert(sym, next_slot);
                next_slot += 8;
            } else {
                // Use standard calling convention registers or rotate through volatiles.
                let reg = self
                    .convention
                    .argument_reg(i)
                    .unwrap_or_else(|| volatiles[next_reg % volatiles.len()]);
                ctx.allocs.insert(sym, reg);
                next_reg += 1;
            }
        }

        for (i, instruction) in ctx.instructions.iter().enumerate() {
            for d in instruction.kind.written_symbols() {
                // Force call results into the convention return register.
                if let InstructionKind::Call { dst, .. } = &instruction.kind {
                    if *dst == d {
                        ctx.allocs.insert(d, self.ret());
                        continue;
                    }
                }

                if ctx.allocs.contains_key(&d) || ctx.slots.contains_key(&d) {
                    continue;
                }

                let death = *last_use.get(&d).unwrap_or(&i);

                // Check if symbol needs to survive a call clobbering caller-saved regs.
                let survivor = ctx.instructions[i..=death]
                    .iter()
                    .any(|inst| matches!(inst.kind, InstructionKind::Call { .. }));

                if survivor {
                    ctx.slots.insert(d, next_slot);
                    next_slot += 8;
                } else {
                    // Simple round-robin allocation for short-lived registers.
                    ctx.allocs.insert(d, volatiles[next_reg % volatiles.len()]);
                    next_reg += 1;
                }
            }
        }
    }
}
