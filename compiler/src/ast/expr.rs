use std::error::Error;

use crate::{
    ast::{Node, Parse, Parser},
    lex::TokenKind,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind<'a> {
    Binary {
        left: Box<Expr<'a>>,
        op: Op,
        right: Box<Expr<'a>>,
    },
    Compound {
        dst: &'a [u8],
        op: Op,
        src: Box<Expr<'a>>,
    },
    Identifier(&'a [u8]),
    Literal(&'a [u8]),
    Call {
        callee: &'a [u8],
        args: Vec<Expr<'a>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    Equals,
}

impl Op {
    fn from_compound(kind: TokenKind) -> Option<(Self, i32)> {
        match kind {
            TokenKind::PlusEqual => Some((Op::Add, 1)),
            _ => None,
        }
    }

    fn from_token(kind: TokenKind) -> Option<(Self, i32)> {
        match kind {
            TokenKind::Equals => Some((Op::Equals, 2)),
            TokenKind::Plus => Some((Op::Add, 4)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr<'a>(pub Node<ExprKind<'a>>);

impl<'a> Expr<'a> {
    fn parse_primary(parser: &mut Parser<'a>) -> Result<Self, Box<dyn Error>> {
        let offset = parser.peek().start;

        if parser.match_token(TokenKind::Number) {
            return Ok(Expr(Node {
                kind: ExprKind::Literal(parser.previous().value),
                offset,
            }));
        }

        if parser.match_token(TokenKind::Identifier) {
            let id = parser.previous().value;

            if parser.match_token(TokenKind::LParen) {
                let mut args = Vec::new();
                if !parser.check(TokenKind::RParen) {
                    loop {
                        args.push(Expr::parse(parser)?);
                        if !parser.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                parser.consume(TokenKind::RParen, "')'")?;

                return Ok(Expr(Node {
                    kind: ExprKind::Call { callee: id, args },
                    offset,
                }));
            }

            return Ok(Expr(Node {
                kind: ExprKind::Identifier(id),
                offset,
            }));
        }

        Err(parser.expected("expression"))
    }

    fn parse_precedence(parser: &mut Parser<'a>, minimum: i32) -> Result<Self, Box<dyn Error>> {
        let offset = parser.peek().start;

        let mut left = Self::parse_primary(parser)?;

        loop {
            let next = parser.peek().kind;

            if let Some((op, precedence)) = Op::from_compound(next) {
                if precedence < minimum {
                    break;
                }

                parser.advance();

                let src = Self::parse_precedence(parser, precedence)?;

                if let ExprKind::Identifier(dst) = left.0.kind {
                    left = Expr(Node {
                        offset,
                        kind: ExprKind::Compound {
                            dst,
                            op,
                            src: Box::new(src),
                        },
                    });
                    continue;
                }
            }

            if let Some((op, precedence)) = Op::from_token(next) {
                if precedence < minimum {
                    break;
                }

                parser.advance();

                let right = Self::parse_precedence(parser, precedence + 1)?;

                left = Expr(Node {
                    offset,
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
