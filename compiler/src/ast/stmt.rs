use std::error::Error;

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
        condition: Expr<'a>,
        consequent: Vec<Stmt<'a>>,
        alternate: Option<Vec<Stmt<'a>>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt<'a>(pub Node<StmtKind<'a>>);

impl<'a> Stmt<'a> {
    fn parse_block(parser: &mut Parser<'a>) -> Result<Vec<Stmt<'a>>, Box<dyn Error>> {
        parser.consume(TokenKind::LBrace, "'{'")?;

        let mut stmts = Vec::new();

        while !parser.check(TokenKind::RBrace) && !parser.is_at_end() {
            stmts.push(Stmt::parse(parser)?);
        }

        parser.consume(TokenKind::RBrace, "'}'")?;

        Ok(stmts)
    }
}

impl<'a> Parse<'a> for Stmt<'a> {
    fn parse(parser: &mut Parser<'a>) -> Result<Self, Box<dyn Error>> {
        let offset = parser.peek().start;

        if parser.match_token(TokenKind::Return) {
            return Ok(Stmt(Node {
                kind: StmtKind::Return(Expr::parse(parser)?),
                offset,
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
                    condition,
                    consequent: then_branch,
                    alternate: else_branch,
                },
                offset,
            }));
        }

        if parser.check(TokenKind::Identifier) && parser.check_next(TokenKind::Equal) {
            let name = parser.consume(TokenKind::Identifier, "identifier")?.value;
            parser.consume(TokenKind::Equal, "'='")?;
            let value = Expr::parse(parser)?;

            return Ok(Stmt(Node {
                kind: StmtKind::Assignment { name, value },
                offset,
            }));
        }

        Err(parser.expected("statement"))
    }
}
