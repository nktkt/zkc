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
    Include(IncludeDecl),
    Import(ImportDecl),
    Input(InputDecl),
    Function(FunctionDecl),
    Statement(Statement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeDecl {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDecl {
    pub path: String,
    pub alias: String,
    pub span: Span,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Field,
    Bool,
    U8,
    U16,
    U32,
}

impl Type {
    pub fn name(self) -> &'static str {
        match self {
            Self::Field => "field",
            Self::Bool => "bool",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
        }
    }

    pub fn uint_bits(self) -> Option<u8> {
        match self {
            Self::U8 => Some(8),
            Self::U16 => Some(16),
            Self::U32 => Some(32),
            Self::Field | Self::Bool => None,
        }
    }

    pub fn is_uint(self) -> bool {
        self.uint_bits().is_some()
    }
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
    Bool {
        value: bool,
        span: Span,
    },
    Ident {
        name: String,
        span: Span,
    },
    Call {
        callee: Vec<String>,
        args: Vec<Expr>,
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
    IfElse {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Self::Number { span, .. }
            | Self::Bool { span, .. }
            | Self::Ident { span, .. }
            | Self::Call { span, .. }
            | Self::Unary { span, .. }
            | Self::Binary { span, .. }
            | Self::IfElse { span, .. } => *span,
        }
    }
}

pub fn format_callee(path: &[String]) -> String {
    path.join("::")
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
