use std::collections::HashMap;

use crate::ast::{Expr, Item, Program, Statement, Type};
use crate::error::{CompileError, CompileResult};

pub fn validate(program: &Program) -> CompileResult<()> {
    let mut symbols: HashMap<String, Type> = HashMap::new();

    for item in &program.circuit.items {
        match item {
            Item::Input(input) => {
                ensure_fresh(&symbols, &input.name, input.span)?;
                symbols.insert(input.name.clone(), input.ty);
            }
            Item::Statement(Statement::Let(stmt)) => {
                validate_expr(&stmt.expr, &symbols)?;
                ensure_fresh(&symbols, &stmt.name, stmt.span)?;
                symbols.insert(stmt.name.clone(), Type::Field);
            }
            Item::Statement(Statement::Constrain(stmt)) => {
                validate_expr(&stmt.lhs, &symbols)?;
                validate_expr(&stmt.rhs, &symbols)?;
            }
            Item::Statement(Statement::Expose(stmt)) => {
                validate_expr(&stmt.expr, &symbols)?;
            }
        }
    }

    Ok(())
}

fn validate_expr(expr: &Expr, symbols: &HashMap<String, Type>) -> CompileResult<()> {
    match expr {
        Expr::Number { .. } => Ok(()),
        Expr::Ident { name, span } => {
            if symbols.contains_key(name) {
                Ok(())
            } else {
                Err(CompileError::new(
                    *span,
                    format!("undeclared identifier `{name}`"),
                ))
            }
        }
        Expr::Unary { expr, .. } => validate_expr(expr, symbols),
        Expr::Binary { lhs, rhs, .. } => {
            validate_expr(lhs, symbols)?;
            validate_expr(rhs, symbols)
        }
    }
}

fn ensure_fresh(
    symbols: &HashMap<String, Type>,
    name: &str,
    span: crate::span::Span,
) -> CompileResult<()> {
    if symbols.contains_key(name) {
        Err(CompileError::new(
            span,
            format!("duplicate declaration for `{name}`"),
        ))
    } else {
        Ok(())
    }
}
