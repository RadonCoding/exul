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
                let n = String::from_utf8_lossy(v).parse::<i64>().unwrap_or(0);
                Ok(Value::Constant(n))
            }
            ExprKind::Variable(v) => {
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
        }
    }
}
