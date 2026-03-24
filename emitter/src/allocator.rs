use crate::convention::Convention;
use intermediate::{Instruction, InstructionKind, SymbolId};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;

pub struct Liveness {
    ranges: BTreeMap<SymbolId, Range<usize>>,
}

impl Liveness {
    pub fn new(instructions: &[Instruction]) -> Self {
        let mut ranges = BTreeMap::new();
        let mut labels = HashMap::new();

        for (i, instruction) in instructions.iter().enumerate() {
            if let InstructionKind::Label(l) = instruction.kind {
                labels.insert(l, i);
            }
        }

        let mut edges = vec![Vec::new(); instructions.len()];

        for (i, instruction) in instructions.iter().enumerate() {
            for target in instruction.kind.targets() {
                if let Some(&t) = labels.get(&target) {
                    edges[i].push(t);
                }
            }
            if i + 1 < instructions.len() && !matches!(instruction.kind, InstructionKind::Jump(_)) {
                edges[i].push(i + 1);
            }
        }

        let mut inflows = vec![HashSet::new(); instructions.len()];
        let mut dirty = true;

        while dirty {
            dirty = false;

            for i in (0..instructions.len()).rev() {
                let mut aggregate = HashSet::new();

                for &e in &edges[i] {
                    aggregate.extend(inflows[e].iter().copied());
                }
                for w in instructions[i].kind.written_symbols() {
                    aggregate.remove(&w);
                }
                for r in instructions[i].kind.read_symbols() {
                    aggregate.insert(r);
                }
                if aggregate != inflows[i] {
                    inflows[i] = aggregate;
                    dirty = true;
                }
            }
        }

        for (i, set) in inflows.iter().enumerate() {
            for &s in set.iter() {
                ranges
                    .entry(s)
                    .and_modify(|r: &mut Range<usize>| r.end = i + 1)
                    .or_insert(i..(i + 1));
            }
        }

        Self { ranges }
    }

    /// Iterate over all live ranges.
    pub fn iter(&self) -> impl Iterator<Item = (&SymbolId, &Range<usize>)> {
        self.ranges.iter()
    }

    /// Whether the symbol is live at the given cursor position.
    pub fn is_live(&self, sym: SymbolId, cursor: usize) -> bool {
        self.ranges
            .get(&sym)
            .map_or(false, |range| range.start <= cursor && cursor < range.end)
    }

    /// Whether the symbol will be live after the given instruction.
    pub fn will_be_live(&self, sym: SymbolId, cursor: usize) -> bool {
        self.ranges
            .get(&sym)
            .map_or(false, |range| cursor + 1 < range.end)
    }

    /// Get the live range for a symbol.
    pub fn range(&self, sym: SymbolId) -> Option<&Range<usize>> {
        self.ranges.get(&sym)
    }
}

pub struct Allocator<'a, C: Convention> {
    convention: &'a C,
    offset: i32,
}

impl<'a, C: Convention> Allocator<'a, C> {
    pub fn new(convention: &'a C) -> Self {
        Self {
            convention,
            offset: convention.shadow_space() as i32,
        }
    }

    pub fn allocate_parameters(
        &mut self,
        slots: &mut BTreeMap<SymbolId, i32>,
        parameters: &[SymbolId],
    ) {
        for (i, &sym) in parameters.iter().enumerate() {
            if i < self.convention.argument_registers().len() {
                slots.insert(sym, self.offset);
                self.offset += 8;
            } else {
                let offset = -(16 + ((i - self.convention.argument_registers().len()) as i32 * 8));
                slots.insert(sym, offset);
            }
        }
    }

    pub fn allocate_symbols(
        &mut self,
        slots: &mut BTreeMap<SymbolId, i32>,
        instructions: &[Instruction],
        liveness: &Liveness,
    ) {
        // Find symbols that are both written and read
        let written_and_read = instructions
            .iter()
            .flat_map(|i| i.kind.written_symbols())
            .filter(|&sym| {
                instructions
                    .iter()
                    .any(|i| i.kind.read_symbols().contains(&sym))
            })
            .collect::<HashSet<SymbolId>>();

        for (&sym, range) in liveness.iter() {
            if slots.contains_key(&sym) {
                continue;
            }

            // Only allocate if the symbol is actually needed and has a non-trivial live range
            if written_and_read.contains(&sym) && range.start < range.end - 1 {
                slots.insert(sym, self.offset);
                self.offset += 8;
            }
        }
    }
}
