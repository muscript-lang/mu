use std::fmt;

use crate::ast::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexErrorCode {
    UnexpectedChar,
    UnterminatedString,
    UnterminatedEscape,
    InvalidEscape,
    UnterminatedBlockComment,
    InvalidIntLeadingZero,
    IntOutOfRange,
}

impl LexErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            LexErrorCode::UnexpectedChar => "E1001",
            LexErrorCode::UnterminatedString => "E1002",
            LexErrorCode::UnterminatedEscape => "E1003",
            LexErrorCode::InvalidEscape => "E1004",
            LexErrorCode::UnterminatedBlockComment => "E1005",
            LexErrorCode::InvalidIntLeadingZero => "E1006",
            LexErrorCode::IntOutOfRange => "E1007",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub code: LexErrorCode,
    pub span: Span,
    pub message: String,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} at bytes {}..{}",
            self.code.as_str(),
            self.message,
            self.span.start,
            self.span.end
        )
    }
}

impl std::error::Error for LexError {}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    At,
    Colon,
    Semicolon,
    Comma,
    Dot,
    Eq,
    Pipe,
    Bang,
    Question,
    Caret,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Arrow,
    FatArrow,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    Underscore,
    Ident(String),
    Int(i64),
    String(String),
    Eof,
}

pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer {
        src,
        chars: src.char_indices().peekable(),
        last_end: 0,
    };
    lexer.tokenize()
}

