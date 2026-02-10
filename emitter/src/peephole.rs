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

    fn optimize_instructions(&self) -> Vec<Instruction> {
        let mut result = Vec::new();
        let mut i = 0;

        while i < self.function.instructions.len() {
            let consumed = self
                .try_call_assign(&mut result, i)
                .or_else(|| self.try_compare_branch(&mut result, i))
                .or_else(|| self.try_remove_unreachable(&mut result, i));

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

        if is_terminator && !matches!(current.kind, InstructionKind::Label(_)) {
            return Some(1);
        }

        None
    }

    fn symbol_read_after(&self, symbol: SymbolId, start: usize) -> bool {
        self.function.instructions[start..]
            .iter()
            .any(|inst| inst.kind.read_symbols().contains(&symbol))
    }
}
