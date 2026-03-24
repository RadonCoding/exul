pub mod symbols;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FunctionId(pub usize);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SymbolId(pub usize);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LabelId(pub usize);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Value {
    Function(FunctionId),
    Symbol(SymbolId),
    Constant(i64),
    String(usize),
}

impl Value {
    fn symbols(&self) -> Vec<SymbolId> {
        match self {
            Value::Symbol(s) => vec![*s],
            _ => vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Memory {
    Byte,
    Word,
    Dword,
    Qword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    Gs,
    Fs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Builtin {
    Resolve,
    Strlen,
    Print,
}

impl Builtin {
    pub fn all() -> &'static [Builtin] {
        &[Builtin::Resolve, Builtin::Strlen, Builtin::Print]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Builtin::Resolve => "__resolve__",
            Builtin::Strlen => "strlen",
            Builtin::Print => "print",
        }
    }

    pub fn source(&self) -> &'static [u8] {
        match self {
            Builtin::Resolve => include_bytes!("../../stdlib/resolve.exl"),
            Builtin::Strlen => include_bytes!("../../stdlib/strlen.exl"),
            Builtin::Print => include_bytes!("../../stdlib/print.exl"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Import {
    pub id: FunctionId,
    pub module: String,
    pub function: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionKind {
    Eq {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    NotEq {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    Lte {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    Gte {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    Lt {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    Gt {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    Add {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    Sub {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    Mul {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    And {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    Or {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    Xor {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    Shl {
        dst: SymbolId,
        left: Value,
        right: Value,
    },
    Shr {
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
        callee: FunctionId,
        args: Vec<Value>,
    },
    Load {
        dst: SymbolId,
        size: Memory,
        src: Value,
    },
    Store {
        size: Memory,
        dst: Value,
        src: Value,
    },
    Import {
        import: FunctionId,
        src: Value,
    },
    Segment {
        dst: SymbolId,
        seg: Segment,
        offset: Value,
    },
    Return(Value),
    Label(LabelId),
    JumpIfFalse {
        val: Value,
        dst: LabelId,
    },
    JumpIfEq {
        left: Value,
        right: Value,
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
    /// Whether the instruction has observable side effects beyond producing a value.
    pub fn has_effects(&self) -> bool {
        matches!(
            self,
            InstructionKind::Call { .. }
                | InstructionKind::Store { .. }
                | InstructionKind::Return(_)
                | InstructionKind::Jump(_)
                | InstructionKind::JumpIfFalse { .. }
                | InstructionKind::JumpIfEq { .. }
                | InstructionKind::JumpIfNotEq { .. }
                | InstructionKind::Label(_)
        )
    }

    /// Symbols that are read by this instruction.
    pub fn read_symbols(&self) -> Vec<SymbolId> {
        let mut symbols = Vec::new();
        match self {
            InstructionKind::Eq { left, right, .. }
            | InstructionKind::NotEq { left, right, .. }
            | InstructionKind::Lte { left, right, .. }
            | InstructionKind::Gte { left, right, .. }
            | InstructionKind::Lt { left, right, .. }
            | InstructionKind::Gt { left, right, .. }
            | InstructionKind::Add { left, right, .. }
            | InstructionKind::Sub { left, right, .. }
            | InstructionKind::Mul { left, right, .. }
            | InstructionKind::And { left, right, .. }
            | InstructionKind::Or { left, right, .. }
            | InstructionKind::Xor { left, right, .. }
            | InstructionKind::Shl { left, right, .. }
            | InstructionKind::Shr { left, right, .. }
            | InstructionKind::JumpIfEq { left, right, .. }
            | InstructionKind::JumpIfNotEq { left, right, .. } => {
                symbols.extend(left.symbols());
                symbols.extend(right.symbols());
            }
            InstructionKind::Assign { src, .. } => {
                symbols.extend(src.symbols());
            }
            InstructionKind::Call { args, .. } => {
                for arg in args {
                    symbols.extend(arg.symbols());
                }
            }
            InstructionKind::Return(val) | InstructionKind::JumpIfFalse { val, .. } => {
                symbols.extend(val.symbols());
            }
            InstructionKind::Load { src, .. } => {
                symbols.extend(src.symbols());
            }
            InstructionKind::Store { dst, src, .. } => {
                symbols.extend(dst.symbols());
                symbols.extend(src.symbols());
            }
            InstructionKind::Import { src, .. } => {
                symbols.extend(src.symbols());
            }
            InstructionKind::Segment { offset, .. } => {
                symbols.extend(offset.symbols());
            }
            InstructionKind::Jump(_) | InstructionKind::Label(_) => {}
        }
        symbols
    }

    /// Symbols that are written by this instruction.
    pub fn written_symbols(&self) -> Vec<SymbolId> {
        match self {
            InstructionKind::Eq { dst, .. }
            | InstructionKind::NotEq { dst, .. }
            | InstructionKind::Lte { dst, .. }
            | InstructionKind::Gte { dst, .. }
            | InstructionKind::Lt { dst, .. }
            | InstructionKind::Gt { dst, .. }
            | InstructionKind::Add { dst, .. }
            | InstructionKind::Sub { dst, .. }
            | InstructionKind::Mul { dst, .. }
            | InstructionKind::And { dst, .. }
            | InstructionKind::Or { dst, .. }
            | InstructionKind::Xor { dst, .. }
            | InstructionKind::Shl { dst, .. }
            | InstructionKind::Shr { dst, .. }
            | InstructionKind::Assign { dst, .. }
            | InstructionKind::Call { dst, .. }
            | InstructionKind::Load { dst, .. }
            | InstructionKind::Segment { dst, .. } => vec![*dst],
            _ => vec![],
        }
    }

    /// Functions called by this instruction.
    pub fn called(&self) -> Vec<FunctionId> {
        match self {
            InstructionKind::Call { callee, .. } => vec![*callee],
            _ => vec![],
        }
    }

    /// Labels this instruction can jump to.
    pub fn targets(&self) -> Vec<LabelId> {
        match self {
            InstructionKind::Jump(l) => vec![*l],
            InstructionKind::JumpIfFalse { dst, .. }
            | InstructionKind::JumpIfEq { dst, .. }
            | InstructionKind::JumpIfNotEq { dst, .. } => vec![*dst],
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
    pub id: FunctionId,
    pub name: String,
    pub instructions: Vec<Instruction>,
    pub params: Vec<SymbolId>,
    pub capacity: usize,
}

#[derive(Debug)]
pub struct Module {
    pub entry: Option<usize>,
    pub strings: Vec<String>,
    pub imports: Vec<Import>,
    pub functions: Vec<Function>,
}

pub struct Context {
    pub instructions: Vec<Instruction>,
    pub strings: Vec<String>,
    pub imports: Vec<Import>,
    pub loops: Vec<LabelId>,
    pub functions: usize,
    pub symbols: usize,
    pub labels: usize,
}

impl Context {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            imports: Vec::new(),
            strings: Vec::new(),
            loops: Vec::new(),
            functions: 0,
            symbols: 0,
            labels: 0,
        }
    }

    pub fn reset(&mut self) {
        self.instructions.clear();
        self.symbols = 0;
        self.labels = 0;
    }

    pub fn next_function(&mut self) -> FunctionId {
        let id = FunctionId(self.functions);
        self.functions += 1;
        id
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

    pub fn string(&mut self, s: String) -> usize {
        if let Some(i) = self.strings.iter().position(|x| x == &s) {
            return i;
        }
        let i = self.strings.len();
        self.strings.push(s);
        i
    }
}
