use intermediate::{Context, InstructionKind, SymbolId, Value, symbols::Symbols};

use crate::{
    ast::expr::{Expr, ExprKind, Op},
    lower::Generate,
};
use std::error::Error;

impl Generate for Expr<'_> {
    type Output = Value;

    fn generate(
        self,
        ctx: &mut Context,
        scope: &mut Symbols,
        id: SymbolId,
    ) -> Result<Self::Output, Box<dyn Error>> {
        let offset = self.0.offset;

        match self.0.kind {
            ExprKind::Literal(v) => {
                let n = String::from_utf8_lossy(v).parse::<i64>()?;
                Ok(Value::Constant(n))
            }
            ExprKind::Identifier(v) => {
                let name = String::from_utf8_lossy(v).to_string();

                if let Some(val) = scope.resolve(&name) {
                    return Ok(val);
                }

                Err(format!("Undeclared identifier '{}' at offset {}", name, offset).into())
            }
            ExprKind::Binary { left, op, right } => {
                let l = left.generate(ctx, scope, id)?;
                let r = right.generate(ctx, scope, id)?;
                let dst = ctx.next_symbol();

                match op {
                    Op::Add => ctx.emit(
                        InstructionKind::Add {
                            dst,
                            left: l,
                            right: r,
                        },
                        offset,
                    ),
                    Op::Equals => ctx.emit(
                        InstructionKind::Eq {
                            dst,
                            left: l,
                            right: r,
                        },
                        offset,
                    ),
                }

                Ok(Value::Symbol(dst))
            }
            ExprKind::Call { callee, args } => {
                let name = String::from_utf8_lossy(callee).to_string();

                let c = if let Some(Value::Symbol(s)) = scope.resolve(&name) {
                    s
                } else {
                    return Err(
                        format!("Undeclared identifier '{}' at offset {}", name, offset).into(),
                    );
                };

                let mut a = Vec::new();

                for arg in args {
                    let value = arg.generate(ctx, scope, id)?;
                    a.push(value);
                }

                let dst = ctx.next_symbol();

                ctx.emit(
                    InstructionKind::Call {
                        dst,
                        callee: c,
                        args: a,
                    },
                    offset,
                );

                Ok(Value::Symbol(dst))
            }
        }
    }
}
