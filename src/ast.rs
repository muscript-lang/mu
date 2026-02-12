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
    pub module: Module,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub mod_id: ModId,
    pub decls: Vec<Decl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModId {
    pub parts: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Import(ImportDecl),
    Export(ExportDecl),
    Type(TypeDecl),
    Value(ValueDecl),
    Function(FunctionDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub alias: Ident,
    pub module: ModId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportDecl {
    pub names: Vec<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDecl {
    pub name: Ident,
    pub params: Vec<Ident>,
    pub ctors: Vec<CtorDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CtorDecl {
    pub name: Ident,
    pub fields: Vec<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueDecl {
    pub name: Ident,
    pub ty: TypeExpr,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: Ident,
    pub type_params: Vec<Ident>,
    pub sig: FunctionType,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimType {
    Bool,
    String,
    I32,
    I64,
    U32,
    U64,
    F32,
    F64,
    Unit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Prim(PrimType, Span),
    Named {
        name: Ident,
        args: Vec<TypeExpr>,
        span: Span,
    },
    Optional {
        inner: Box<TypeExpr>,
        span: Span,
    },
    Array {
        inner: Box<TypeExpr>,
        span: Span,
    },
    Map {
        key: Box<TypeExpr>,
        value: Box<TypeExpr>,
        span: Span,
    },
    Tuple {
        items: Vec<TypeExpr>,
        span: Span,
    },
    Function {
        sig: FunctionType,
        span: Span,
    },
    ResultSugar {
        ok: Box<TypeExpr>,
        err: Box<TypeExpr>,
        span: Span,
    },
    Group {
        inner: Box<TypeExpr>,
        span: Span,
    },
}

impl TypeExpr {
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Prim(_, span)
            | TypeExpr::Named { span, .. }
            | TypeExpr::Optional { span, .. }
            | TypeExpr::Array { span, .. }
            | TypeExpr::Map { span, .. }
            | TypeExpr::Tuple { span, .. }
            | TypeExpr::Function { span, .. }
            | TypeExpr::ResultSugar { span, .. }
            | TypeExpr::Group { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub params: Vec<TypeExpr>,
    pub ret: Box<TypeExpr>,
    pub effects: EffectSet,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EffectAtom {
    Io,
    Fs,
    Net,
    Proc,
    Rand,
    Time,
    St,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EffectSet {
    pub atoms: Vec<EffectAtom>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64, Span),
    Bool(bool, Span),
    String(String, Span),
}

impl Literal {
    pub fn span(&self) -> Span {
        match self {
            Literal::Int(_, span) | Literal::Bool(_, span) | Literal::String(_, span) => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Block {
        prefix: Vec<Expr>,
        tail: Box<Expr>,
        span: Span,
    },
    Unit(Span),
    Let {
        name: Ident,
        ty: Option<TypeExpr>,
        value: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
        span: Span,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    Lambda {
        params: Vec<Param>,
        ret: TypeExpr,
        effects: EffectSet,
        body: Box<Expr>,
        span: Span,
    },
    Assert {
        cond: Box<Expr>,
        msg: Option<Box<Expr>>,
        span: Span,
    },
    Require {
        expr: Box<Expr>,
        span: Span,
    },
    Ensure {
        expr: Box<Expr>,
        span: Span,
    },
    Name(Ident),
    NameApp {
        name: Ident,
        args: Vec<Expr>,
        span: Span,
    },
    Literal(Literal),
    Paren {
        inner: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Block { span, .. }
            | Expr::Unit(span)
            | Expr::Let { span, .. }
            | Expr::If { span, .. }
            | Expr::Match { span, .. }
            | Expr::Call { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::Assert { span, .. }
            | Expr::Require { span, .. }
            | Expr::Ensure { span, .. }
            | Expr::NameApp { span, .. }
            | Expr::Paren { span, .. } => *span,
            Expr::Name(name) => name.span,
            Expr::Literal(lit) => lit.span(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: Ident,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard(Span),
    Literal(Literal),
    Name(Ident),
    Ctor {
        name: Ident,
        args: Vec<Pattern>,
        span: Span,
    },
    Tuple {
        items: Vec<Pattern>,
        span: Span,
    },
    Paren {
        inner: Box<Pattern>,
        span: Span,
    },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard(span)
            | Pattern::Ctor { span, .. }
            | Pattern::Tuple { span, .. }
            | Pattern::Paren { span, .. } => *span,
            Pattern::Literal(lit) => lit.span(),
            Pattern::Name(id) => id.span,
        }
    }
}
