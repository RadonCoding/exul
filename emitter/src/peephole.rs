use intermediate::{Function, Instruction, InstructionKind, SymbolId, Value};

pub fn optimize(function: &mut Function) {
    let mut optimizer = Peephole::new(function);
    optimizer.run()
}

struct Peephole<'a> {
    function: &'a mut Function,
}

impl<'a> Peephole<'a> {
    fn new(function: &'a mut Function) -> Self {
        Self { function }
    }

    fn run(&mut self) {
        let optimized = self.optimize_instructions();
        self.function.instructions = optimized;
    }

    /// Iterates through the instruction stream to collapse redundant logic and eliminate dead code.
    fn optimize_instructions(&self) -> Vec<Instruction> {
        let mut result = Vec::new();
        let mut i = 0;

        while i < self.function.instructions.len() {
            let consumed = self
                .try_call_assign(&mut result, i)
                .or_else(|| self.try_compare_branch(&mut result, i))
                .or_else(|| self.try_remove_unreachable(&mut result, i))
                .or_else(|| self.try_remove_dead_store(&mut result, i));

            match consumed {
                Some(skip) => {
                    i += skip;
                }
                None => {
                    result.push(self.function.instructions[i].clone());
                    i += 1;
                }
            }
        }

        result
    }

    /// Removes instructions that define a symbol that is overwritten before it is ever read.
    fn try_remove_dead_store(&self, _result: &mut Vec<Instruction>, i: usize) -> Option<usize> {
        let current = &self.function.instructions[i];

        let written = current.kind.written_symbols();

        if written.len() != 1 {
            return None;
        }

        let target = written[0];

        for inst in self.function.instructions.iter().skip(i + 1) {
            if inst.kind.read_symbols().contains(&target) {
                return None;
            }
            if inst.kind.written_symbols().contains(&target) {
                return Some(1);
            }
            if matches!(inst.kind, InstructionKind::Label(_)) {
                return None;
            }
        }

        Some(1)
    }

    /// Fuses a subroutine call directly into a destination symbol, bypassing an intermediate [`InstructionKind::Assign`].
    fn try_call_assign(&self, result: &mut Vec<Instruction>, i: usize) -> Option<usize> {
        if i + 1 >= self.function.instructions.len() {
            return None;
        }

        let current = &self.function.instructions[i];
        let next = &self.function.instructions[i + 1];

        if let (
            InstructionKind::Call {
                dst: tmp,
                callee,
                args,
            },
            InstructionKind::Assign {
                dst,
                src: Value::Symbol(s),
            },
        ) = (&current.kind, &next.kind)
        {
            if tmp == s {
                if !self.symbol_read_after(*tmp, i + 2) {
                    result.push(Instruction {
                        kind: InstructionKind::Call {
                            dst: *dst,
                            callee: *callee,
                            args: args.clone(),
                        },
                        offset: current.offset,
                    });
                    return Some(2);
                }
            }
        }

        None
    }

    /// Merges an [`InstructionKind::Eq`] comparison with its dependent [`InstructionKind::JumpIfFalse`] into a single branch.
    fn try_compare_branch(&self, result: &mut Vec<Instruction>, i: usize) -> Option<usize> {
        if i + 1 >= self.function.instructions.len() {
            return None;
        }

        let current = &self.function.instructions[i];
        let next = &self.function.instructions[i + 1];

        if let (
            InstructionKind::Eq { dst, left, right },
            InstructionKind::JumpIfFalse {
                cond: Value::Symbol(s),
                dst: label,
            },
        ) = (&current.kind, &next.kind)
        {
            // Fuse if the branch condition is the result of the equality check.
            if dst == s && !self.symbol_read_after(*dst, i + 2) {
                result.push(Instruction {
                    kind: InstructionKind::JumpIfNotEq {
                        left: *left,
                        right: *right,
                        dst: *label,
                    },
                    offset: current.offset,
                });
                return Some(2);
            }
        }
        None
    }

    /// Discards code that follows an unconditional terminator like [`InstructionKind::Return`] or [`InstructionKind::Jump`].
    fn try_remove_unreachable(&self, _result: &mut Vec<Instruction>, i: usize) -> Option<usize> {
        if i == 0 || i >= self.function.instructions.len() {
            return None;
        }

        let previous = &self.function.instructions[i - 1];
        let current = &self.function.instructions[i];

        let is_terminator = matches!(
            previous.kind,
            InstructionKind::Return(_) | InstructionKind::Jump(_)
        );

        // If the block has terminated, subsequent instructions are dead until a new entry point is defined.
        if is_terminator && !matches!(current.kind, InstructionKind::Label(_)) {
            return Some(1);
        }

        None
    }

    /// Scans subsequent instructions to determine if a [`SymbolId`] is read before being overwritten.
    fn symbol_read_after(&self, symbol: SymbolId, start: usize) -> bool {
        self.function.instructions[start..]
            .iter()
            .any(|inst| inst.kind.read_symbols().contains(&symbol))
    }
}
