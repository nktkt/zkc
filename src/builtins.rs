use crate::ast::{BinaryOp, Type, UnaryOp};
use crate::error::{CompileError, CompileResult};
use crate::hir::{ExprKind, TypedExpr};
use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinSpec {
    pub name: &'static str,
    pub arity: usize,
    pub signature: &'static str,
    pub description: &'static str,
}

const BUILTINS: &[BuiltinSpec] = &[
    BuiltinSpec {
        name: "square",
        arity: 1,
        signature: "square(value: field) -> field",
        description: "Returns value * value.",
    },
    BuiltinSpec {
        name: "cube",
        arity: 1,
        signature: "cube(value: field) -> field",
        description: "Returns value * value * value.",
    },
    BuiltinSpec {
        name: "double",
        arity: 1,
        signature: "double(value: field) -> field",
        description: "Returns value + value.",
    },
    BuiltinSpec {
        name: "triple",
        arity: 1,
        signature: "triple(value: field) -> field",
        description: "Returns value + value + value.",
    },
    BuiltinSpec {
        name: "quad",
        arity: 1,
        signature: "quad(value: field) -> field",
        description: "Returns value multiplied by four.",
    },
    BuiltinSpec {
        name: "negate",
        arity: 1,
        signature: "negate(value: field) -> field",
        description: "Returns the additive inverse of value.",
    },
    BuiltinSpec {
        name: "sum2",
        arity: 2,
        signature: "sum2(a: field, b: field) -> field",
        description: "Returns a + b.",
    },
    BuiltinSpec {
        name: "sum3",
        arity: 3,
        signature: "sum3(a: field, b: field, c: field) -> field",
        description: "Returns a + b + c.",
    },
    BuiltinSpec {
        name: "sum4",
        arity: 4,
        signature: "sum4(a: field, b: field, c: field, d: field) -> field",
        description: "Returns a + b + c + d.",
    },
    BuiltinSpec {
        name: "mul_add",
        arity: 3,
        signature: "mul_add(a: field, b: field, c: field) -> field",
        description: "Returns a * b + c.",
    },
    BuiltinSpec {
        name: "blend2",
        arity: 4,
        signature: "blend2(a: field, wa: field, b: field, wb: field) -> field",
        description: "Returns a * wa + b * wb.",
    },
    BuiltinSpec {
        name: "weighted_sum3",
        arity: 6,
        signature: "weighted_sum3(a: field, wa: field, b: field, wb: field, c: field, wc: field) -> field",
        description: "Returns a * wa + b * wb + c * wc.",
    },
    BuiltinSpec {
        name: "not",
        arity: 1,
        signature: "not(value: bool) -> bool",
        description: "Returns the boolean negation of value.",
    },
    BuiltinSpec {
        name: "and",
        arity: 2,
        signature: "and(lhs: bool, rhs: bool) -> bool",
        description: "Returns the boolean conjunction of lhs and rhs.",
    },
    BuiltinSpec {
        name: "or",
        arity: 2,
        signature: "or(lhs: bool, rhs: bool) -> bool",
        description: "Returns the boolean disjunction of lhs and rhs.",
    },
    BuiltinSpec {
        name: "xor",
        arity: 2,
        signature: "xor(lhs: bool, rhs: bool) -> bool",
        description: "Returns the boolean exclusive-or of lhs and rhs.",
    },
    BuiltinSpec {
        name: "choose",
        arity: 3,
        signature: "choose(cond: bool, when_true: field, when_false: field) -> field",
        description: "Returns when_true if cond is true, otherwise when_false.",
    },
    BuiltinSpec {
        name: "choose_bool",
        arity: 3,
        signature: "choose_bool(cond: bool, when_true: bool, when_false: bool) -> bool",
        description: "Returns when_true if cond is true, otherwise when_false.",
    },
    BuiltinSpec {
        name: "into_u8",
        arity: 1,
        signature: "into_u8(value: field) -> u8",
        description: "Range-checks a field expression and reinterprets it as u8.",
    },
    BuiltinSpec {
        name: "into_u16",
        arity: 1,
        signature: "into_u16(value: field) -> u16",
        description: "Range-checks a field expression and reinterprets it as u16.",
    },
    BuiltinSpec {
        name: "into_u32",
        arity: 1,
        signature: "into_u32(value: field) -> u32",
        description: "Range-checks a field expression and reinterprets it as u32.",
    },
    BuiltinSpec {
        name: "into_field",
        arity: 1,
        signature: "into_field(value: scalar) -> field",
        description: "Reinterprets bool and unsigned integer values as field elements.",
    },
];

