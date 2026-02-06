use std::error::Error;

use crate::{
    ast::{Node, Parse, Parser, stmt::Stmt},
    lex::TokenKind,
};

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl<'a> {
    pub name: &'a [u8],
    pub body: Vec<Stmt<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeclKind<'a> {
    Function(FunctionDecl<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Decl<'a>(pub Node<DeclKind<'a>>);

impl<'a> Parse<'a> for Decl<'a> {
    fn parse(parser: &mut Parser<'a>) -> Result<Self, Box<dyn Error>> {
        let offset = parser.peek().start;

        if parser.match_token(TokenKind::Function) {
            return Ok(Decl(Node {
                kind: DeclKind::Function(FunctionDecl::parse(parser)?),
                offset,
            }));
        }

        Err(parser.expected("declaration"))
    }
}

impl<'a> Parse<'a> for FunctionDecl<'a> {
    fn parse(parser: &mut Parser<'a>) -> Result<Self, Box<dyn Error>> {
        let name = parser.consume(TokenKind::Identifier, "identifier")?.value;

        parser.consume(TokenKind::LParen, "'('")?;
        parser.consume(TokenKind::RParen, "')'")?;
        parser.consume(TokenKind::LBrace, "'{'")?;

        let mut body = Vec::new();
        while !parser.check(TokenKind::RBrace) && !parser.is_empty() {
            body.push(Stmt::parse(parser)?);
        }

        parser.consume(TokenKind::RBrace, "'}'")?;

        Ok(FunctionDecl { name, body })
    }
}
