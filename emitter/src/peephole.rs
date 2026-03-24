use intermediate::{Function, Instruction, InstructionKind, Module, SymbolId, Value};

pub fn optimize(module: &mut Module) {
    for function in &mut module.functions {
        let mut optimizer = Peephole::new(function);

        optimizer.run();
    }
}

struct Peephole<'a> {
    function: &'a mut Function,
}

impl<'a> Peephole<'a> {
    fn new(function: &'a mut Function) -> Self {
        Self { function }
    }

    fn run(&mut self) {
        let mut i = 0;

        while i < self.function.instructions.len() {
            let consumed = self
                .try_call_assign(i)
                .or_else(|| self.try_compare_branch(i))
                .or_else(|| self.try_remove_unreachable(i))
                .or_else(|| self.try_remove_redundant_jump(i))
                .or_else(|| self.try_multiply_to_shift(i))
                .or_else(|| self.try_remove_dead_store(i));

            match consumed {
                Some(skip) => {
                    i += skip;
                }
                None => {
                    i += 1;
                }
            }
        }
    }

    /// Fuses a subroutine call directly into a destination symbol, bypassing an intermediate [`InstructionKind::Assign`].
    fn try_call_assign(&mut self, i: usize) -> Option<usize> {
        if i + 1 >= self.function.instructions.len() {
            return None;
        }

        let (current, next) = {
            let current = &self.function.instructions[i];
            let next = &self.function.instructions[i + 1];
            (current.clone(), next.clone())
        };

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
        ) = (current.kind, next.kind)
        {
            if tmp == s && !self.symbol_read_after(tmp, i + 2) {
                // modify in place
                self.function.instructions[i] = Instruction {
                    kind: InstructionKind::Call { dst, callee, args },
                    offset: current.offset,
                };
                self.function.instructions.remove(i + 1);
                return Some(1);
            }
        }

        None
    }

    /// Merges an [`InstructionKind::Eq`] or [`InstructionKind::NotEq`] comparison with its dependent [`InstructionKind::JumpIfFalse`] into a single branch.
    fn try_compare_branch(&mut self, i: usize) -> Option<usize> {
        if i + 1 >= self.function.instructions.len() {
            return None;
        }

        let (current, next) = {
            let current = &self.function.instructions[i];
            let next = &self.function.instructions[i + 1];
            (current.clone(), next.clone())
        };

        if let InstructionKind::JumpIfFalse {
            val: Value::Symbol(jif_condition),
            dst: jif_dst,
        } = next.kind
        {
            match current.kind {
                InstructionKind::Eq {
                    dst: eq_dst,
                    left: eq_left,
                    right: eq_right,
                } if eq_dst == jif_condition && !self.symbol_read_after(eq_dst, i + 2) => {
                    self.function.instructions[i] = Instruction {
                        kind: InstructionKind::JumpIfNotEq {
                            left: eq_left,
                            right: eq_right,
                            dst: jif_dst,
                        },
                        offset: current.offset,
                    };
                    self.function.instructions.remove(i + 1);
                    return Some(1);
                }
                InstructionKind::NotEq {
                    dst: neq_dst,
                    left: neq_left,
                    right: neq_right,
                } if neq_dst == jif_condition && !self.symbol_read_after(neq_dst, i + 2) => {
                    self.function.instructions[i] = Instruction {
                        kind: InstructionKind::JumpIfEq {
                            left: neq_left,
                            right: neq_right,
                            dst: jif_dst,
                        },
                        offset: current.offset,
                    };
                    self.function.instructions.remove(i + 1);
                    return Some(1);
                }
                _ => {}
            }
        }

        None
    }

    /// Discards code that follows an unconditional terminator like [`InstructionKind::Return`] or [`InstructionKind::Jump`].
    fn try_remove_unreachable(&mut self, i: usize) -> Option<usize> {
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
            self.function.instructions.remove(i);
            return Some(0); // current index now points to next element
        }

        None
    }

    /// Eliminates an unconditional [`InstructionKind::Jump`] whose target is the immediately following instruction.
    fn try_remove_redundant_jump(&mut self, i: usize) -> Option<usize> {
        let current = &self.function.instructions[i];

        if let InstructionKind::Jump(dst) = current.kind {
            let mut j = i + 1;
            while j < self.function.instructions.len() {
                match self.function.instructions[j].kind {
                    InstructionKind::Label(id) if id == dst => {
                        self.function.instructions.remove(i);
                        return Some(0);
                    }
                    InstructionKind::Label(_) => j += 1,
                    _ => break,
                }
            }
        }

        None
    }

    /// Replaces a [`InstructionKind::Mul`] by a power-of-two constant with a [`InstructionKind::Shl`].
    fn try_multiply_to_shift(&mut self, i: usize) -> Option<usize> {
        let current = &self.function.instructions[i];

        if let InstructionKind::Mul { dst, left, right } = current.kind {
            if let Value::Constant(n) = right {
                if n > 0 && n.count_ones() == 1 {
                    let shift = n.trailing_zeros() as i64;
                    self.function.instructions[i] = Instruction {
                        kind: InstructionKind::Shl {
                            dst,
                            left,
                            right: Value::Constant(shift),
                        },
                        offset: current.offset,
                    };
                    return Some(1);
                }
            }
        }

        None
    }

    /// Removes instructions that define a symbol that is overwritten before it is ever read.
    fn try_remove_dead_store(&mut self, i: usize) -> Option<usize> {
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
                self.function.instructions.remove(i);
                return Some(0);
            }
            if matches!(instruction.kind, InstructionKind::Label(_)) {
                return None;
            }
        }

        None
    }

    /// Scans subsequent instructions to determine if a [`SymbolId`] is read before being overwritten.
    fn symbol_read_after(&self, symbol: SymbolId, start: usize) -> bool {
        self.function.instructions[start..]
            .iter()
            .any(|i| i.kind.read_symbols().contains(&symbol))
    }
}
