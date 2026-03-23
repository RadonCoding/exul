use intermediate::{Context, FunctionId, Import, InstructionKind, Value, symbols::Symbols};

use crate::{
    ast::expr::{BinaryOp, Expr, ExprKind, UnaryOp},
    lower::Generate,
};
use std::error::Error;

fn parse_number(v: &[u8]) -> Result<i64, Box<dyn std::error::Error>> {
    let s = String::from_utf8_lossy(v);

    if let Some(hex) = s.strip_prefix("0x") {
        Ok(u64::from_str_radix(hex, 16)? as i64)
    } else {
        Ok(s.parse::<i64>()?)
    }
}

impl Generate for Expr<'_> {
    type Output = Value;

    fn generate(
        self,
        ctx: &mut Context,
        scope: &mut Symbols,
        id: FunctionId,
    ) -> Result<Self::Output, Box<dyn Error>> {
        match self.0.kind {
            ExprKind::Number(v) => Ok(Value::Constant(parse_number(v)?)),
            ExprKind::String(v) => {
                let s = String::from_utf8_lossy(v).to_string();
                Ok(Value::String(ctx.string(s)))
            }
            ExprKind::Identifier(v) => {
                let name = String::from_utf8_lossy(v).to_string();
                scope.resolve(&name).ok_or_else(|| {
                    format!(
                        "Undeclared identifier '{}' at offset {}",
                        name, self.0.position
                    )
                    .into()
                })
            }
            ExprKind::Unary {
                op: UnaryOp::Neg,
                expr,
            } => match expr.0.kind {
                ExprKind::Number(v) => Ok(Value::Constant(-parse_number(v)?)),
                _ => {
                    let val = expr.generate(ctx, scope, id)?;
                    let dst = ctx.next_symbol();
                    ctx.emit(
                        InstructionKind::Sub {
                            dst,
                            left: Value::Constant(0),
                            right: val,
                        },
                        self.0.position,
                    );
                    Ok(Value::Symbol(dst))
                }
            },
            ExprKind::Compound { dst, op, src } => {
                let name = String::from_utf8_lossy(dst).to_string();
                let lhs = scope.resolve(&name).ok_or_else(|| {
                    format!(
                        "Undeclared identifier '{}' at offset {}",
                        name, self.0.position
                    )
                })?;

                let rhs = src.generate(ctx, scope, id)?;

                if let Value::Symbol(s) = lhs {
                    match op {
                        BinaryOp::Add => ctx.emit(
                            InstructionKind::Add {
                                dst: s,
                                left: lhs,
                                right: rhs,
                            },
                            self.0.position,
                        ),
                        BinaryOp::Sub => ctx.emit(
                            InstructionKind::Sub {
                                dst: s,
                                left: lhs,
                                right: rhs,
                            },
                            self.0.position,
                        ),
                        _ => {}
                    }
                    return Ok(Value::Symbol(s));
                }

                Err(format!("Cannot assign to '{}' at offset {}", name, self.0.position).into())
            }
            ExprKind::Binary { left, op, right } => {
                let left = left.generate(ctx, scope, id)?;
                let right = right.generate(ctx, scope, id)?;
                let dst = ctx.next_symbol();

                match op {
                    BinaryOp::Equals => {
                        ctx.emit(InstructionKind::Eq { dst, left, right }, self.0.position)
                    }
                    BinaryOp::NotEquals => {
                        ctx.emit(InstructionKind::NotEq { dst, left, right }, self.0.position)
                    }
                    BinaryOp::Lte => {
                        ctx.emit(InstructionKind::Lte { dst, left, right }, self.0.position)
                    }
                    BinaryOp::Gte => {
                        ctx.emit(InstructionKind::Gte { dst, left, right }, self.0.position)
                    }
                    BinaryOp::Lt => {
                        ctx.emit(InstructionKind::Lt { dst, left, right }, self.0.position)
                    }
                    BinaryOp::Gt => {
                        ctx.emit(InstructionKind::Gt { dst, left, right }, self.0.position)
                    }
                    BinaryOp::Add => {
                        ctx.emit(InstructionKind::Add { dst, left, right }, self.0.position)
                    }
                    BinaryOp::Sub => {
                        ctx.emit(InstructionKind::Sub { dst, left, right }, self.0.position)
                    }
                    BinaryOp::Mul => {
                        ctx.emit(InstructionKind::Mul { dst, left, right }, self.0.position)
                    }
                    BinaryOp::And => {
                        ctx.emit(InstructionKind::And { dst, left, right }, self.0.position)
                    }
                    BinaryOp::Or => {
                        ctx.emit(InstructionKind::Or { dst, left, right }, self.0.position)
                    }
                    BinaryOp::Xor => {
                        ctx.emit(InstructionKind::Xor { dst, left, right }, self.0.position)
                    }
                    BinaryOp::Shl => {
                        ctx.emit(InstructionKind::Shl { dst, left, right }, self.0.position)
                    }
                    BinaryOp::Shr => {
                        ctx.emit(InstructionKind::Shr { dst, left, right }, self.0.position)
                    }
                }

                Ok(Value::Symbol(dst))
            }
            ExprKind::Call { callee, args } => {
                let name = String::from_utf8_lossy(callee).to_string();

                let callee = match scope.resolve(&name) {
                    Some(Value::Function(f)) => f,
                    _ => {
                        return Err(format!(
                            "Undeclared identifier '{}' at offset {}",
                            name, self.0.position
                        )
                        .into());
                    }
                };

                let args = args
                    .into_iter()
                    .map(|arg| arg.generate(ctx, scope, id))
                    .collect::<Result<Vec<_>, _>>()?;

                let dst = ctx.next_symbol();

                ctx.emit(InstructionKind::Call { dst, callee, args }, self.0.position);

                Ok(Value::Symbol(dst))
            }
            ExprKind::Import {
                module,
                function,
                args,
            } => {
                let module = String::from_utf8_lossy(module).to_string();
                let function = String::from_utf8_lossy(function).to_string();
                let key = format!("{}!{}", module, function);

                let callee = if let Some(Value::Function(f)) = scope.resolve(&key) {
                    f
                } else {
                    let id = ctx.next_function();
                    scope.define(key, Value::Function(id));
                    ctx.imports.push(Import {
                        module,
                        function,
                        id,
                    });
                    id
                };

                let args = args
                    .into_iter()
                    .map(|arg| arg.generate(ctx, scope, id))
                    .collect::<Result<Vec<Value>, Box<dyn Error>>>()?;

                let dst = ctx.next_symbol();

                ctx.emit(InstructionKind::Call { dst, callee, args }, self.0.position);

                Ok(Value::Symbol(dst))
            }
            ExprKind::Load { size, address } => {
                let address = address.generate(ctx, scope, id)?;
                let dst = ctx.next_symbol();

                ctx.emit(
                    InstructionKind::Load {
                        dst,
                        size: size.into(),
                        src: address,
                    },
                    self.0.position,
                );

                Ok(Value::Symbol(dst))
            }
            ExprKind::Segment { seg, offset } => {
                let offset = offset.generate(ctx, scope, id)?;
                let dst = ctx.next_symbol();

                ctx.emit(
                    InstructionKind::Segment { dst, seg, offset },
                    self.0.position,
                );

                Ok(Value::Symbol(dst))
            }
        }
    }
}
