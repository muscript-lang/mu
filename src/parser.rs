use std::fmt;

use crate::ast::{
    CtorDecl, Decl, EffectAtom, EffectSet, ExportDecl, Expr, FunctionDecl, FunctionType, Ident,
    ImportDecl, Literal, MatchArm, ModId, Module, Param, Pattern, PrimType, Program, Span,
    TypeDecl, TypeExpr, ValueDecl,
};
use crate::lexer::{LexError, Token, TokenKind, tokenize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorCode {
    UnexpectedToken,
    ExpectedToken,
    ExpectedIdent,
    ExpectedType,
    ExpectedExpr,
}

impl ParseErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ParseErrorCode::UnexpectedToken => "E2001",
            ParseErrorCode::ExpectedToken => "E2002",
            ParseErrorCode::ExpectedIdent => "E2003",
            ParseErrorCode::ExpectedType => "E2004",
            ParseErrorCode::ExpectedExpr => "E2005",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub code: ParseErrorCode,
    pub span: Span,
    pub message: String,
}

impl fmt::Display for ParseError {
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

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(value: LexError) -> Self {
        ParseError {
            code: ParseErrorCode::UnexpectedToken,
            span: value.span,
            message: format!("{}: {}", value.code.as_str(), value.message),
        }
    }
}

