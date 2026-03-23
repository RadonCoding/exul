use std::error::Error;

use intermediate::{Memory, Segment};

use crate::{
    ast::{Node, Parse, Parser},
    lex::TokenKind,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind<'a> {
    Binary {
        left: Box<Expr<'a>>,
        op: BinaryOp,
        right: Box<Expr<'a>>,
    },
    Compound {
        dst: &'a [u8],
        op: BinaryOp,
        src: Box<Expr<'a>>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr<'a>>,
    },
    Identifier(&'a [u8]),
    Number(&'a [u8]),
    String(&'a [u8]),
    Call {
        callee: &'a [u8],
        args: Vec<Expr<'a>>,
    },
    Import {
        module: &'a [u8],
        function: &'a [u8],
        args: Vec<Expr<'a>>,
    },
    Load {
        size: Memory,
        address: Box<Expr<'a>>,
    },
    Segment {
        seg: Segment,
        offset: Box<Expr<'a>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Equals,
    NotEquals,
    Lte,
    Gte,
    Lt,
    Gt,
    Add,
    Sub,
    Mul,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
}

impl BinaryOp {
    fn from_compound(kind: TokenKind) -> Option<(Self, i32)> {
        match kind {
            TokenKind::PlusEqual => Some((BinaryOp::Add, 1)),
            TokenKind::MinusEqual => Some((BinaryOp::Sub, 1)),
            TokenKind::AmpersandEqual => Some((BinaryOp::And, 1)),
            TokenKind::PipeEqual => Some((BinaryOp::Or, 1)),
            TokenKind::CaretEqual => Some((BinaryOp::Xor, 1)),
            _ => None,
        }
    }

    fn from_token(kind: TokenKind) -> Option<(Self, i32)> {
        match kind {
            TokenKind::Equals => Some((BinaryOp::Equals, 2)),
            TokenKind::NotEquals => Some((BinaryOp::NotEquals, 2)),
            TokenKind::LessEqual => Some((BinaryOp::Lte, 3)),
            TokenKind::GreaterEqual => Some((BinaryOp::Gte, 3)),
            TokenKind::Less => Some((BinaryOp::Lt, 3)),
            TokenKind::Greater => Some((BinaryOp::Gt, 3)),
            TokenKind::Plus => Some((BinaryOp::Add, 4)),
            TokenKind::Minus => Some((BinaryOp::Sub, 4)),
            TokenKind::Star => Some((BinaryOp::Mul, 5)),
            TokenKind::Ampersand => Some((BinaryOp::And, 5)),
            TokenKind::Pipe => Some((BinaryOp::Or, 5)),
            TokenKind::Caret => Some((BinaryOp::Xor, 5)),
            TokenKind::ShiftLeft => Some((BinaryOp::Shl, 6)),
            TokenKind::ShiftRight => Some((BinaryOp::Shr, 6)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr<'a>(pub Node<ExprKind<'a>>);

impl<'a> Expr<'a> {
    fn parse_primary(parser: &mut Parser<'a>) -> Result<Self, Box<dyn Error>> {
        let position = parser.peek().start;

        // Grouped expression
        if parser.match_token(TokenKind::LParen) {
            let inner = Expr::parse(parser)?;
            parser.consume(TokenKind::RParen, "')'")?;
            return Ok(inner);
        }

        // Unary negation
        if parser.match_token(TokenKind::Minus) {
            let expr = Expr::parse_primary(parser)?;
            return Ok(Expr(Node {
                kind: ExprKind::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                },
                position,
            }));
        }

        // Literals
        if parser.match_token(TokenKind::Number) {
            return Ok(Expr(Node {
                kind: ExprKind::Number(parser.previous().value),
                position,
            }));
        }

        if parser.match_token(TokenKind::String) {
            return Ok(Expr(Node {
                kind: ExprKind::String(parser.previous().value),
                position,
            }));
        }

        // Segment registers
        if parser.match_token(TokenKind::Gs) || parser.match_token(TokenKind::Fs) {
            let seg = if parser.previous().kind == TokenKind::Gs {
                Segment::Gs
            } else {
                Segment::Fs
            };
            parser.consume(TokenKind::LParen, "'('")?;
            let offset = Expr::parse(parser)?;
            parser.consume(TokenKind::RParen, "')'")?;
            return Ok(Expr(Node {
                kind: ExprKind::Segment {
                    seg,
                    offset: Box::new(offset),
                },
                position,
            }));
        }

        // Memory dereference: *byte(...), *word(...), *dword(...), *qword(...)
        if parser.match_token(TokenKind::Star) {
            let size = match parser.peek().kind {
                TokenKind::Byte => {
                    parser.advance();
                    Memory::Byte
                }
                TokenKind::Word => {
                    parser.advance();
                    Memory::Word
                }
                TokenKind::Dword => {
                    parser.advance();
                    Memory::Dword
                }
                TokenKind::Qword => {
                    parser.advance();
                    Memory::Qword
                }
                _ => return Err(parser.expected("memory size")),
            };
            parser.consume(TokenKind::LParen, "'('")?;
            let addr = Expr::parse(parser)?;
            parser.consume(TokenKind::RParen, "')'")?;
            return Ok(Expr(Node {
                kind: ExprKind::Load {
                    size,
                    address: Box::new(addr),
                },
                position,
            }));
        }

        // Identifiers, calls, and imports
        if parser.match_token(TokenKind::Identifier) {
            let id = parser.previous().value;

            // Import: module!function(args)
            if parser.match_token(TokenKind::Bang) {
                let function = parser
                    .consume(TokenKind::Identifier, "function name")?
                    .value;
                parser.consume(TokenKind::LParen, "'('")?;
                let args = parse_args(parser)?;
                parser.consume(TokenKind::RParen, "')'")?;
                return Ok(Expr(Node {
                    kind: ExprKind::Import {
                        module: id,
                        function,
                        args,
                    },
                    position,
                }));
            }

            // Call: name(args)
            if parser.match_token(TokenKind::LParen) {
                let args = parse_args(parser)?;
                parser.consume(TokenKind::RParen, "')'")?;
                return Ok(Expr(Node {
                    kind: ExprKind::Call { callee: id, args },
                    position,
                }));
            }

            return Ok(Expr(Node {
                kind: ExprKind::Identifier(id),
                position,
            }));
        }

        Err(parser.expected("expression"))
    }

    fn parse_precedence(parser: &mut Parser<'a>, minimum: i32) -> Result<Self, Box<dyn Error>> {
        let offset = parser.peek().start;
        let mut left = Self::parse_primary(parser)?;

        loop {
            let next = parser.peek().kind;

            if let Some((op, precedence)) = BinaryOp::from_compound(next) {
                if precedence < minimum {
                    break;
                }
                parser.advance();
                let src = Self::parse_precedence(parser, precedence)?;
                if let ExprKind::Identifier(dst) = left.0.kind {
                    left = Expr(Node {
                        position: offset,
                        kind: ExprKind::Compound {
                            dst,
                            op,
                            src: Box::new(src),
                        },
                    });
                    continue;
                }
            }

            if let Some((op, precedence)) = BinaryOp::from_token(next) {
                if precedence < minimum {
                    break;
                }
                parser.advance();
                let right = Self::parse_precedence(parser, precedence + 1)?;
                left = Expr(Node {
                    position: offset,
                    kind: ExprKind::Binary {
                        left: Box::new(left),
                        op,
                        right: Box::new(right),
                    },
                });
                continue;
            }

            break;
        }

        Ok(left)
    }
}

impl<'a> Parse<'a> for Expr<'a> {
    fn parse(parser: &mut Parser<'a>) -> Result<Self, Box<dyn Error>> {
        Self::parse_precedence(parser, 0)
    }
}

fn parse_args<'a>(parser: &mut Parser<'a>) -> Result<Vec<Expr<'a>>, Box<dyn Error>> {
    let mut args = Vec::new();
    if !parser.check(TokenKind::RParen) {
        loop {
            args.push(Expr::parse(parser)?);
            if !parser.match_token(TokenKind::Comma) {
                break;
            }
        }
    }
    Ok(args)
}
