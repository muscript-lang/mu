#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Stmt(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: Ident,
        value: Expr,
        span: Span,
    },
    Expr {
        expr: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Ident(Ident),
    Int(i64, Span),
    String(String, Span),
    Bool(bool, Span),
    List(Vec<Expr>, Span),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    Binary {
        lhs: Box<Expr>,
        op: BinaryOp,
        rhs: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    EqEq,
}

impl BinaryOp {
    pub fn as_str(self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::EqEq => "==",
        }
    }
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Ident(id) => id.span,
            Expr::Int(_, span)
            | Expr::String(_, span)
            | Expr::Bool(_, span)
            | Expr::List(_, span)
            | Expr::Binary { span, .. }
            | Expr::Call { span, .. } => *span,
        }
    }
}