pub fn parse_str(src: &str) -> Result<Program, ParseError> {
    let tokens = tokenize(src)?;
    let mut p = Parser { tokens, pos: 0 };
    p.parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let module = self.parse_module()?;
        self.expect_simple(TokenKind::Eof, "expected end of file")?;
        Ok(Program { module })
    }

    fn parse_module(&mut self) -> Result<Module, ParseError> {
        let start = self.expect_simple(TokenKind::At, "expected `@` for module start")?;
        let mod_id = self.parse_mod_id()?;
        self.expect_simple(TokenKind::LBrace, "expected `{` after module id")?;
        let mut decls = Vec::new();
        while !self.at_simple(TokenKind::RBrace) {
            decls.push(self.parse_decl()?);
        }
        let end = self.expect_simple(TokenKind::RBrace, "expected `}` to close module")?;
        Ok(Module {
            mod_id,
            decls,
            span: start.span.merge(end.span),
        })
    }

    fn parse_mod_id(&mut self) -> Result<ModId, ParseError> {
        let first = self.expect_ident("expected module identifier")?;
        let mut parts = vec![first.name];
        let mut span = first.span;
        while self.at_simple(TokenKind::Dot) {
            self.bump();
            let part = self.expect_ident("expected identifier after `.`")?;
            span = span.merge(part.span);
            parts.push(part.name);
        }
        Ok(ModId { parts, span })
    }

    fn parse_decl(&mut self) -> Result<Decl, ParseError> {
        if self.at_simple(TokenKind::Colon) {
            return self.parse_import_decl().map(Decl::Import);
        }
        if self.at_ident_text("E") {
            return self.parse_export_decl().map(Decl::Export);
        }
        if self.at_ident_text("T") {
            return self.parse_type_decl().map(Decl::Type);
        }
        if self.at_ident_text("V") {
            return self.parse_value_decl().map(Decl::Value);
        }
        if self.at_ident_text("F") {
            return self.parse_function_decl().map(Decl::Function);
        }
        Err(ParseError {
            code: ParseErrorCode::UnexpectedToken,
            span: self.peek().span,
            message: "expected declaration".to_string(),
        })
    }

    fn parse_import_decl(&mut self) -> Result<ImportDecl, ParseError> {
        let start = self.expect_simple(TokenKind::Colon, "expected `:` for import")?;
        let alias = self.expect_ident("expected import alias")?;
        self.expect_simple(TokenKind::Eq, "expected `=` in import")?;
        let module = self.parse_mod_id()?;
        let end = self.expect_simple(TokenKind::Semicolon, "expected `;` after import")?;
        Ok(ImportDecl {
            alias,
            module,
            span: start.span.merge(end.span),
        })
    }

    fn parse_export_decl(&mut self) -> Result<ExportDecl, ParseError> {
        let start = self.expect_ident_text("E", "expected `E`")?;
        self.expect_simple(TokenKind::LBracket, "expected `[` after E")?;
        let mut names = Vec::new();
        if !self.at_simple(TokenKind::RBracket) {
            names = self.parse_ident_list()?;
        }
        self.expect_simple(TokenKind::RBracket, "expected `]` in export decl")?;
        let end = self.expect_simple(TokenKind::Semicolon, "expected `;` after export")?;
        Ok(ExportDecl {
            names,
            span: start.span.merge(end.span),
        })
    }

    fn parse_type_decl(&mut self) -> Result<TypeDecl, ParseError> {
        let start = self.expect_ident_text("T", "expected `T`")?;
        let name = self.expect_ident("expected type name")?;
        let params = if self.at_simple(TokenKind::LBracket) {
            self.parse_type_params()?
        } else {
            Vec::new()
        };
        self.expect_simple(TokenKind::Eq, "expected `=` in type declaration")?;
        let mut ctors = vec![self.parse_ctor_decl()?];
        while self.at_simple(TokenKind::Pipe) {
            self.bump();
            ctors.push(self.parse_ctor_decl()?);
        }
        let end = self.expect_simple(TokenKind::Semicolon, "expected `;` after type declaration")?;
        Ok(TypeDecl {
            name,
            params,
            ctors,
            span: start.span.merge(end.span),
        })
    }

    fn parse_ctor_decl(&mut self) -> Result<CtorDecl, ParseError> {
        let name = self.expect_ident("expected constructor name")?;
        if !self.at_simple(TokenKind::LParen) {
            return Ok(CtorDecl {
                span: name.span,
                name,
                fields: Vec::new(),
            });
        }
        self.bump();
        let mut fields = Vec::new();
        if !self.at_simple(TokenKind::RParen) {
            fields = self.parse_type_list()?;
        }
        let end = self.expect_simple(TokenKind::RParen, "expected `)` in constructor fields")?;
        Ok(CtorDecl {
            span: name.span.merge(end.span),
            name,
            fields,
        })
    }

    fn parse_value_decl(&mut self) -> Result<ValueDecl, ParseError> {
        let start = self.expect_ident_text("V", "expected `V`")?;
        let name = self.expect_ident("expected value name")?;
        self.expect_simple(TokenKind::Colon, "expected `:` after value name")?;
        let ty = self.parse_type()?;
        self.expect_simple(TokenKind::Eq, "expected `=` after value type")?;
        let expr = self.parse_expr()?;
        let end = self.expect_simple(TokenKind::Semicolon, "expected `;` after value declaration")?;
        Ok(ValueDecl {
            name,
            ty,
            expr,
            span: start.span.merge(end.span),
        })
    }

    fn parse_function_decl(&mut self) -> Result<FunctionDecl, ParseError> {
        let start = self.expect_ident_text("F", "expected `F`")?;
        let name = self.expect_ident("expected function name")?;
        let type_params = if self.at_simple(TokenKind::LBracket) {
            self.parse_type_params()?
        } else {
            Vec::new()
        };
        self.expect_simple(TokenKind::Colon, "expected `:` after function name")?;
        let sig = self.parse_function_type()?;
        self.expect_simple(TokenKind::Eq, "expected `=` in function declaration")?;
        let expr = self.parse_expr()?;
        let end =
            self.expect_simple(TokenKind::Semicolon, "expected `;` after function declaration")?;
        Ok(FunctionDecl {
            name,
            type_params,
            sig,
            expr,
            span: start.span.merge(end.span),
        })
    }

    fn parse_type_params(&mut self) -> Result<Vec<Ident>, ParseError> {
        self.expect_simple(TokenKind::LBracket, "expected `[` for type parameters")?;
        if self.at_simple(TokenKind::RBracket) {
            return Err(ParseError {
                code: ParseErrorCode::ExpectedIdent,
                span: self.peek().span,
                message: "expected type parameter identifier".to_string(),
            });
        }
        let params = self.parse_ident_list()?;
        self.expect_simple(TokenKind::RBracket, "expected `]` after type parameters")?;
        Ok(params)
    }

    fn parse_ident_list(&mut self) -> Result<Vec<Ident>, ParseError> {
        let mut out = vec![self.expect_ident("expected identifier")?];
        while self.at_simple(TokenKind::Comma) {
            self.bump();
            out.push(self.expect_ident("expected identifier after `,`")?);
        }
        Ok(out)
    }

    fn parse_type_list(&mut self) -> Result<Vec<TypeExpr>, ParseError> {
        let mut out = vec![self.parse_type()?];
        while self.at_simple(TokenKind::Comma) {
            self.bump();
            out.push(self.parse_type()?);
        }
        Ok(out)
    }

    fn parse_type(&mut self) -> Result<TypeExpr, ParseError> {
        let mut lhs = if self.at_simple(TokenKind::LParen) {
            self.parse_type_paren_or_tuple_or_function()?
        } else if self.at_simple(TokenKind::Question) {
            let start = self.bump().span;
            let inner = self.parse_type()?;
            TypeExpr::Optional {
                span: start.merge(inner.span()),
                inner: Box::new(inner),
            }
        } else if self.at_simple(TokenKind::LBrace) {
            let start = self.bump().span;
            let key = self.parse_type()?;
            self.expect_simple(TokenKind::Colon, "expected `:` in map type")?;
            let value = self.parse_type()?;
            let end = self.expect_simple(TokenKind::RBrace, "expected `}` in map type")?;
            TypeExpr::Map {
                key: Box::new(key),
                value: Box::new(value),
                span: start.merge(end.span),
            }
        } else {
            self.parse_named_or_prim_type()?
        };

        while self.at_simple(TokenKind::LBracket) {
            let open = self.bump();
            let close = self.expect_simple(TokenKind::RBracket, "expected `]` in array type")?;
            lhs = TypeExpr::Array {
                span: lhs.span().merge(open.span).merge(close.span),
                inner: Box::new(lhs),
            };
        }

        if self.at_simple(TokenKind::Bang) {
            if self.lookahead_is_simple(1, TokenKind::LBrace) {
                return Ok(lhs);
            }
            self.bump();
            let rhs = self.parse_type_atom()?;
            let span = lhs.span().merge(rhs.span());
            lhs = TypeExpr::ResultSugar {
                ok: Box::new(lhs),
                err: Box::new(rhs),
                span,
            };
        }

        Ok(lhs)
    }

    fn parse_type_atom(&mut self) -> Result<TypeExpr, ParseError> {
        if self.at_simple(TokenKind::Question) {
            let start = self.bump().span;
            let inner = self.parse_type_atom()?;
            return Ok(TypeExpr::Optional {
                span: start.merge(inner.span()),
                inner: Box::new(inner),
            });
        }
        if self.at_simple(TokenKind::LBrace) {
            let start = self.bump().span;
            let key = self.parse_type()?;
            self.expect_simple(TokenKind::Colon, "expected `:` in map type")?;
            let value = self.parse_type()?;
            let end = self.expect_simple(TokenKind::RBrace, "expected `}` in map type")?;
            return Ok(TypeExpr::Map {
                key: Box::new(key),
                value: Box::new(value),
                span: start.merge(end.span),
            });
        }
        if self.at_simple(TokenKind::LParen) {
            let start = self.bump().span;
            let inner = self.parse_type()?;
            let end = self.expect_simple(TokenKind::RParen, "expected `)` in grouped type")?;
            return Ok(TypeExpr::Group {
                inner: Box::new(inner),
                span: start.merge(end.span),
            });
        }
        let mut base = self.parse_named_or_prim_type()?;
        while self.at_simple(TokenKind::LBracket) {
            let open = self.bump();
            let close = self.expect_simple(TokenKind::RBracket, "expected `]` in array type")?;
            base = TypeExpr::Array {
                span: base.span().merge(open.span).merge(close.span),
                inner: Box::new(base),
            };
        }
        Ok(base)
    }

    fn parse_type_paren_or_tuple_or_function(&mut self) -> Result<TypeExpr, ParseError> {
        let start = self.expect_simple(TokenKind::LParen, "expected `(`")?;
        if self.at_simple(TokenKind::RParen) {
            let close = self.bump();
            if self.at_simple(TokenKind::Arrow) {
                let sig = self.finish_function_type(start.span, close.span, Vec::new())?;
                let span = sig.span;
                return Ok(TypeExpr::Function { sig, span });
            }
            return Err(ParseError {
                code: ParseErrorCode::ExpectedType,
                span: start.span.merge(close.span),
                message: "empty tuple type is not allowed".to_string(),
            });
        }

        let first = self.parse_type()?;
        if self.at_simple(TokenKind::Comma) {
            self.bump();
            let second = self.parse_type()?;
            let mut items = vec![first, second];
            while self.at_simple(TokenKind::Comma) {
                self.bump();
                items.push(self.parse_type()?);
            }
            let close = self.expect_simple(TokenKind::RParen, "expected `)` in tuple type")?;
            if self.at_simple(TokenKind::Arrow) {
                let sig = self.finish_function_type(start.span, close.span, items)?;
                let span = sig.span;
                return Ok(TypeExpr::Function { sig, span });
            }
            return Ok(TypeExpr::Tuple {
                span: start.span.merge(close.span),
                items,
            });
        }

        let close = self.expect_simple(TokenKind::RParen, "expected `)`")?;
        if self.at_simple(TokenKind::Arrow) {
            let sig = self.finish_function_type(start.span, close.span, vec![first])?;
            let span = sig.span;
            return Ok(TypeExpr::Function { sig, span });
        }
        Ok(TypeExpr::Group {
            span: start.span.merge(close.span),
            inner: Box::new(first),
        })
    }

    fn parse_function_type(&mut self) -> Result<FunctionType, ParseError> {
        let open = self.expect_simple(TokenKind::LParen, "expected `(` in function type")?;
        let mut params = Vec::new();
        if !self.at_simple(TokenKind::RParen) {
            params = self.parse_type_list()?;
        }
        let close = self.expect_simple(TokenKind::RParen, "expected `)` in function type")?;
        self.finish_function_type(open.span, close.span, params)
    }

    fn finish_function_type(
        &mut self,
        open: Span,
        close: Span,
        params: Vec<TypeExpr>,
    ) -> Result<FunctionType, ParseError> {
        self.expect_simple(TokenKind::Arrow, "expected `->` in function type")?;
        let ret = self.parse_type()?;
        let effects = if self.at_simple(TokenKind::Bang) {
            self.parse_effect_set()?
        } else {
            EffectSet::default()
        };
        let span = open.merge(close).merge(ret.span());
        Ok(FunctionType {
            params,
            ret: Box::new(ret),
            effects,
            span,
        })
    }

    fn parse_effect_set(&mut self) -> Result<EffectSet, ParseError> {
        self.expect_simple(TokenKind::Bang, "expected `!` for effect set")?;
        self.expect_simple(TokenKind::LBrace, "expected `{` in effect set")?;
        let mut atoms = vec![self.parse_effect_atom()?];
        while self.at_simple(TokenKind::Comma) {
            self.bump();
            atoms.push(self.parse_effect_atom()?);
        }
        self.expect_simple(TokenKind::RBrace, "expected `}` in effect set")?;
        Ok(EffectSet { atoms })
    }

    fn parse_effect_atom(&mut self) -> Result<EffectAtom, ParseError> {
        let ident = self.expect_ident("expected effect atom")?;
        match ident.name.as_str() {
            "io" => Ok(EffectAtom::Io),
            "fs" => Ok(EffectAtom::Fs),
            "net" => Ok(EffectAtom::Net),
            "proc" => Ok(EffectAtom::Proc),
            "rand" => Ok(EffectAtom::Rand),
            "time" => Ok(EffectAtom::Time),
            "st" => Ok(EffectAtom::St),
            _ => Err(ParseError {
                code: ParseErrorCode::ExpectedToken,
                span: ident.span,
                message: format!("unknown effect atom `{}`", ident.name),
            }),
        }
    }

    fn parse_named_or_prim_type(&mut self) -> Result<TypeExpr, ParseError> {
        let name = self.expect_ident("expected type")?;
        let prim = match name.name.as_str() {
            "b" => Some(PrimType::Bool),
            "s" => Some(PrimType::String),
            "i32" => Some(PrimType::I32),
            "i64" => Some(PrimType::I64),
            "u32" => Some(PrimType::U32),
            "u64" => Some(PrimType::U64),
            "f32" => Some(PrimType::F32),
            "f64" => Some(PrimType::F64),
            "unit" => Some(PrimType::Unit),
            _ => None,
        };
        if let Some(p) = prim {
            return Ok(TypeExpr::Prim(p, name.span));
        }
        let mut args = Vec::new();
        let mut span = name.span;
        if self.at_simple(TokenKind::LBracket) && !self.lookahead_is_simple(1, TokenKind::RBracket) {
            self.bump();
            args = self.parse_type_list()?;
            let close = self.expect_simple(TokenKind::RBracket, "expected `]` in type args")?;
            span = span.merge(close.span);
        }
        Ok(TypeExpr::Named { name, args, span })
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        if self.at_simple(TokenKind::LBrace) {
            return self.parse_block_expr();
        }
        if self.at_ident_text("v") && self.lookahead_is_simple(1, TokenKind::LParen) {
            return self.parse_let_expr();
        }
        if self.at_ident_text("i") && self.lookahead_is_simple(1, TokenKind::LParen) {
            return self.parse_if_expr();
        }
        if self.at_ident_text("m") && self.lookahead_is_simple(1, TokenKind::LParen) {
            return self.parse_match_expr();
        }
        if self.at_ident_text("c") && self.lookahead_is_simple(1, TokenKind::LParen) {
            return self.parse_call_expr();
        }
        if self.at_ident_text("l") && self.lookahead_is_simple(1, TokenKind::LParen) {
            return self.parse_lambda_expr();
        }
        if self.at_ident_text("a") && self.lookahead_is_simple(1, TokenKind::LParen) {
            return self.parse_assert_expr();
        }
        if self.at_simple(TokenKind::Underscore) {
            let start = self.bump();
            let expr = self.parse_expr()?;
            let span = start.span.merge(expr.span());
            return Ok(Expr::Ensure {
                expr: Box::new(expr),
                span,
            });
        }
        if self.at_simple(TokenKind::Caret) {
            let start = self.bump();
            let expr = self.parse_expr()?;
            let span = start.span.merge(expr.span());
            return Ok(Expr::Require {
                expr: Box::new(expr),
                span,
            });
        }
        if self.at_simple(TokenKind::LParen) {
            let open = self.bump();
            if self.at_simple(TokenKind::RParen) {
                let close = self.bump();
                return Ok(Expr::Unit(open.span.merge(close.span)));
            }
            let inner = self.parse_expr()?;
            let close = self.expect_simple(TokenKind::RParen, "expected `)`")?;
            return Ok(Expr::Paren {
                inner: Box::new(inner),
                span: open.span.merge(close.span),
            });
        }
        if self.at_simple(TokenKind::Int(0))
            || self.at_simple(TokenKind::String(String::new()))
            || self.at_ident_text("t")
            || self.at_ident_text("f")
        {
            return Ok(Expr::Literal(self.parse_literal()?));
        }
        if self.at_simple(TokenKind::Plus)
            || self.at_simple(TokenKind::Minus)
            || self.at_simple(TokenKind::Star)
            || self.at_simple(TokenKind::Slash)
            || self.at_simple(TokenKind::Percent)
            || self.at_simple(TokenKind::EqEq)
            || self.at_simple(TokenKind::NotEq)
            || self.at_simple(TokenKind::Lt)
            || self.at_simple(TokenKind::Le)
            || self.at_simple(TokenKind::Gt)
            || self.at_simple(TokenKind::Ge)
        {
            return self.parse_symbol_name_expr();
        }
        if matches!(self.peek().kind, TokenKind::Ident(_)) {
            return self.parse_name_or_name_app();
        }

        Err(ParseError {
            code: ParseErrorCode::ExpectedExpr,
            span: self.peek().span,
            message: "expected expression".to_string(),
        })
    }

    fn parse_name_or_name_app(&mut self) -> Result<Expr, ParseError> {
        let name = self.expect_ident("expected identifier")?;
        if !self.at_simple(TokenKind::LParen) {
            return Ok(Expr::Name(name));
        }
        self.bump();
        let mut args = Vec::new();
        if !self.at_simple(TokenKind::RParen) {
            args = self.parse_expr_list()?;
        }
        let close = self.expect_simple(TokenKind::RParen, "expected `)` in name application")?;
        Ok(Expr::NameApp {
            span: name.span.merge(close.span),
            name,
            args,
        })
    }

    fn parse_symbol_name_expr(&mut self) -> Result<Expr, ParseError> {
        let token = self.bump();
        let name = match token.kind {
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Percent => "%",
            TokenKind::EqEq => "==",
            TokenKind::NotEq => "!=",
            TokenKind::Lt => "<",
            TokenKind::Le => "<=",
            TokenKind::Gt => ">",
            TokenKind::Ge => ">=",
            _ => {
                return Err(ParseError {
                    code: ParseErrorCode::ExpectedExpr,
                    span: token.span,
                    message: "expected symbolic operator".to_string(),
                });
            }
        };
        Ok(Expr::Name(Ident {
            name: name.to_string(),
            span: token.span,
        }))
    }

    fn parse_expr_list(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut out = vec![self.parse_expr()?];
        while self.at_simple(TokenKind::Comma) {
            self.bump();
            out.push(self.parse_expr()?);
        }
        Ok(out)
    }

    fn parse_block_expr(&mut self) -> Result<Expr, ParseError> {
        let open = self.expect_simple(TokenKind::LBrace, "expected `{` for block")?;
        let mut prefix = Vec::new();
        let tail = loop {
            let e = self.parse_expr()?;
            if self.at_simple(TokenKind::Semicolon) {
                self.bump();
                prefix.push(e);
            } else {
                break e;
            }
        };
        let close = self.expect_simple(TokenKind::RBrace, "expected `}` to close block")?;
        Ok(Expr::Block {
            prefix,
            tail: Box::new(tail),
            span: open.span.merge(close.span),
        })
    }

    fn parse_let_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect_ident_text("v", "expected `v`")?;
        self.expect_simple(TokenKind::LParen, "expected `(` in let expression")?;
        let name = self.expect_ident("expected let binding name")?;
        let ty = if self.at_simple(TokenKind::Colon) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect_simple(TokenKind::Eq, "expected `=` in let expression")?;
        let value = self.parse_expr()?;
        self.expect_simple(TokenKind::Comma, "expected `,` in let expression")?;
        let body = self.parse_expr()?;
        let close = self.expect_simple(TokenKind::RParen, "expected `)` in let expression")?;
        Ok(Expr::Let {
            name,
            ty,
            value: Box::new(value),
            body: Box::new(body),
            span: start.span.merge(close.span),
        })
    }

    fn parse_if_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect_ident_text("i", "expected `i`")?;
        self.expect_simple(TokenKind::LParen, "expected `(` in if expression")?;
        let cond = self.parse_expr()?;
        self.expect_simple(TokenKind::Comma, "expected `,` in if expression")?;
        let then_branch = self.parse_expr()?;
        self.expect_simple(TokenKind::Comma, "expected `,` in if expression")?;
        let else_branch = self.parse_expr()?;
        let close = self.expect_simple(TokenKind::RParen, "expected `)` in if expression")?;
        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
            span: start.span.merge(close.span),
        })
    }

    fn parse_match_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect_ident_text("m", "expected `m`")?;
        self.expect_simple(TokenKind::LParen, "expected `(` in match expression")?;
        let scrutinee = self.parse_expr()?;
        self.expect_simple(TokenKind::RParen, "expected `)` in match expression")?;
        self.expect_simple(TokenKind::LBrace, "expected `{` in match expression")?;
        let mut arms = Vec::new();
        while !self.at_simple(TokenKind::RBrace) {
            arms.push(self.parse_match_arm()?);
        }
        let close = self.expect_simple(TokenKind::RBrace, "expected `}` in match expression")?;
        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            span: start.span.merge(close.span),
        })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, ParseError> {
        let pattern = self.parse_pattern()?;
        self.expect_simple(TokenKind::FatArrow, "expected `=>` in match arm")?;
        let expr = self.parse_expr()?;
        let semi = self.expect_simple(TokenKind::Semicolon, "expected `;` after match arm")?;
        Ok(MatchArm {
            span: pattern.span().merge(semi.span),
            pattern,
            expr,
        })
    }

    fn parse_call_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect_ident_text("c", "expected `c`")?;
        self.expect_simple(TokenKind::LParen, "expected `(` in call expression")?;
        let callee = self.parse_expr()?;
        let mut args = Vec::new();
        if self.at_simple(TokenKind::Comma) {
            self.bump();
            args = self.parse_expr_list()?;
        }
        let close = self.expect_simple(TokenKind::RParen, "expected `)` in call expression")?;
        Ok(Expr::Call {
            callee: Box::new(callee),
            args,
            span: start.span.merge(close.span),
        })
    }

    fn parse_lambda_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect_ident_text("l", "expected `l`")?;
        self.expect_simple(TokenKind::LParen, "expected `(` in lambda")?;
        let mut params = vec![self.parse_param()?];
        while self.at_simple(TokenKind::Comma) {
            self.bump();
            params.push(self.parse_param()?);
        }
        self.expect_simple(TokenKind::RParen, "expected `)` in lambda params")?;
        self.expect_simple(TokenKind::Colon, "expected `:` before lambda return type")?;
        let ret = self.parse_type()?;
        let effects = if self.at_simple(TokenKind::Bang) {
            self.parse_effect_set()?
        } else {
            EffectSet::default()
        };
        self.expect_simple(TokenKind::Eq, "expected `=` in lambda")?;
        let body = self.parse_expr()?;
        Ok(Expr::Lambda {
            params,
            ret,
            effects,
            span: start.span.merge(body.span()),
            body: Box::new(body),
        })
    }

    fn parse_param(&mut self) -> Result<Param, ParseError> {
        let name = self.expect_ident("expected parameter name")?;
        self.expect_simple(TokenKind::Colon, "expected `:` in parameter")?;
        let ty = self.parse_type()?;
        Ok(Param {
            span: name.span.merge(ty.span()),
            name,
            ty,
        })
    }

    fn parse_assert_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect_ident_text("a", "expected `a`")?;
        self.expect_simple(TokenKind::LParen, "expected `(` in assert")?;
        let cond = self.parse_expr()?;
        let msg = if self.at_simple(TokenKind::Comma) {
            self.bump();
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        let close = self.expect_simple(TokenKind::RParen, "expected `)` in assert")?;
        Ok(Expr::Assert {
            cond: Box::new(cond),
            msg,
            span: start.span.merge(close.span),
        })
    }

    fn parse_literal(&mut self) -> Result<Literal, ParseError> {
        let token = self.bump();
        match token.kind {
            TokenKind::Int(v) => Ok(Literal::Int(v, token.span)),
            TokenKind::Ident(name) if name == "t" => Ok(Literal::Bool(true, token.span)),
            TokenKind::Ident(name) if name == "f" => Ok(Literal::Bool(false, token.span)),
            TokenKind::String(v) => Ok(Literal::String(v, token.span)),
            _ => Err(ParseError {
                code: ParseErrorCode::ExpectedExpr,
                span: token.span,
                message: "expected literal".to_string(),
            }),
        }
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        if self.at_simple(TokenKind::Underscore) {
            return Ok(Pattern::Wildcard(self.bump().span));
        }
        if self.at_simple(TokenKind::Int(0))
            || self.at_simple(TokenKind::String(String::new()))
            || self.at_ident_text("t")
            || self.at_ident_text("f")
        {
            return Ok(Pattern::Literal(self.parse_literal()?));
        }
        if self.at_simple(TokenKind::LParen) {
            let open = self.bump();
            let first = self.parse_pattern()?;
            if self.at_simple(TokenKind::Comma) {
                self.bump();
                let second = self.parse_pattern()?;
                let mut items = vec![first, second];
                while self.at_simple(TokenKind::Comma) {
                    self.bump();
                    items.push(self.parse_pattern()?);
                }
                let close = self.expect_simple(TokenKind::RParen, "expected `)` in tuple pattern")?;
                return Ok(Pattern::Tuple {
                    items,
                    span: open.span.merge(close.span),
                });
            }
            let close = self.expect_simple(TokenKind::RParen, "expected `)` in pattern")?;
            return Ok(Pattern::Paren {
                inner: Box::new(first),
                span: open.span.merge(close.span),
            });
        }

        let name = self.expect_ident("expected pattern")?;
        if !self.at_simple(TokenKind::LParen) {
            return Ok(Pattern::Name(name));
        }
        self.bump();
        let mut args = Vec::new();
        if !self.at_simple(TokenKind::RParen) {
            args = self.parse_pattern_list()?;
        }
        let close = self.expect_simple(TokenKind::RParen, "expected `)` in constructor pattern")?;
        Ok(Pattern::Ctor {
            span: name.span.merge(close.span),
            name,
            args,
        })
    }

    fn parse_pattern_list(&mut self) -> Result<Vec<Pattern>, ParseError> {
        let mut out = vec![self.parse_pattern()?];
        while self.at_simple(TokenKind::Comma) {
            self.bump();
            out.push(self.parse_pattern()?);
        }
        Ok(out)
    }

    fn expect_ident(&mut self, message: &str) -> Result<Ident, ParseError> {
        let token = self.bump();
        if let TokenKind::Ident(name) = token.kind {
            return Ok(Ident {
                name,
                span: token.span,
            });
        }
        Err(ParseError {
            code: ParseErrorCode::ExpectedIdent,
            span: token.span,
            message: message.to_string(),
        })
    }

    fn expect_ident_text(&mut self, expected: &str, message: &str) -> Result<Ident, ParseError> {
        let ident = self.expect_ident(message)?;
        if ident.name == expected {
            return Ok(ident);
        }
        Err(ParseError {
            code: ParseErrorCode::ExpectedToken,
            span: ident.span,
            message: format!("expected `{expected}`"),
        })
    }

    fn expect_simple(&mut self, expected: TokenKind, message: &str) -> Result<Token, ParseError> {
        if self.at_simple(expected.clone()) {
            return Ok(self.bump());
        }
        Err(ParseError {
            code: ParseErrorCode::ExpectedToken,
            span: self.peek().span,
            message: message.to_string(),
        })
    }

    fn at_ident_text(&self, expected: &str) -> bool {
        matches!(&self.peek().kind, TokenKind::Ident(name) if name == expected)
    }

    fn lookahead_is_simple(&self, n: usize, expected: TokenKind) -> bool {
        if self.pos + n >= self.tokens.len() {
            return false;
        }
        use TokenKind::*;
        match (&self.tokens[self.pos + n].kind, &expected) {
            (Int(_), Int(_)) => true,
            (String(_), String(_)) => true,
            (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
        }
    }

    fn at_simple(&self, expected: TokenKind) -> bool {
        use TokenKind::*;
        match (&self.peek().kind, &expected) {
            (Int(_), Int(_)) => true,
            (String(_), String(_)) => true,
            (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
        }
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
