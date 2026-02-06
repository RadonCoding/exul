use intermediate::{Context, Function, SymbolId, Value, symbols::Symbols};

use crate::{
    ast::decl::{Decl, DeclKind, FunctionDecl},
    lower::Generate,
};
use std::error::Error;

impl<'a> Decl<'a> {
    pub fn name(&self) -> String {
        match &self.0.kind {
            DeclKind::Function(f) => String::from_utf8_lossy(f.name).to_string(),
        }
    }
}

impl Generate for Decl<'_> {
    type Output = Function;

    fn generate(
        self,
        ctx: &mut Context,
        scope: &mut Symbols,
        id: SymbolId,
    ) -> Result<Self::Output, Box<dyn Error>> {
        match self.0.kind {
            DeclKind::Function(func) => func.generate(ctx, scope, id),
        }
    }
}

impl Generate for FunctionDecl<'_> {
    type Output = Function;

    fn generate(
        self,
        ctx: &mut Context,
        parent: &mut Symbols,
        id: SymbolId,
    ) -> Result<Self::Output, Box<dyn Error>> {
        let name = String::from_utf8_lossy(self.name).to_string();

        parent.define(name, Value::Symbol(id));

        ctx.reset();

        let mut scope = Symbols::new(Some(parent));

        for stmt in self.body {
            stmt.generate(ctx, &mut scope, id)?;
        }

        Ok(Function {
            id,
            instructions: ctx.instructions.clone(),
            capacity: ctx.symbols,
        })
    }
}