pub fn all() -> &'static [BuiltinSpec] {
    BUILTINS
}

pub fn get(name: &str) -> Option<&'static BuiltinSpec> {
    BUILTINS.iter().find(|builtin| builtin.name == name)
}

pub fn contains(name: &str) -> bool {
    get(name).is_some()
}

pub fn expand(name: &str, args: &[TypedExpr], span: Span) -> CompileResult<TypedExpr> {
    let builtin = get(name)
        .ok_or_else(|| CompileError::new(span, format!("undeclared function `{name}`")))?;
    if args.len() != builtin.arity {
        return Err(CompileError::new(
            span,
            format!(
                "function `{name}` expects {} arguments but got {}",
                builtin.arity,
                args.len()
            ),
        ));
    }

    match name {
        "square" => {
            expect_all_types(name, args, Type::Field, span)?;
            Ok(mul(args[0].clone(), args[0].clone(), span))
        }
        "cube" => {
            expect_all_types(name, args, Type::Field, span)?;
            Ok(mul(
                mul(args[0].clone(), args[0].clone(), span),
                args[0].clone(),
                span,
            ))
        }
        "double" => {
            expect_all_types(name, args, Type::Field, span)?;
            Ok(add(args[0].clone(), args[0].clone(), span))
        }
        "triple" => {
            expect_all_types(name, args, Type::Field, span)?;
            Ok(add(
                add(args[0].clone(), args[0].clone(), span),
                args[0].clone(),
                span,
            ))
        }
        "quad" => {
            expect_all_types(name, args, Type::Field, span)?;
            Ok(add(
                add(args[0].clone(), args[0].clone(), span),
                add(args[0].clone(), args[0].clone(), span),
                span,
            ))
        }
        "negate" => {
            expect_all_types(name, args, Type::Field, span)?;
            Ok(field_unary(UnaryOp::Neg, args[0].clone(), span))
        }
        "sum2" => {
            expect_all_types(name, args, Type::Field, span)?;
            Ok(add(args[0].clone(), args[1].clone(), span))
        }
        "sum3" => {
            expect_all_types(name, args, Type::Field, span)?;
            Ok(add(
                add(args[0].clone(), args[1].clone(), span),
                args[2].clone(),
                span,
            ))
        }
        "sum4" => {
            expect_all_types(name, args, Type::Field, span)?;
            Ok(add(
                add(args[0].clone(), args[1].clone(), span),
                add(args[2].clone(), args[3].clone(), span),
                span,
            ))
        }
        "mul_add" => {
            expect_all_types(name, args, Type::Field, span)?;
            Ok(add(
                mul(args[0].clone(), args[1].clone(), span),
                args[2].clone(),
                span,
            ))
        }
        "blend2" => {
            expect_all_types(name, args, Type::Field, span)?;
            Ok(add(
                mul(args[0].clone(), args[1].clone(), span),
                mul(args[2].clone(), args[3].clone(), span),
                span,
            ))
        }
        "weighted_sum3" => {
            expect_all_types(name, args, Type::Field, span)?;
            Ok(add(
                add(
                    mul(args[0].clone(), args[1].clone(), span),
                    mul(args[2].clone(), args[3].clone(), span),
                    span,
                ),
                mul(args[4].clone(), args[5].clone(), span),
                span,
            ))
        }
        "not" => {
            expect_all_types(name, args, Type::Bool, span)?;
            Ok(bool_not(args[0].clone(), span))
        }
        "and" => {
            expect_all_types(name, args, Type::Bool, span)?;
            Ok(bool_and(args[0].clone(), args[1].clone(), span))
        }
        "or" => {
            expect_all_types(name, args, Type::Bool, span)?;
            Ok(bool_or(args[0].clone(), args[1].clone(), span))
        }
        "xor" => {
            expect_all_types(name, args, Type::Bool, span)?;
            Ok(bool_xor(args[0].clone(), args[1].clone(), span))
        }
        "choose" => {
            expect_types(name, args, &[Type::Bool, Type::Field, Type::Field], span)?;
            Ok(select(
                args[0].clone(),
                args[1].clone(),
                args[2].clone(),
                Type::Field,
                span,
            ))
        }
        "choose_bool" => {
            expect_types(name, args, &[Type::Bool, Type::Bool, Type::Bool], span)?;
            Ok(select(
                args[0].clone(),
                args[1].clone(),
                args[2].clone(),
                Type::Bool,
                span,
            ))
        }
        "into_u8" => {
            expect_types(name, args, &[Type::Field], span)?;
            Ok(cast(args[0].clone(), Type::U8, span))
        }
        "into_u16" => {
            expect_types(name, args, &[Type::Field], span)?;
            Ok(cast(args[0].clone(), Type::U16, span))
        }
        "into_u32" => {
            expect_types(name, args, &[Type::Field], span)?;
            Ok(cast(args[0].clone(), Type::U32, span))
        }
        "into_field" => match args[0].ty {
            Type::Field => Ok(args[0].clone()),
            Type::Bool | Type::U8 | Type::U16 | Type::U32 => {
                Ok(cast(args[0].clone(), Type::Field, span))
            }
        },
        _ => Err(CompileError::new(
            span,
            format!("builtin expansion missing for `{name}`"),
        )),
    }
}

