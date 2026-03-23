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
        self.function.instructions = self.optimize_instructions();
    }

    fn optimize_instructions(&self) -> Vec<Instruction> {
        let mut result = Vec::new();
        let mut i = 0;

        while i < self.function.instructions.len() {
            let consumed = self
                .try_call_assign(&mut result, i)
                .or_else(|| self.try_compare_branch(&mut result, i))
                .or_else(|| self.try_remove_unreachable(&mut result, i))
                .or_else(|| self.try_remove_redundant_jump(&mut result, i))
                .or_else(|| self.try_multiply_to_shift(&mut result, i))
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

    /// Merges an [`InstructionKind::Eq`] or [`InstructionKind::NotEq`] comparison with its dependent [`InstructionKind::JumpIfFalse`] into a single branch.
    fn try_compare_branch(&self, result: &mut Vec<Instruction>, i: usize) -> Option<usize> {
        if i + 1 >= self.function.instructions.len() {
            return None;
        }

        let current = &self.function.instructions[i];
        let next = &self.function.instructions[i + 1];

        if let InstructionKind::JumpIfFalse {
            cond: Value::Symbol(s),
            dst: label,
        } = &next.kind
        {
            match &current.kind {
                InstructionKind::Eq { dst, left, right }
                    if dst == s && !self.symbol_read_after(*dst, i + 2) =>
                {
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
                InstructionKind::NotEq { dst, left, right }
                    if dst == s && !self.symbol_read_after(*dst, i + 2) =>
                {
                    result.push(Instruction {
                        kind: InstructionKind::JumpIfEq {
                            left: *left,
                            right: *right,
                            dst: *label,
                        },
                        offset: current.offset,
                    });
                    return Some(2);
                }
                _ => {}
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

    /// Eliminates an unconditional [`InstructionKind::Jump`] whose target is the immediately following instruction.
    fn try_remove_redundant_jump(&self, _result: &mut Vec<Instruction>, i: usize) -> Option<usize> {
        let current = &self.function.instructions[i];

        if let InstructionKind::Jump(dst) = &current.kind {
            let mut j = i + 1;
            while j < self.function.instructions.len() {
                match &self.function.instructions[j].kind {
                    InstructionKind::Label(id) if id == dst => return Some(1),
                    InstructionKind::Label(_) => j += 1,
                    _ => break,
                }
            }
        }

        None
    }

    /// Replaces a [`InstructionKind::Mul`] by a power-of-two constant with a [`InstructionKind::Shl`].
    fn try_multiply_to_shift(&self, result: &mut Vec<Instruction>, i: usize) -> Option<usize> {
        let current = &self.function.instructions[i];

        if let InstructionKind::Mul { dst, left, right } = &current.kind {
            let shift = match right {
                Value::Constant(n) if *n > 0 && n.count_ones() == 1 => n.trailing_zeros() as i64,
                _ => return None,
            };

            result.push(Instruction {
                kind: InstructionKind::Shl {
                    dst: *dst,
                    left: *left,
                    right: Value::Constant(shift),
                },
                offset: current.offset,
            });

            return Some(1);
        }

        None
    }

    /// Removes instructions that define a symbol that is overwritten before it is ever read.
    fn try_remove_dead_store(&self, _result: &mut Vec<Instruction>, i: usize) -> Option<usize> {
        let current = &self.function.instructions[i];

        // Skip instructions that have side-effects and are not simple stores.
        if matches!(current.kind, InstructionKind::Call { .. }) {
            return None;
        }

        let written = current.kind.written_symbols();

        if written.len() != 1 {
            return None;
        }

        let target = written[0];

        for instruction in self.function.instructions.iter().skip(i + 1) {
            if instruction.kind.read_symbols().contains(&target) {
                return None;
            }
            if instruction.kind.written_symbols().contains(&target) {
                return Some(1);
            }
            if matches!(instruction.kind, InstructionKind::Label(_)) {
                return None;
            }
        }

        Some(1)
    }

    /// Scans subsequent instructions to determine if a [`SymbolId`] is read before being overwritten.
    fn symbol_read_after(&self, symbol: SymbolId, start: usize) -> bool {
        self.function.instructions[start..]
            .iter()
            .any(|i| i.kind.read_symbols().contains(&symbol))
    }
}
