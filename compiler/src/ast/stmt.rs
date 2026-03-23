use std::error::Error;

use intermediate::Memory;

use crate::{
    ast::{Node, Parse, Parser},
    lex::TokenKind,
};

use super::expr::Expr;

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind<'a> {
    Assignment {
        name: &'a [u8],
        value: Expr<'a>,
    },
    Return(Expr<'a>),
    If {
        cond: Expr<'a>,
        consequent: Vec<Stmt<'a>>,
        alternate: Option<Vec<Stmt<'a>>>,
    },
    For {
        init: Box<Stmt<'a>>,
        cond: Expr<'a>,
        step: Box<Stmt<'a>>,
        body: Vec<Stmt<'a>>,
    },
    Loop(Vec<Stmt<'a>>),
    Break,
    Store {
        size: Memory,
        address: Expr<'a>,
        value: Expr<'a>,
    },
    Expression(Expr<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt<'a>(pub Node<StmtKind<'a>>);

impl<'a> Stmt<'a> {
    fn parse_block(parser: &mut Parser<'a>) -> Result<Vec<Stmt<'a>>, Box<dyn Error>> {
        parser.consume(TokenKind::LBrace, "'{'")?;

        let mut stmts = Vec::new();

        while !parser.check(TokenKind::RBrace) && !parser.is_eof() {
            stmts.push(Stmt::parse(parser)?);
        }

        parser.consume(TokenKind::RBrace, "'}'")?;

        Ok(stmts)
    }

    fn parse_clause(parser: &mut Parser<'a>) -> Result<Stmt<'a>, Box<dyn Error>> {
        let position = parser.peek().start;

        if parser.check(TokenKind::Identifier) && parser.check_next(TokenKind::Equal) {
            let name = parser.consume(TokenKind::Identifier, "identifier")?.value;
            parser.consume(TokenKind::Equal, "'='")?;

            let value = Expr::parse(parser)?;

            return Ok(Stmt(Node {
                kind: StmtKind::Assignment { name, value },
                position,
            }));
        }

        Ok(Stmt(Node {
            kind: StmtKind::Expression(Expr::parse(parser)?),
            position,
        }))
    }
}

impl<'a> Parse<'a> for Stmt<'a> {
    fn parse(parser: &mut Parser<'a>) -> Result<Self, Box<dyn Error>> {
        let position = parser.peek().start;

        if parser.match_token(TokenKind::Return) {
            return Ok(Stmt(Node {
                kind: StmtKind::Return(Expr::parse(parser)?),
                position,
            }));
        }

        if parser.match_token(TokenKind::If) {
            let condition = Expr::parse(parser)?;
            let then_branch = Self::parse_block(parser)?;
            let mut else_branch = None;

            if parser.match_token(TokenKind::Else) {
                else_branch = Some(Self::parse_block(parser)?);
            }

            return Ok(Stmt(Node {
                kind: StmtKind::If {
                    cond: condition,
                    consequent: then_branch,
                    alternate: else_branch,
                },
                position,
            }));
        }

        if parser.match_token(TokenKind::For) {
            let init = Box::new(Stmt::parse_clause(parser)?);
            parser.consume(TokenKind::Semicolon, "';'")?;

            let cond = Expr::parse(parser)?;
            parser.consume(TokenKind::Semicolon, "';'")?;

            let step = Box::new(Stmt::parse_clause(parser)?);

            let body = if parser.match_token(TokenKind::LBrace) {
                let mut stmts = Vec::new();

                while !parser.check(TokenKind::RBrace) && !parser.is_eof() {
                    stmts.push(Stmt::parse(parser)?);
                }

                parser.consume(TokenKind::RBrace, "'}'")?;
                stmts
            } else if parser.match_token(TokenKind::Semicolon) {
                Vec::new()
            } else {
                return Err(parser.expected("'{' or ';'"));
            };

            return Ok(Stmt(Node {
                kind: StmtKind::For {
                    init,
                    cond,
                    step,
                    body,
                },
                position,
            }));
        }

        if parser.match_token(TokenKind::Loop) {
            let body = Stmt::parse_block(parser)?;
            return Ok(Stmt(Node {
                kind: StmtKind::Loop(body),
                position,
            }));
        }

        if parser.match_token(TokenKind::Break) {
            return Ok(Stmt(Node {
                kind: StmtKind::Break,
                position,
            }));
        }

        if parser.match_token(TokenKind::Star) {
            let size = match parser.peek().kind {
                TokenKind::Byte => {
                    parser.advance();
                    intermediate::Memory::Byte
                }
                TokenKind::Word => {
                    parser.advance();
                    intermediate::Memory::Word
                }
                TokenKind::Dword => {
                    parser.advance();
                    intermediate::Memory::Dword
                }
                TokenKind::Qword => {
                    parser.advance();
                    intermediate::Memory::Qword
                }
                _ => return Err(parser.expected("memory size")),
            };

            parser.consume(TokenKind::LParen, "'('")?;
            let address = Expr::parse(parser)?;
            parser.consume(TokenKind::RParen, "')'")?;

            parser.consume(TokenKind::Equal, "'='")?;
            let value = Expr::parse(parser)?;

            return Ok(Stmt(Node {
                kind: StmtKind::Store {
                    size,
                    address,
                    value,
                },
                position,
            }));
        }

        if parser.check(TokenKind::Identifier) && parser.check_next(TokenKind::Equal) {
            let name = parser.consume(TokenKind::Identifier, "identifier")?.value;
            parser.consume(TokenKind::Equal, "'='")?;
            let value = Expr::parse(parser)?;

            return Ok(Stmt(Node {
                kind: StmtKind::Assignment { name, value },
                position,
            }));
        }

        Ok(Stmt(Node {
            kind: StmtKind::Expression(Expr::parse(parser)?),
            position,
        }))
    }
}
