use crate::ast::{BinaryOp, Type, UnaryOp, Visibility};
use crate::span::Span;

pub type BindingId = usize;

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
    Function(FunctionDecl),
    Statement(Statement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub id: BindingId,
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDecl {
    pub binding: Binding,
    pub visibility: Visibility,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Binding>,
    pub return_type: Type,
    pub body: TypedExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Let(LetStmt),
    Constrain(ConstrainStmt),
    Expose(ExposeStmt),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetStmt {
    pub binding: Binding,
    pub expr: TypedExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstrainStmt {
    pub lhs: TypedExpr,
    pub rhs: TypedExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExposeStmt {
    pub expr: TypedExpr,
    pub label: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedExpr {
    pub kind: ExprKind,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Constant(i128),
    BoolConstant(bool),
    Reference(Binding),
    Unary {
        op: UnaryOp,
        expr: Box<TypedExpr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<TypedExpr>,
        rhs: Box<TypedExpr>,
    },
    IfElse {
        condition: Box<TypedExpr>,
        then_branch: Box<TypedExpr>,
        else_branch: Box<TypedExpr>,
    },
    BoolNot {
        expr: Box<TypedExpr>,
    },
    BoolAnd {
        lhs: Box<TypedExpr>,
        rhs: Box<TypedExpr>,
    },
    BoolOr {
        lhs: Box<TypedExpr>,
        rhs: Box<TypedExpr>,
    },
    BoolXor {
        lhs: Box<TypedExpr>,
        rhs: Box<TypedExpr>,
    },
    Cast {
        expr: Box<TypedExpr>,
        target: Type,
    },
}

impl TypedExpr {
    pub fn field(kind: ExprKind, span: Span) -> Self {
        Self {
            kind,
            ty: Type::Field,
            span,
        }
    }

    pub fn bool(kind: ExprKind, span: Span) -> Self {
        Self {
            kind,
            ty: Type::Bool,
            span,
        }
    }

    pub fn new(kind: ExprKind, ty: Type, span: Span) -> Self {
        Self { kind, ty, span }
    }
}
