use std::fmt;

use crate::ast::{BinaryOp, Expr, Ident, Item, Program, Span, Stmt};
use crate::lexer::{LexError, Token, TokenKind, tokenize};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub span: Span,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(value: LexError) -> Self {
        ParseError {
            span: value.span,
            message: value.message,
        }
    }
}

pub fn parse_str(src: &str) -> Result<Program, ParseError> {
    let tokens = tokenize(src)?;
    let mut parser = Parser { tokens, pos: 0 };
    parser.parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();
        while !self.at(TokenKind::Eof) {
            let stmt = self.parse_stmt()?;
            items.push(Item::Stmt(stmt));
        }
        Ok(Program { items })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        if self.at(TokenKind::Let) {
            let start = self.bump().span;
            let name_tok = self.expect_ident()?;
            self.expect(TokenKind::Eq, "expected `=` after let binding name")?;
            let value = self.parse_expr(0)?;
            let semi = self.expect(TokenKind::Semicolon, "expected `;` after let binding")?;
            return Ok(Stmt::Let {
                name: Ident {
                    name: name_tok.0,
                    span: name_tok.1,
                },
                value,
                span: start.merge(semi.span),
            });
        }

        let expr = self.parse_expr(0)?;
        let semi = self.expect(TokenKind::Semicolon, "expected `;` after expression")?;
        Ok(Stmt::Expr {
            span: expr.span().merge(semi.span),
            expr,
        })
    }

    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_postfix()?;

        loop {
            let (op, left_bp, right_bp) = match self.peek().kind {
                TokenKind::EqEq => (BinaryOp::EqEq, 2, 3),
                TokenKind::Plus => (BinaryOp::Add, 4, 5),
                TokenKind::Minus => (BinaryOp::Sub, 4, 5),
                TokenKind::Star => (BinaryOp::Mul, 6, 7),
                TokenKind::Slash => (BinaryOp::Div, 6, 7),
                _ => break,
            };
            if left_bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.parse_expr(right_bp)?;
            let span = lhs.span().merge(rhs.span());
            lhs = Expr::Binary {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
                span,
            };
        }

        Ok(lhs)
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            if !self.at(TokenKind::LParen) {
                break;
            }
            let open = self.bump().span;
            let mut args = Vec::new();
            if !self.at(TokenKind::RParen) {
                loop {
                    args.push(self.parse_expr(0)?);
                    if self.at(TokenKind::Comma) {
                        self.bump();
                        continue;
                    }
                    break;
                }
            }
            let close = self.expect(TokenKind::RParen, "expected `)` to close call")?;
            expr = Expr::Call {
                span: expr.span().merge(open).merge(close.span),
                callee: Box::new(expr),
                args,
            };
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.bump();
        match token.kind {
            TokenKind::Ident(name) => Ok(Expr::Ident(Ident {
                name,
                span: token.span,
            })),
            TokenKind::Int(v) => Ok(Expr::Int(v, token.span)),
            TokenKind::String(v) => Ok(Expr::String(v, token.span)),
            TokenKind::True => Ok(Expr::Bool(true, token.span)),
            TokenKind::False => Ok(Expr::Bool(false, token.span)),
            TokenKind::LParen => {
                let expr = self.parse_expr(0)?;
                self.expect(TokenKind::RParen, "expected `)` to close group")?;
                Ok(expr)
            }
            TokenKind::LBracket => self.parse_list(token.span),
            other => Err(ParseError {
                span: token.span,
                message: format!("unexpected token {other:?} in expression"),
            }),
        }
    }

    fn parse_list(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        let mut values = Vec::new();
        if !self.at(TokenKind::RBracket) {
            loop {
                values.push(self.parse_expr(0)?);
                if self.at(TokenKind::Comma) {
                    self.bump();
                    if self.at(TokenKind::RBracket) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        let close = self.expect(TokenKind::RBracket, "expected `]` to close list")?;
        Ok(Expr::List(values, start_span.merge(close.span)))
    }

    fn expect_ident(&mut self) -> Result<(String, Span), ParseError> {
        let token = self.bump();
        if let TokenKind::Ident(name) = token.kind {
            return Ok((name, token.span));
        }
        Err(ParseError {
            span: token.span,
            message: "expected identifier".to_string(),
        })
    }

    fn expect(&mut self, expected: TokenKind, msg: &str) -> Result<Token, ParseError> {
        if self.at(expected.clone()) {
            return Ok(self.bump());
        }
        Err(ParseError {
            span: self.peek().span,
            message: msg.to_string(),
        })
    }

    fn at(&self, expected: TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(&expected)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if !matches!(token.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        token
    }
}
