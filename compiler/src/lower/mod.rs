mod decl;
mod expr;
mod stmt;

use intermediate::symbols::Symbols;
use intermediate::{Context, Module, SymbolId};
use std::error::Error;

use crate::ast::Tree;

const ENTRY_POINT: &str = "main";

pub trait Generate {
    type Output;

    fn generate(
        self,
        ctx: &mut Context,
        scope: &mut Symbols,
        id: SymbolId,
    ) -> Result<Self::Output, Box<dyn Error>>;
}

pub fn generate(tree: Tree) -> Result<Module, Box<dyn Error>> {
    let mut ctx = Context::new();
    let mut root = Symbols::new(None);
    let mut functions = Vec::new();
    let mut entry = None;

    for (i, decl) in tree.decls.into_iter().enumerate() {
        let name = decl.name();
        let id = SymbolId(i);

        if name == ENTRY_POINT {
            entry = Some(functions.len());
        }

        let func = decl.generate(&mut ctx, &mut root, id)?;
        functions.push(func);
    }

    Ok(Module { functions, entry })
}
