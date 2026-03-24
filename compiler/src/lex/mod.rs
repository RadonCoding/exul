use std::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Ignore,
    // Keywords
    Function,
    Return,
    If,
    Else,
    For,
    Loop,
    Break,
    Gs,
    Fs,
    Byte,
    Word,
    Dword,
    Qword,
    // Literals
    Number,
    Char,
    String,
    Identifier,
    // Multi-character operators
    Equals,
    NotEquals,
    LessEqual,
    GreaterEqual,
    ShiftLeft,
    ShiftRight,
    PlusEqual,
    PlusPlus,
    MinusEqual,
    MinusMinus,
    StarEqual,
    AmpersandEqual,
    PipeEqual,
    CaretEqual,
    // Single-character operators
    Less,
    Greater,
    Equal,
    Plus,
    Minus,
    Star,
    Ampersand,
    Pipe,
    Caret,
    Bang,
    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
}

#[derive(Debug)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub value: &'a [u8],
    pub start: usize,
    pub _end: usize,
}

enum Matcher {
    Fixed(&'static [u8]),
    Dynamic(fn(&[u8]) -> Option<(usize, usize)>),
}

struct Rule {
    matcher: Matcher,
    kind: TokenKind,
}

fn match_pattern(input: &[u8], pattern: &[u8]) -> Option<(usize, usize)> {
    if !input.starts_with(pattern) {
        return None;
    }

    if pattern.iter().all(|b| b.is_ascii_alphabetic()) {
        let next = input.get(pattern.len());
        if matches!(next, Some(b) if matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')) {
            return None;
        }
    }

    Some((pattern.len(), pattern.len()))
}

fn match_number(input: &[u8]) -> Option<(usize, usize)> {
    if input.starts_with(b"0x") {
        let len = input[2..]
            .iter()
            .take_while(|&&b| matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F'))
            .count();
        if len > 0 {
            Some((len + 2, len + 2))
        } else {
            None
        }
    } else {
        let len = input
            .iter()
            .take_while(|&&b| matches!(b, b'0'..=b'9'))
            .count();
        if len > 0 { Some((len, len)) } else { None }
    }
}

fn match_string(input: &[u8]) -> Option<(usize, usize)> {
    if !input.starts_with(b"\"") {
        return None;
    }

    input[1..]
        .iter()
        .position(|&b| b == b'"')
        .map(|pos| (pos + 2, pos))
}

fn match_char(input: &[u8]) -> Option<(usize, usize)> {
    if !input.starts_with(b"'") {
        return None;
    }

    if input.len() >= 4 && input[1] == b'\\' {
        if input[3] == b'\'' {
            return Some((4, 2));
        }
    } else if input.len() >= 3 && input[2] == b'\'' {
        return Some((3, 1));
    }

    None
}

fn match_identifier(input: &[u8]) -> Option<(usize, usize)> {
    let first = input.first()?;

    if matches!(first, b'a'..=b'z' | b'A'..=b'Z' | b'_') {
        let len = input
            .iter()
            .take_while(|&&b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_'))
            .count();
        Some((len, len))
    } else {
        None
    }
}

fn match_whitespace(input: &[u8]) -> Option<(usize, usize)> {
    let len = input
        .iter()
        .take_while(|&&b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        .count();
    if len > 0 { Some((len, len)) } else { None }
}

fn match_comment(input: &[u8]) -> Option<(usize, usize)> {
    if input.starts_with(b"//") {
        let len = input.iter().take_while(|&&b| b != b'\n').count();
        return Some((len, len));
    }

    if input.starts_with(b"/*") {
        let mut i = 2;

        while i + 1 < input.len() {
            if input[i] == b'*' && input[i + 1] == b'/' {
                let total = i + 2;
                return Some((total, total));
            }
            i += 1;
        }
        return None;
    }

    None
}

static RULES: &[Rule] = &[
    Rule {
        matcher: Matcher::Dynamic(match_whitespace),
        kind: TokenKind::Ignore,
    },
    Rule {
        matcher: Matcher::Dynamic(match_comment),
        kind: TokenKind::Ignore,
    },
    // Keywords
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
        matcher: Matcher::Fixed(b"for"),
        kind: TokenKind::For,
    },
    Rule {
        matcher: Matcher::Fixed(b"loop"),
        kind: TokenKind::Loop,
    },
    Rule {
        matcher: Matcher::Fixed(b"break"),
        kind: TokenKind::Break,
    },
    Rule {
        matcher: Matcher::Fixed(b"gs"),
        kind: TokenKind::Gs,
    },
    Rule {
        matcher: Matcher::Fixed(b"fs"),
        kind: TokenKind::Fs,
    },
    Rule {
        matcher: Matcher::Fixed(b"byte"),
        kind: TokenKind::Byte,
    },
    Rule {
        matcher: Matcher::Fixed(b"word"),
        kind: TokenKind::Word,
    },
    Rule {
        matcher: Matcher::Fixed(b"dword"),
        kind: TokenKind::Dword,
    },
    Rule {
        matcher: Matcher::Fixed(b"qword"),
        kind: TokenKind::Qword,
    },
    // Literals
    Rule {
        matcher: Matcher::Dynamic(match_number),
        kind: TokenKind::Number,
    },
    Rule {
        matcher: Matcher::Dynamic(match_char),
        kind: TokenKind::Char,
    },
    Rule {
        matcher: Matcher::Dynamic(match_string),
        kind: TokenKind::String,
    },
    Rule {
        matcher: Matcher::Dynamic(match_identifier),
        kind: TokenKind::Identifier,
    },
    // Multi-character operators
    Rule {
        matcher: Matcher::Fixed(b"=="),
        kind: TokenKind::Equals,
    },
    Rule {
        matcher: Matcher::Fixed(b"!="),
        kind: TokenKind::NotEquals,
    },
    Rule {
        matcher: Matcher::Fixed(b"<="),
        kind: TokenKind::LessEqual,
    },
    Rule {
        matcher: Matcher::Fixed(b">="),
        kind: TokenKind::GreaterEqual,
    },
    Rule {
        matcher: Matcher::Fixed(b"<<"),
        kind: TokenKind::ShiftLeft,
    },
    Rule {
        matcher: Matcher::Fixed(b">>"),
        kind: TokenKind::ShiftRight,
    },
    Rule {
        matcher: Matcher::Fixed(b"+="),
        kind: TokenKind::PlusEqual,
    },
    Rule {
        matcher: Matcher::Fixed(b"++"),
        kind: TokenKind::PlusPlus,
    },
    Rule {
        matcher: Matcher::Fixed(b"-="),
        kind: TokenKind::MinusEqual,
    },
    Rule {
        matcher: Matcher::Fixed(b"--"),
        kind: TokenKind::MinusMinus,
    },
    Rule {
        matcher: Matcher::Fixed(b"*="),
        kind: TokenKind::StarEqual,
    },
    Rule {
        matcher: Matcher::Fixed(b"&="),
        kind: TokenKind::AmpersandEqual,
    },
    Rule {
        matcher: Matcher::Fixed(b"|="),
        kind: TokenKind::PipeEqual,
    },
    Rule {
        matcher: Matcher::Fixed(b"^="),
        kind: TokenKind::CaretEqual,
    },
    // Single-character operators
    Rule {
        matcher: Matcher::Fixed(b"<"),
        kind: TokenKind::Less,
    },
    Rule {
        matcher: Matcher::Fixed(b">"),
        kind: TokenKind::Greater,
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
        matcher: Matcher::Fixed(b"-"),
        kind: TokenKind::Minus,
    },
    Rule {
        matcher: Matcher::Fixed(b"*"),
        kind: TokenKind::Star,
    },
    Rule {
        matcher: Matcher::Fixed(b"&"),
        kind: TokenKind::Ampersand,
    },
    Rule {
        matcher: Matcher::Fixed(b"|"),
        kind: TokenKind::Pipe,
    },
    Rule {
        matcher: Matcher::Fixed(b"^"),
        kind: TokenKind::Caret,
    },
    Rule {
        matcher: Matcher::Fixed(b"!"),
        kind: TokenKind::Bang,
    },
    // Punctuation
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
    Rule {
        matcher: Matcher::Fixed(b","),
        kind: TokenKind::Comma,
    },
    Rule {
        matcher: Matcher::Fixed(b";"),
        kind: TokenKind::Semicolon,
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

            if let Some((total_len, value_len)) = res {
                if !matches!(rule.kind, TokenKind::Ignore) {
                    let start_skip = if matches!(rule.kind, TokenKind::String | TokenKind::Char) {
                        1
                    } else {
                        0
                    };
                    tokens.push(Token {
                        kind: rule.kind,
                        value: &rest[start_skip..start_skip + value_len],
                        start: offset,
                        _end: offset + total_len,
                    });
                }
                offset += total_len;
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
