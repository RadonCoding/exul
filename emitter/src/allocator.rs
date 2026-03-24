use crate::convention::Convention;
use intermediate::{Instruction, InstructionKind, SymbolId};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;

pub struct Allocator {
    offset: i32,
}

pub fn compute_live_ranges(instructions: &[Instruction]) -> HashMap<SymbolId, Range<usize>> {
    let mut ranges = HashMap::new();
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

    ranges
}

impl Allocator {
    pub fn new<C: Convention>(convention: &C) -> Self {
        Self {
            offset: convention.shadow_space() as i32,
        }
    }

    pub fn allocate_parameters(
        &mut self,
        slots: &mut BTreeMap<SymbolId, i32>,
        params: &[SymbolId],
    ) {
        for &sym in params {
            slots.insert(sym, self.offset);
            self.offset += 8;
        }
    }

    pub fn allocate_symbols(
        &mut self,
        slots: &mut BTreeMap<SymbolId, i32>,
        instructions: &[Instruction],
    ) {
        let ranges = compute_live_ranges(instructions);
        let written = instructions
            .iter()
            .flat_map(|i| i.kind.written_symbols())
            .collect::<HashSet<SymbolId>>();

        for (&sym, range) in ranges.iter() {
            if slots.contains_key(&sym) {
                continue;
            }
            if written.contains(&sym) && range.start < range.end {
                slots.insert(sym, self.offset);
                self.offset += 8;
            }
        }
    }
}
