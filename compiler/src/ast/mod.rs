pub mod decl;
pub mod expr;
pub mod stmt;

use crate::{
    ast::decl::Decl,
    lex::{Token, TokenKind},
};
use std::error::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Node<T> {
    pub kind: T,
    pub position: usize,
}

#[derive(Debug)]
pub struct Tree<'a> {
    pub decls: Vec<Decl<'a>>,
}

pub struct Parser<'a> {
    tokens: Vec<Token<'a>>,
    cursor: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token<'a>>) -> Self {
        Self { tokens, cursor: 0 }
    }

    pub fn consume(&mut self, kind: TokenKind, msg: &str) -> Result<&Token<'a>, Box<dyn Error>> {
        if self.check(kind.clone()) {
            return Ok(self.advance());
        }
        Err(self.expected(msg))
    }

    pub fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            return true;
        }
        false
    }

    pub fn check(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    pub fn check_next(&self, kind: TokenKind) -> bool {
        let next = self.cursor + 1;

        if next < self.tokens.len() {
            return self.tokens[next].kind == kind;
        }
        false
    }

    pub fn advance(&mut self) -> &Token<'a> {
        if !self.is_eof() {
            self.cursor += 1;
        }
        self.previous()
    }

    pub fn is_eof(&self) -> bool {
        self.cursor == self.tokens.len() - 1
    }

    pub fn peek(&self) -> &Token<'a> {
        &self.tokens[self.cursor]
    }

    pub fn previous(&self) -> &Token<'a> {
        &self.tokens[self.cursor - 1]
    }

    pub fn expected(&self, msg: &str) -> Box<dyn Error> {
        let token = self.peek();
        let found = String::from_utf8_lossy(token.value);
        format!(
            "Expected {}, but found '{}' at offset {}",
            msg, found, token.start
        )
        .into()
    }
}

pub trait Parse<'a>: Sized {
    fn parse(parser: &mut Parser<'a>) -> Result<Self, Box<dyn Error>>;
}

pub fn parse(tokens: Vec<Token>) -> Result<Tree, Box<dyn Error>> {
    let mut parser = Parser::new(tokens);
    let mut decls = Vec::new();

    while !parser.is_eof() {
        decls.push(Decl::parse(&mut parser)?);
    }

    Ok(Tree { decls })
}
