mod decl;
mod expr;
mod stmt;

use intermediate::symbols::Symbols;
use intermediate::{Builtin, Context, Function, FunctionId, InstructionKind, Module, Value};
use std::collections::HashSet;
use std::error::Error;

use crate::ast::{self, Tree};
use crate::lex;

const BOOTSTRAP: &str = "__bootstrap__";

const ENTRY_POINT: &str = "main";

pub trait Generate {
    type Output;
    fn generate(
        self,
        ctx: &mut Context,
        scope: &mut Symbols,
        id: FunctionId,
    ) -> Result<Self::Output, Box<dyn Error>>;
}

pub fn generate(tree: Tree) -> Result<Module, Box<dyn Error>> {
    let mut ctx = Context::new();
    let mut root = Symbols::new(None);

    // Register all builtins in scope but don't compile them yet.
    for builtin in Builtin::all() {
        root.define(builtin.name().to_string(), Value::Function(builtin.id()));
    }

    // Pre-register all user functions before lowering so forward calls resolve.
    let mut entry = None;
    let mut pending = Vec::new();
    for decl in tree.decls.iter() {
        let name = decl.name();
        let id = ctx.next_function();
        if name == ENTRY_POINT {
            entry = Some(id);
        }
        root.define(name, Value::Function(id));
        pending.push(id);
    }

    let mut functions = Vec::new();

    for (decl, id) in tree.decls.into_iter().zip(pending) {
        functions.push(decl.generate(&mut ctx, &mut root, id)?);
    }

    if !ctx.imports.is_empty() {
        let stub = bootstrap(&mut ctx, entry);
        functions.insert(0, stub);
    }

    let mut compiled = HashSet::new();

    // Compile standard library on demand - repeat until no new ones are referenced.
    loop {
        let required = Builtin::all()
            .iter()
            .filter(|b| {
                !compiled.contains(&b.id())
                    && functions.iter().any(|f| {
                        f.instructions
                            .iter()
                            .any(|i| i.kind.called().contains(&b.id()))
                    })
            })
            .copied()
            .collect::<Vec<_>>();

        if required.is_empty() {
            break;
        }

        for builtin in required {
            compiled.insert(builtin.id());
            let tokens = lex::tokenize(builtin.source())?;
            let tree = ast::parse(tokens)?;

            // Pre-register all functions in this source file before lowering any of them.
            let mut pending = Vec::new();
            for decl in tree.decls.iter() {
                let name = decl.name();
                let id = if name == builtin.name() {
                    builtin.id()
                } else {
                    let id = ctx.next_function();
                    root.define(name, Value::Function(id));
                    id
                };
                pending.push(id);
            }

            for (decl, id) in tree.decls.into_iter().zip(pending) {
                functions.push(decl.generate(&mut ctx, &mut root, id)?);
            }
        }
    }

    let imports = ctx.imports.clone();

    let entry = if !imports.is_empty() {
        Some(0)
    } else {
        functions.iter().position(|f| Some(f.id) == entry)
    };

    Ok(Module {
        entry,
        imports,
        strings: ctx.strings,
        functions,
    })
}

fn bootstrap(ctx: &mut Context, entry: Option<FunctionId>) -> Function {
    let id = ctx.next_function();

    ctx.reset();

    for import in ctx.imports.clone() {
        let module = Value::String(ctx.string(import.module.clone()));
        let function = Value::String(ctx.string(import.function.clone()));
        let result = ctx.next_symbol();
        ctx.emit(
            InstructionKind::Call {
                dst: result,
                callee: Builtin::Resolve.id(),
                args: vec![module, function],
            },
            0,
        );
        ctx.emit(
            InstructionKind::Import {
                import: import.id,
                src: Value::Symbol(result),
            },
            0,
        );
    }

    if let Some(entry) = entry {
        let dst = ctx.next_symbol();
        ctx.emit(
            InstructionKind::Call {
                dst,
                callee: entry,
                args: vec![],
            },
            0,
        );
    }

    ctx.emit(InstructionKind::Return(Value::Constant(0)), 0);

    Function {
        id,
        name: BOOTSTRAP.to_string(),
        instructions: ctx.instructions.clone(),
        params: vec![],
        capacity: ctx.symbols,
    }
}
