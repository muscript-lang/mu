use std::fmt;

use crate::ast::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Let,
    True,
    False,
    Ident(String),
    Int(i64),
    String(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Eq,
    EqEq,
    Plus,
    Minus,
    Star,
    Slash,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub span: Span,
    pub message: String,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for LexError {}

pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
    let mut lx = Lexer {
        chars: src.char_indices().peekable(),
        last_end: 0,
    };
    lx.tokenize()
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    last_end: usize,
}

impl<'a> Lexer<'a> {
    fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut out = Vec::new();
        while let Some((idx, ch)) = self.peek() {
            if ch.is_whitespace() {
                self.bump();
                continue;
            }
            if ch == '/' && self.peek_next_char() == Some('/') {
                self.bump();
                self.bump();
                while let Some((_, c)) = self.peek() {
                    self.bump();
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }
            let token = match ch {
                '(' => self.simple(idx, TokenKind::LParen),
                ')' => self.simple(idx, TokenKind::RParen),
                '[' => self.simple(idx, TokenKind::LBracket),
                ']' => self.simple(idx, TokenKind::RBracket),
                ',' => self.simple(idx, TokenKind::Comma),
                ';' => self.simple(idx, TokenKind::Semicolon),
                '+' => self.simple(idx, TokenKind::Plus),
                '-' => self.simple(idx, TokenKind::Minus),
                '*' => self.simple(idx, TokenKind::Star),
                '/' => self.simple(idx, TokenKind::Slash),
                '=' => {
                    self.bump();
                    if self.peek_char() == Some('=') {
                        let _ = self.bump().expect("peeked before bump");
                        Token {
                            kind: TokenKind::EqEq,
                            span: Span {
                                start: idx,
                                end: idx + 2,
                            },
                        }
                    } else {
                        Token {
                            kind: TokenKind::Eq,
                            span: Span {
                                start: idx,
                                end: idx + 1,
                            },
                        }
                    }
                }
                '"' => self.lex_string()?,
                c if is_ident_start(c) => self.lex_ident_or_keyword(),
                c if c.is_ascii_digit() => self.lex_int()?,
                _ => {
                    return Err(LexError {
                        span: Span {
                            start: idx,
                            end: idx + ch.len_utf8(),
                        },
                        message: format!("unexpected character `{ch}`"),
                    });
                }
            };
            out.push(token);
        }

        out.push(Token {
            kind: TokenKind::Eof,
            span: Span {
                start: self.last_end,
                end: self.last_end,
            },
        });
        Ok(out)
    }

    fn simple(&mut self, idx: usize, kind: TokenKind) -> Token {
        let (_, ch) = self.bump().expect("peeked before bump");
        Token {
            kind,
            span: Span {
                start: idx,
                end: idx + ch.len_utf8(),
            },
        }
    }

    fn lex_ident_or_keyword(&mut self) -> Token {
        let (start, first) = self.bump().expect("peeked before bump");
        let mut name = String::from(first);

        while let Some((_, c)) = self.peek() {
            if is_ident_continue(c) {
                let (_, consumed) = self.bump().expect("peeked before bump");
                name.push(consumed);
            } else {
                break;
            }
        }

        let kind = match name.as_str() {
            "let" => TokenKind::Let,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            _ => TokenKind::Ident(name),
        };

        Token {
            kind,
            span: Span {
                start,
                end: self.last_end,
            },
        }
    }

    fn lex_int(&mut self) -> Result<Token, LexError> {
        let (start, first) = self.bump().expect("peeked before bump");
        let mut digits = String::from(first);

        while let Some((_, c)) = self.peek() {
            if c.is_ascii_digit() {
                let (_, consumed) = self.bump().expect("peeked before bump");
                digits.push(consumed);
            } else {
                break;
            }
        }

        let value = digits.parse::<i64>().map_err(|_| LexError {
            span: Span {
                start,
                end: self.last_end,
            },
            message: "integer literal out of range".to_string(),
        })?;

        Ok(Token {
            kind: TokenKind::Int(value),
            span: Span {
                start,
                end: self.last_end,
            },
        })
    }

    fn lex_string(&mut self) -> Result<Token, LexError> {
        let (start, _) = self.bump().expect("peeked before bump");
        let mut value = String::new();

        while let Some((idx, ch)) = self.bump() {
            match ch {
                '"' => {
                    return Ok(Token {
                        kind: TokenKind::String(value),
                        span: Span {
                            start,
                            end: idx + 1,
                        },
                    });
                }
                '\\' => {
                    let (_, next) = self.bump().ok_or(LexError {
                        span: Span {
                            start,
                            end: self.last_end,
                        },
                        message: "unterminated escape sequence".to_string(),
                    })?;
                    let escaped = match next {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '"' => '"',
                        '\\' => '\\',
                        other => {
                            return Err(LexError {
                                span: Span {
                                    start: idx,
                                    end: self.last_end,
                                },
                                message: format!("unsupported escape `\\{other}`"),
                            });
                        }
                    };
                    value.push(escaped);
                }
                _ => value.push(ch),
            }
        }

        Err(LexError {
            span: Span {
                start,
                end: self.last_end,
            },
            message: "unterminated string literal".to_string(),
        })
    }

    fn peek(&mut self) -> Option<(usize, char)> {
        self.chars.peek().copied()
    }

    fn peek_char(&mut self) -> Option<char> {
        self.peek().map(|(_, ch)| ch)
    }

    fn peek_next_char(&self) -> Option<char> {
        let mut iter = self.chars.clone();
        iter.next()?;
        iter.next().map(|(_, ch)| ch)
    }

    fn bump(&mut self) -> Option<(usize, char)> {
        let next = self.chars.next();
        if let Some((idx, ch)) = next {
            self.last_end = idx + ch.len_utf8();
        }
        next
    }
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}
