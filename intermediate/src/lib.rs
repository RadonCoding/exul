pub mod symbols;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LabelId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Symbol(SymbolId),
    Constant(i64),
}

impl Value {
    fn symbols(&self) -> Vec<SymbolId> {
        match self {
            Value::Symbol(s) => vec![*s],
            Value::Constant(_) => vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub enum InstructionKind {
    Add {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    Eq {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    Assign {
        dst: SymbolId,
        src: Value,
    },
    Call {
        dst: SymbolId,
        callee: SymbolId,
        args: Vec<Value>,
    },
    Return(Value),
    Label(LabelId),
    JumpIfFalse {
        cond: Value,
        dst: LabelId,
    },
    JumpIfNotEq {
        left: Value,
        right: Value,
        dst: LabelId,
    },
    Jump(LabelId),
}

impl InstructionKind {
    pub fn read_symbols(&self) -> Vec<SymbolId> {
        let mut symbols = Vec::new();
        match self {
            InstructionKind::Assign { src, .. } => {
                symbols.extend(src.symbols());
            }
            InstructionKind::Add { left, right, .. }
            | InstructionKind::Eq { left, right, .. }
            | InstructionKind::JumpIfNotEq { left, right, .. } => {
                symbols.extend(left.symbols());
                symbols.extend(right.symbols());
            }
            InstructionKind::Call { args, .. } => {
                for arg in args {
                    symbols.extend(arg.symbols());
                }
            }
            InstructionKind::Return(val) | InstructionKind::JumpIfFalse { cond: val, .. } => {
                symbols.extend(val.symbols());
            }
            InstructionKind::Jump(_) | InstructionKind::Label(_) => {}
        }
        symbols
    }

    pub fn written_symbols(&self) -> Vec<SymbolId> {
        match self {
            InstructionKind::Assign { dst, .. }
            | InstructionKind::Add { dst, .. }
            | InstructionKind::Eq { dst, .. }
            | InstructionKind::Call { dst, .. } => vec![*dst],
            _ => vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub kind: InstructionKind,
    pub offset: usize,
}

#[derive(Debug)]
pub struct Function {
    pub id: SymbolId,
    pub instructions: Vec<Instruction>,
    pub params: usize,
    pub capacity: usize,
}

#[derive(Debug)]
pub struct Module {
    pub functions: Vec<Function>,
    pub entry: Option<usize>,
}

pub struct Context {
    pub instructions: Vec<Instruction>,
    pub symbols: usize,
    pub labels: usize,
}

impl Context {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            symbols: 0,
            labels: 0,
        }
    }

    pub fn reset(&mut self) {
        self.instructions.clear();
        self.symbols = 0;
        self.labels = 0;
    }

    pub fn next_symbol(&mut self) -> SymbolId {
        let id = SymbolId(self.symbols);
        self.symbols += 1;
        id
    }

    pub fn next_label(&mut self) -> LabelId {
        let id = LabelId(self.labels);
        self.labels += 1;
        id
    }

    pub fn emit(&mut self, kind: InstructionKind, offset: usize) {
        self.instructions.push(Instruction { kind, offset });
    }
}
