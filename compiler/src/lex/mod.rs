use std::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Ignore,
    Function,
    Return,
    If,
    Else,
    Identifier,
    Number,
    Equals,
    Equal,
    Plus,
    LParen,
    RParen,
    LBrace,
    RBrace,
}

#[derive(Debug)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub value: &'a [u8],
    pub start: usize,
    pub end: usize,
}

enum Matcher {
    Fixed(&'static [u8]),
    Dynamic(fn(&[u8]) -> Option<usize>),
}

struct Rule {
    matcher: Matcher,
    kind: TokenKind,
}

fn match_pattern(input: &[u8], pattern: &[u8]) -> Option<usize> {
    if input.starts_with(pattern) {
        Some(pattern.len())
    } else {
        None
    }
}

fn match_identifier(input: &[u8]) -> Option<usize> {
    if input.is_empty() {
        return None;
    }

    let first = input[0];

    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }

    let mut len = 1;

    while len < input.len() && (input[len].is_ascii_alphanumeric() || input[len] == b'_') {
        len += 1;
    }

    Some(len)
}

fn match_number(input: &[u8]) -> Option<usize> {
    let mut len = 0;

    while len < input.len() && input[len].is_ascii_digit() {
        len += 1;
    }

    if len > 0 { Some(len) } else { None }
}

fn match_whitespace(input: &[u8]) -> Option<usize> {
    let mut len = 0;

    while len < input.len() && input[len].is_ascii_whitespace() {
        len += 1;
    }

    if len > 0 { Some(len) } else { None }
}

static RULES: &[Rule] = &[
    Rule {
        matcher: Matcher::Dynamic(match_whitespace),
        kind: TokenKind::Ignore,
    },
    Rule {
        matcher: Matcher::Fixed(b"fn"),
        kind: TokenKind::Function,
    },
    Rule {
        matcher: Matcher::Fixed(b"return"),
        kind: TokenKind::Return,
    },
    Rule {
        matcher: Matcher::Fixed(b"if"),
        kind: TokenKind::If,
    },
    Rule {
        matcher: Matcher::Fixed(b"else"),
        kind: TokenKind::Else,
    },
    Rule {
        matcher: Matcher::Dynamic(match_number),
        kind: TokenKind::Number,
    },
    Rule {
        matcher: Matcher::Dynamic(match_identifier),
        kind: TokenKind::Identifier,
    },
    Rule {
        matcher: Matcher::Fixed(b"=="),
        kind: TokenKind::Equals,
    },
    Rule {
        matcher: Matcher::Fixed(b"="),
        kind: TokenKind::Equal,
    },
    Rule {
        matcher: Matcher::Fixed(b"+"),
        kind: TokenKind::Plus,
    },
    Rule {
        matcher: Matcher::Fixed(b"("),
        kind: TokenKind::LParen,
    },
    Rule {
        matcher: Matcher::Fixed(b")"),
        kind: TokenKind::RParen,
    },
    Rule {
        matcher: Matcher::Fixed(b"{"),
        kind: TokenKind::LBrace,
    },
    Rule {
        matcher: Matcher::Fixed(b"}"),
        kind: TokenKind::RBrace,
    },
];

pub fn tokenize<'a>(input: &'a [u8]) -> Result<Vec<Token<'a>>, Box<dyn Error>> {
    let mut tokens = Vec::new();
    let mut offset = 0;

    while offset < input.len() {
        let mut matched = false;
        let rest = &input[offset..];

        for rule in RULES {
            let res = match rule.matcher {
                Matcher::Fixed(p) => match_pattern(rest, p),
                Matcher::Dynamic(f) => f(rest),
            };

            if let Some(len) = res {
                if rule.kind != TokenKind::Ignore {
                    tokens.push(Token {
                        kind: rule.kind.clone(),
                        value: &rest[..len],
                        start: offset,
                        end: offset + len,
                    });
                }
                offset += len;
                matched = true;
                break;
            }
        }

        if !matched {
            let b = input[offset];
            let c = if b.is_ascii_graphic() || b == b' ' {
                format!("'{}'", b as char)
            } else {
                format!("0x{:02X}", b)
            };
            return Err(format!("Unexpected character {} at offset {}", c, offset).into());
        }
    }

    Ok(tokens)
}
