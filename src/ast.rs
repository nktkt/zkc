use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub circuit: Circuit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Circuit {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Input(InputDecl),
    Statement(Statement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDecl {
    pub visibility: Visibility,
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Field,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Let(LetStmt),
    Constrain(ConstrainStmt),
    Expose(ExposeStmt),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetStmt {
    pub name: String,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstrainStmt {
    pub lhs: Expr,
    pub rhs: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExposeStmt {
    pub expr: Expr,
    pub label: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Number {
        value: i128,
        span: Span,
    },
    Ident {
        name: String,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Self::Number { span, .. }
            | Self::Ident { span, .. }
            | Self::Unary { span, .. }
            | Self::Binary { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
}

impl BinaryOp {
    pub fn mnemonic(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Mul => "mul",
        }
    }
}
