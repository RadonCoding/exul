mod decl;
mod expr;
mod stmt;

use intermediate::symbols::Symbols;
use intermediate::{Builtin, Context, Function, FunctionId, InstructionKind, Module, Value};
use std::collections::{HashMap, HashSet};
use std::error::Error;

use crate::ast::{self, Tree};
use crate::lex;

const BOOTSTRAP: &str = "__bootstrap__";
const ENTRY_POINT: &str = "main";

pub fn generate(tree: Tree) -> Result<Module, Box<dyn Error>> {
    Lowerer::new().generate(tree)
}

pub trait Generate {
    type Output;
    fn generate(
        self,
        ctx: &mut Context,
        scope: &mut Symbols,
        id: FunctionId,
    ) -> Result<Self::Output, Box<dyn Error>>;
}

struct Lowerer {
    ctx: Context,
    builtins: HashMap<Builtin, FunctionId>,
    compiled: HashSet<FunctionId>,
    functions: Vec<Function>,
}

impl Lowerer {
    pub fn new() -> Self {
        let mut ctx = Context::new();
        let mut builtins = HashMap::new();

        for builtin in Builtin::all() {
            let id = ctx.next_function();
            builtins.insert(*builtin, id);
        }

        Self {
            ctx,
            builtins,
            compiled: HashSet::new(),
            functions: Vec::new(),
        }
    }

    fn compile_builtin(
        &mut self,
        builtin: Builtin,
        root: &mut Symbols,
    ) -> Result<(), Box<dyn Error>> {
        let id = self.builtins[&builtin];

        let tokens = lex::tokenize(builtin.source())?;
        let tree = ast::parse(tokens)?;

        let mut pending = Vec::new();

        for decl in tree.decls.iter() {
            let name = decl.name();
            let id = if name == builtin.name() {
                id
            } else {
                let id = self.ctx.next_function();
                root.define(name, Value::Function(id));
                id
            };
            pending.push(id);
        }

        for (decl, id) in tree.decls.into_iter().zip(pending) {
            let function = decl.generate(&mut self.ctx, root, id)?;
            self.functions.push(function);
        }

        Ok(())
    }

    pub fn generate(mut self, tree: Tree) -> Result<Module, Box<dyn Error>> {
        let mut root = Symbols::new(None);

        // Register all builtins in scope
        for builtin in Builtin::all() {
            let id = self.builtins[&builtin];
            root.define(builtin.name().to_string(), Value::Function(id));
        }

        // Pre-register all user functions before lowering so forward calls resolve.
        let mut entry = None;
        let mut pending = Vec::new();

        for decl in tree.decls.iter() {
            let name = decl.name();
            let id = self.ctx.next_function();
            if name == ENTRY_POINT {
                entry = Some(id);
            }
            root.define(name, Value::Function(id));
            pending.push(id);
        }

        for (decl, id) in tree.decls.into_iter().zip(pending) {
            let function = decl.generate(&mut self.ctx, &mut root, id)?;
            self.functions.push(function);
        }

        // Compile standard library on demand
        loop {
            let required = Builtin::all()
                .iter()
                .filter(|b| {
                    let id = self.builtins[b];
                    !self.compiled.contains(&id)
                        && self
                            .functions
                            .iter()
                            .any(|f| f.instructions.iter().any(|i| i.kind.called().contains(&id)))
                })
                .copied()
                .collect::<Vec<Builtin>>();

            if required.is_empty() {
                break;
            }

            for builtin in required {
                let id = self.builtins[&builtin];
                self.compiled.insert(id);
                self.compile_builtin(builtin, &mut root)?;
            }
        }

        let imports = self.ctx.imports.clone();

        if !imports.is_empty() {
            if !self.compiled.contains(&self.builtins[&Builtin::Resolve]) {
                self.compile_builtin(Builtin::Resolve, &mut root)?;
            }
            let stub = self.bootstrap(entry);
            self.functions.insert(0, stub);
        }

        let entry = if !imports.is_empty() {
            Some(0)
        } else {
            self.functions.iter().position(|f| Some(f.id) == entry)
        };

        Ok(Module {
            entry,
            imports,
            strings: self.ctx.strings,
            functions: self.functions,
        })
    }

    fn bootstrap(&mut self, entry: Option<FunctionId>) -> Function {
        let id = self.ctx.next_function();

        self.ctx.reset();

        for import in self.ctx.imports.clone() {
            let module = Value::String(self.ctx.string(import.module.clone()));
            let function = Value::String(self.ctx.string(import.function.clone()));
            let result = self.ctx.next_symbol();
            self.ctx.emit(
                InstructionKind::Call {
                    dst: result,
                    callee: self.builtins[&Builtin::Resolve],
                    args: vec![module, function],
                },
                0,
            );
            self.ctx.emit(
                InstructionKind::Import {
                    import: import.id,
                    src: Value::Symbol(result),
                },
                0,
            );
        }

        if let Some(entry) = entry {
            let dst = self.ctx.next_symbol();
            self.ctx.emit(
                InstructionKind::Call {
                    dst,
                    callee: entry,
                    args: vec![],
                },
                0,
            );
        }

        self.ctx
            .emit(InstructionKind::Return(Value::Constant(0)), 0);

        Function {
            id,
            name: BOOTSTRAP.to_string(),
            instructions: self.ctx.instructions.clone(),
            params: vec![],
            capacity: self.ctx.symbols,
        }
    }
}
