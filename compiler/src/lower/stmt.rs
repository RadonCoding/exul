use intermediate::{Context, FunctionId, InstructionKind, Value, symbols::Symbols};

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
        id: FunctionId,
    ) -> Result<Self::Output, Box<dyn Error>> {
        match self.0.kind {
            StmtKind::Assignment { name, value } => {
                let value = value.generate(ctx, scope, id)?;
                let name = String::from_utf8_lossy(name).to_string();
                let dst = if let Some(Value::Symbol(dst)) = scope.resolve(&name) {
                    dst
                } else {
                    let dst = ctx.next_symbol();
                    scope.define(name, Value::Symbol(dst));
                    dst
                };
                ctx.emit(InstructionKind::Assign { dst, src: value }, self.0.position);
            }
            StmtKind::Return(expr) => {
                let value = expr.generate(ctx, scope, id)?;
                ctx.emit(InstructionKind::Return(value), self.0.position);
            }
            StmtKind::If {
                cond,
                consequent,
                alternate,
            } => {
                let cond = cond.generate(ctx, scope, id)?;
                let l1 = ctx.next_label();
                let l2 = ctx.next_label();

                ctx.emit(
                    InstructionKind::JumpIfFalse { cond, dst: l1 },
                    self.0.position,
                );
                for stmt in consequent {
                    stmt.generate(ctx, scope, id)?;
                }
                ctx.emit(InstructionKind::Jump(l2), self.0.position);
                ctx.emit(InstructionKind::Label(l1), self.0.position);

                if let Some(stmts) = alternate {
                    for stmt in stmts {
                        stmt.generate(ctx, scope, id)?;
                    }
                }

                ctx.emit(InstructionKind::Label(l2), self.0.position);
            }
            StmtKind::For {
                init,
                cond,
                step,
                body,
            } => {
                init.generate(ctx, scope, id)?;

                let loop_start = ctx.next_label();
                let loop_end = ctx.next_label();

                ctx.emit(InstructionKind::Label(loop_start), self.0.position);

                let cond = cond.generate(ctx, scope, id)?;
                ctx.emit(
                    InstructionKind::JumpIfFalse {
                        cond,
                        dst: loop_end,
                    },
                    self.0.position,
                );

                for stmt in body {
                    stmt.generate(ctx, scope, id)?;
                }

                step.generate(ctx, scope, id)?;

                ctx.emit(InstructionKind::Jump(loop_start), self.0.position);
                ctx.emit(InstructionKind::Label(loop_end), self.0.position);
            }
            StmtKind::Store {
                size,
                address,
                value,
            } => {
                let address = address.generate(ctx, scope, id)?;
                let value = value.generate(ctx, scope, id)?;
                ctx.emit(
                    InstructionKind::Store {
                        size: size.into(),
                        dst: address,
                        src: value,
                    },
                    self.0.position,
                );
            }
            StmtKind::Expression(expr) => {
                expr.generate(ctx, scope, id)?;
            }
        }

        Ok(())
    }
}
