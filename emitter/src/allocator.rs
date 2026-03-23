use crate::convention::Convention;
use intermediate::{Instruction, InstructionKind, SymbolId};
use std::collections::HashMap;
use std::ops::Range;

pub struct Allocator {
    offset: i32,
}

fn compute_live_ranges(instructions: &[Instruction]) -> HashMap<SymbolId, Range<usize>> {
    let mut ranges = HashMap::new();
    let mut labels = HashMap::new();

    for (i, instruction) in instructions.iter().enumerate() {
        if let InstructionKind::Label(id) = &instruction.kind {
            labels.insert(*id, i);
        }

        for sym in instruction.kind.written_symbols() {
            let r = ranges.entry(sym).or_insert(i..i);
            if i >= r.end {
                r.end = i;
            }
        }

        for sym in instruction.kind.read_symbols() {
            if let Some(r) = ranges.get_mut(&sym) {
                if i >= r.end {
                    r.end = i;
                }
            }
        }
    }

    for (jump, instruction) in instructions.iter().enumerate() {
        let target = match &instruction.kind {
            InstructionKind::Jump(id) => id,
            _ => continue,
        };

        if let Some(&target) = labels.get(target) {
            if target < jump {
                for r in ranges.values_mut() {
                    if r.end >= target && r.end <= jump && r.start <= jump {
                        r.end = jump;
                    }
                }
            }
        }
    }

    ranges
}

impl Allocator {
    pub fn new<C: Convention>(convention: &C) -> Self {
        Self {
            offset: convention.shadow_space() as i32,
        }
    }

    pub fn allocate_parameters(&mut self, slots: &mut HashMap<SymbolId, i32>, params: &[SymbolId]) {
        for &sym in params {
            slots.insert(sym, self.offset);
            self.offset += 8;
        }
    }

    pub fn allocate_symbols(
        &mut self,
        slots: &mut HashMap<SymbolId, i32>,
        instructions: &[Instruction],
    ) {
        let ranges = compute_live_ranges(&instructions);
        let mut ordered = ranges
            .iter()
            .filter(|(s, _)| !slots.contains_key(s))
            .collect::<Vec<(&SymbolId, &Range<usize>)>>();

        ordered.sort_by_key(|(_, r)| r.start);

        let mut pool = Vec::new();

        for (&sym, range) in ordered {
            if let Some(entry) = pool
                .iter_mut()
                .find(|(_, last_use)| *last_use < range.start)
            {
                slots.insert(sym, entry.0);
                entry.1 = range.end;
            } else {
                slots.insert(sym, self.offset);
                pool.push((self.offset, range.end));
                self.offset += 8;
            }
        }
    }
}