fn expect_all_types(
    builtin: &str,
    args: &[TypedExpr],
    expected: Type,
    span: Span,
) -> CompileResult<()> {
    let expected_types = vec![expected; args.len()];
    expect_types(builtin, args, &expected_types, span)
}

fn expect_types(
    builtin: &str,
    args: &[TypedExpr],
    expected: &[Type],
    span: Span,
) -> CompileResult<()> {
    for (index, (arg, expected)) in args.iter().zip(expected.iter()).enumerate() {
        if arg.ty != *expected {
            return Err(CompileError::new(
                span,
                format!(
                    "function `{builtin}` expects argument {} to have type `{}` but got `{}`",
                    index + 1,
                    expected.name(),
                    arg.ty.name()
                ),
            ));
        }
    }
    Ok(())
}

fn add(lhs: TypedExpr, rhs: TypedExpr, span: Span) -> TypedExpr {
    field_binary(BinaryOp::Add, lhs, rhs, span)
}

fn mul(lhs: TypedExpr, rhs: TypedExpr, span: Span) -> TypedExpr {
    field_binary(BinaryOp::Mul, lhs, rhs, span)
}

fn field_binary(op: BinaryOp, lhs: TypedExpr, rhs: TypedExpr, span: Span) -> TypedExpr {
    TypedExpr::field(
        ExprKind::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        span,
    )
}

fn field_unary(op: UnaryOp, expr: TypedExpr, span: Span) -> TypedExpr {
    TypedExpr::field(
        ExprKind::Unary {
            op,
            expr: Box::new(expr),
        },
        span,
    )
}

fn bool_not(expr: TypedExpr, span: Span) -> TypedExpr {
    TypedExpr::bool(
        ExprKind::BoolNot {
            expr: Box::new(expr),
        },
        span,
    )
}

fn bool_and(lhs: TypedExpr, rhs: TypedExpr, span: Span) -> TypedExpr {
    TypedExpr::bool(
        ExprKind::BoolAnd {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        span,
    )
}

fn bool_or(lhs: TypedExpr, rhs: TypedExpr, span: Span) -> TypedExpr {
    TypedExpr::bool(
        ExprKind::BoolOr {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        span,
    )
}

fn bool_xor(lhs: TypedExpr, rhs: TypedExpr, span: Span) -> TypedExpr {
    TypedExpr::bool(
        ExprKind::BoolXor {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        span,
    )
}

fn select(
    condition: TypedExpr,
    then_branch: TypedExpr,
    else_branch: TypedExpr,
    ty: Type,
    span: Span,
) -> TypedExpr {
    TypedExpr {
        kind: ExprKind::IfElse {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        },
        ty,
        span,
    }
}

fn cast(expr: TypedExpr, target: Type, span: Span) -> TypedExpr {
    TypedExpr::new(
        ExprKind::Cast {
            expr: Box::new(expr),
            target,
        },
        target,
        span,
    )
}
