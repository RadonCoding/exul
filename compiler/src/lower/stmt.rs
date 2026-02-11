use intermediate::{Context, InstructionKind, SymbolId, Value, symbols::Symbols};

use crate::{
    ast::stmt::{Stmt, StmtKind},
    lower::Generate,
};
use std::error::Error;

impl Generate for Stmt<'_> {
    type Output = ();

    fn generate(
        self,
        ctx: &mut Context,
        scope: &mut Symbols,
        id: SymbolId,
    ) -> Result<Self::Output, Box<dyn Error>> {
        let offset = self.0.offset;

        match self.0.kind {
            StmtKind::Assignment { name, value } => {
                let src = value.generate(ctx, scope, id)?;
                let name = String::from_utf8_lossy(name).to_string();

                let dst = if let Some(Value::Symbol(existing)) = scope.resolve(&name) {
                    existing
                } else {
                    let sym = ctx.next_symbol();
                    scope.define(name, Value::Symbol(sym));
                    sym
                };

                ctx.emit(InstructionKind::Assign { dst, src }, offset);
            }
            StmtKind::Return(expr) => {
                let val = expr.generate(ctx, scope, id)?;
                ctx.emit(InstructionKind::Return(val), offset);
            }
            StmtKind::If {
                cond,
                consequent,
                alternate,
            } => {
                let cond = cond.generate(ctx, scope, id)?;

                let l1 = ctx.next_label();
                let l2 = ctx.next_label();

                ctx.emit(InstructionKind::JumpIfFalse { cond, dst: l1 }, offset);

                for stmt in consequent {
                    stmt.generate(ctx, scope, id)?;
                }

                ctx.emit(InstructionKind::Jump(l2), offset);

                ctx.emit(InstructionKind::Label(l1), offset);

                if let Some(stmts) = alternate {
                    for stmt in stmts {
                        stmt.generate(ctx, scope, id)?;
                    }
                }

                ctx.emit(InstructionKind::Label(l2), offset);
            }
            StmtKind::Expression(expr) => {
                expr.generate(ctx, scope, id)?;
            }
        }
        Ok(())
    }
}