struct Lexer<'a> {
    src: &'a str,
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
                self.skip_line_comment();
                continue;
            }
            if ch == '/' && self.peek_next_char() == Some('*') {
                self.skip_block_comment()?;
                continue;
            }

            let token = match ch {
                '@' => self.simple(idx, TokenKind::At),
                ':' => self.simple(idx, TokenKind::Colon),
                ';' => self.simple(idx, TokenKind::Semicolon),
                ',' => self.simple(idx, TokenKind::Comma),
                '.' => self.simple(idx, TokenKind::Dot),
                '=' => {
                    self.bump();
                    if self.peek_char() == Some('=') {
                        self.bump();
                        Token {
                            kind: TokenKind::EqEq,
                            span: Span {
                                start: idx,
                                end: idx + 2,
                            },
                        }
                    } else if self.peek_char() == Some('>') {
                        self.bump();
                        Token {
                            kind: TokenKind::FatArrow,
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
                '|' => self.simple(idx, TokenKind::Pipe),
                '!' => {
                    self.bump();
                    if self.peek_char() == Some('=') {
                        self.bump();
                        Token {
                            kind: TokenKind::NotEq,
                            span: Span {
                                start: idx,
                                end: idx + 2,
                            },
                        }
                    } else {
                        Token {
                            kind: TokenKind::Bang,
                            span: Span {
                                start: idx,
                                end: idx + 1,
                            },
                        }
                    }
                }
                '?' => self.simple(idx, TokenKind::Question),
                '^' => self.simple(idx, TokenKind::Caret),
                '+' => self.simple(idx, TokenKind::Plus),
                '*' => self.simple(idx, TokenKind::Star),
                '%' => self.simple(idx, TokenKind::Percent),
                '(' => self.simple(idx, TokenKind::LParen),
                ')' => self.simple(idx, TokenKind::RParen),
                '[' => self.simple(idx, TokenKind::LBracket),
                ']' => self.simple(idx, TokenKind::RBracket),
                '{' => self.simple(idx, TokenKind::LBrace),
                '}' => self.simple(idx, TokenKind::RBrace),
                '<' => {
                    self.bump();
                    if self.peek_char() == Some('=') {
                        self.bump();
                        Token {
                            kind: TokenKind::Le,
                            span: Span {
                                start: idx,
                                end: idx + 2,
                            },
                        }
                    } else {
                        Token {
                            kind: TokenKind::Lt,
                            span: Span {
                                start: idx,
                                end: idx + 1,
                            },
                        }
                    }
                }
                '>' => {
                    self.bump();
                    if self.peek_char() == Some('=') {
                        self.bump();
                        Token {
                            kind: TokenKind::Ge,
                            span: Span {
                                start: idx,
                                end: idx + 2,
                            },
                        }
                    } else {
                        Token {
                            kind: TokenKind::Gt,
                            span: Span {
                                start: idx,
                                end: idx + 1,
                            },
                        }
                    }
                }
                '-' => {
                    self.bump();
                    if self.peek_char() == Some('>') {
                        self.bump();
                        Token {
                            kind: TokenKind::Arrow,
                            span: Span {
                                start: idx,
                                end: idx + 2,
                            },
                        }
                    } else {
                        Token {
                            kind: TokenKind::Minus,
                            span: Span {
                                start: idx,
                                end: idx + 1,
                            },
                        }
                    }
                }
                '_' => {
                    self.bump();
                    if let Some((_, next)) = self.peek() {
                        if is_ident_continue(next) {
                            self.lex_ident(idx, ch)
                        } else {
                            Token {
                                kind: TokenKind::Underscore,
                                span: Span {
                                    start: idx,
                                    end: idx + 1,
                                },
                            }
                        }
                    } else {
                        Token {
                            kind: TokenKind::Underscore,
                            span: Span {
                                start: idx,
                                end: idx + 1,
                            },
                        }
                    }
                }
                '"' => self.lex_string()?,
                c if is_ident_start(c) => {
                    self.bump();
                    self.lex_ident(idx, c)
                }
                c if c.is_ascii_digit() => self.lex_int()?,
                '/' => self.simple(idx, TokenKind::Slash),
                _ => {
                    return Err(LexError {
                        code: LexErrorCode::UnexpectedChar,
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

    fn skip_line_comment(&mut self) {
        self.bump();
        self.bump();
        while let Some((_, ch)) = self.peek() {
            self.bump();
            if ch == '\n' {
                break;
            }
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), LexError> {
        let (start, _) = self.bump().expect("peeked before bump");
        self.bump();
        while let Some((idx, ch)) = self.bump() {
            if ch == '*' && self.peek_char() == Some('/') {
                self.bump();
                return Ok(());
            }
            self.last_end = idx + ch.len_utf8();
        }
        Err(LexError {
            code: LexErrorCode::UnterminatedBlockComment,
            span: Span {
                start,
                end: self.last_end,
            },
            message: "unterminated block comment".to_string(),
        })
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

    fn lex_ident(&mut self, start: usize, first: char) -> Token {
        let mut name = String::from(first);
        while let Some((_, ch)) = self.peek() {
            if is_ident_continue(ch) {
                let (_, consumed) = self.bump().expect("peeked before bump");
                name.push(consumed);
            } else {
                break;
            }
        }

        let kind = TokenKind::Ident(name);
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
        if first == '0' {
            if let Some((idx, next)) = self.peek() {
                if next.is_ascii_digit() {
                    return Err(LexError {
                        code: LexErrorCode::InvalidIntLeadingZero,
                        span: Span {
                            start,
                            end: idx + 1,
                        },
                        message: "leading zeros are not allowed".to_string(),
                    });
                }
            }
            return Ok(Token {
                kind: TokenKind::Int(0),
                span: Span {
                    start,
                    end: start + 1,
                },
            });
        }

        while let Some((_, ch)) = self.peek() {
            if ch.is_ascii_digit() {
                self.bump();
            } else {
                break;
            }
        }

        let text = &self.src[start..self.last_end];
        let value = text.parse::<i64>().map_err(|_| LexError {
            code: LexErrorCode::IntOutOfRange,
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
                    let (_, esc) = self.bump().ok_or(LexError {
                        code: LexErrorCode::UnterminatedEscape,
                        span: Span {
                            start,
                            end: self.last_end,
                        },
                        message: "unterminated escape sequence".to_string(),
                    })?;
                    match esc {
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        other => {
                            return Err(LexError {
                                code: LexErrorCode::InvalidEscape,
                                span: Span {
                                    start: idx,
                                    end: self.last_end,
                                },
                                message: format!("invalid escape `\\{other}`"),
                            });
                        }
                    }
                }
                '\n' => {
                    return Err(LexError {
                        code: LexErrorCode::UnterminatedString,
                        span: Span {
                            start,
                            end: idx,
                        },
                        message: "unterminated string literal".to_string(),
                    });
                }
                c => value.push(c),
            }
        }

        Err(LexError {
            code: LexErrorCode::UnterminatedString,
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
        let mut it = self.chars.clone();
        it.next()?;
        it.next().map(|(_, ch)| ch)
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
