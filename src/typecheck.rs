use std::collections::HashMap;

use crate::ast::{Expr, FunctionDecl, Item, Param, Program, Statement, Type, format_callee};
use crate::builtins;
use crate::error::{CompileError, CompileResult};
use crate::hir;

pub fn validate(program: &Program) -> CompileResult<()> {
    let _ = typecheck(program)?;
    Ok(())
}

pub fn typecheck(program: &Program) -> CompileResult<hir::Program> {
    let mut builder = Builder::default();
    let mut items = Vec::new();

    for item in &program.circuit.items {
        match item {
            Item::Include(include) => {
                return Err(CompileError::new(
                    include.span,
                    "include directives must be resolved before typechecking",
                ));
            }
            Item::Import(import) => {
                return Err(CompileError::new(
                    import.span,
                    "import directives must be resolved before typechecking",
                ));
            }
            Item::Input(input) => items.push(hir::Item::Input(builder.lower_input(input)?)),
            Item::Function(function) => {
                items.push(hir::Item::Function(builder.lower_function(function)?));
            }
            Item::Statement(stmt) => {
                items.push(hir::Item::Statement(builder.lower_statement(stmt)?));
            }
        }
    }

    Ok(hir::Program {
        circuit: hir::Circuit {
            name: program.circuit.name.clone(),
            items,
        },
    })
}

fn ensure_fresh(
    symbols: &HashMap<String, hir::Binding>,
    functions: &HashMap<String, hir::FunctionDecl>,
    name: &str,
    span: crate::span::Span,
) -> CompileResult<()> {
    if symbols.contains_key(name) || functions.contains_key(name) || builtins::contains(name) {
        Err(CompileError::new(
            span,
            format!("duplicate declaration for `{name}`"),
        ))
    } else {
        Ok(())
    }
}

#[derive(Debug, Default)]
struct Builder {
    symbols: HashMap<String, hir::Binding>,
    functions: HashMap<String, hir::FunctionDecl>,
    next_binding_id: hir::BindingId,
}

impl Builder {
    fn lower_input(&mut self, input: &crate::ast::InputDecl) -> CompileResult<hir::InputDecl> {
        ensure_fresh(&self.symbols, &self.functions, &input.name, input.span)?;
        let binding = self.bind(input.name.clone(), input.ty, input.span);
        Ok(hir::InputDecl {
            binding,
            visibility: input.visibility,
            span: input.span,
        })
    }

    fn lower_function(&mut self, function: &FunctionDecl) -> CompileResult<hir::FunctionDecl> {
        ensure_fresh(
            &self.symbols,
            &self.functions,
            &function.name,
            function.span,
        )?;
        let typed_function = self.build_function(function)?;
        self.functions
            .insert(typed_function.name.clone(), typed_function.clone());
        Ok(typed_function)
    }

    fn lower_statement(&mut self, statement: &Statement) -> CompileResult<hir::Statement> {
        match statement {
            Statement::Let(stmt) => {
                let expr = self.lower_expr(&stmt.expr)?;
                ensure_fresh(&self.symbols, &self.functions, &stmt.name, stmt.span)?;
                let binding = self.bind(stmt.name.clone(), expr.ty, stmt.span);
                Ok(hir::Statement::Let(hir::LetStmt {
                    binding,
                    expr,
                    span: stmt.span,
                }))
            }
            Statement::Constrain(stmt) => {
                let lhs = self.lower_expr(&stmt.lhs)?;
                let rhs = self.lower_expr(&stmt.rhs)?;
                let (lhs, rhs) = coerce_comparable_pair(lhs, rhs, stmt.span)?;
                if lhs.ty != rhs.ty {
                    return Err(CompileError::new(
                        stmt.span,
                        format!(
                            "constraint operands must have the same type, got `{}` and `{}`",
                            lhs.ty.name(),
                            rhs.ty.name()
                        ),
                    ));
                }

                Ok(hir::Statement::Constrain(hir::ConstrainStmt {
                    lhs,
                    rhs,
                    span: stmt.span,
                }))
            }
            Statement::Expose(stmt) => Ok(hir::Statement::Expose(hir::ExposeStmt {
                expr: self.lower_expr(&stmt.expr)?,
                label: stmt.label.clone(),
                span: stmt.span,
            })),
        }
    }

    fn lower_expr(&self, expr: &Expr) -> CompileResult<hir::TypedExpr> {
        self.lower_expr_in_scope(expr, &self.symbols)
    }

    fn lower_expr_in_scope(
        &self,
        expr: &Expr,
        scope: &HashMap<String, hir::Binding>,
    ) -> CompileResult<hir::TypedExpr> {
        match expr {
            Expr::Number { value, span } => Ok(hir::TypedExpr::field(
                hir::ExprKind::Constant(*value),
                *span,
            )),
            Expr::Bool { value, span } => Ok(hir::TypedExpr::bool(
                hir::ExprKind::BoolConstant(*value),
                *span,
            )),
            Expr::Ident { name, span } => {
                let binding = scope.get(name).ok_or_else(|| {
                    CompileError::new(*span, format!("undeclared identifier `{name}`"))
                })?;
                Ok(hir::TypedExpr {
                    kind: hir::ExprKind::Reference(binding.clone()),
                    ty: binding.ty,
                    span: *span,
                })
            }
            Expr::Call { callee, args, span } => {
                let callee_name = format_callee(callee);
                let typed_args = args
                    .iter()
                    .map(|arg| self.lower_expr_in_scope(arg, scope))
                    .collect::<CompileResult<Vec<_>>>()?;

                if callee.len() == 1 && builtins::contains(&callee[0]) {
                    return builtins::expand(&callee[0], &typed_args, *span);
                }

                let function = self.functions.get(&callee_name).ok_or_else(|| {
                    CompileError::new(*span, format!("undeclared function `{callee_name}`"))
                })?;
                if typed_args.len() != function.params.len() {
                    return Err(CompileError::new(
                        *span,
                        format!(
                            "function `{}` expects {} arguments but got {}",
                            callee_name,
                            function.params.len(),
                            typed_args.len()
                        ),
                    ));
                }
                let typed_args = function
                    .params
                    .iter()
                    .zip(typed_args.into_iter())
                    .map(|(param, arg)| {
                        let arg_ty = arg.ty;
                        coerce_expr_to_type(arg, param.ty, *span).map_err(|_| {
                            CompileError::new(
                                *span,
                                format!(
                                    "function `{}` expects argument `{}` to have type `{}` but got `{}`",
                                    callee_name,
                                    param.name,
                                    param.ty.name(),
                                    arg_ty.name()
                                ),
                            )
                        })
                    })
                    .collect::<CompileResult<Vec<_>>>()?;
                for (param, arg) in function.params.iter().zip(&typed_args) {
                    if param.ty != arg.ty {
                        return Err(CompileError::new(
                            *span,
                            format!(
                                "function `{}` expects argument `{}` to have type `{}` but got `{}`",
                                callee_name,
                                param.name,
                                param.ty.name(),
                                arg.ty.name()
                            ),
                        ));
                    }
                }

                self.instantiate_function(function, &typed_args, *span)
            }
            Expr::Unary { op, expr, span } => {
                let inner = self.lower_expr_in_scope(expr, scope)?;
                if inner.ty != Type::Field {
                    return Err(CompileError::new(
                        *span,
                        format!("operator `-` expects `field`, got `{}`", inner.ty.name()),
                    ));
                }

                Ok(hir::TypedExpr::field(
                    hir::ExprKind::Unary {
                        op: *op,
                        expr: Box::new(inner),
                    },
                    *span,
                ))
            }
            Expr::Binary { op, lhs, rhs, span } => {
                let lhs = self.lower_expr_in_scope(lhs, scope)?;
                let rhs = self.lower_expr_in_scope(rhs, scope)?;
                let (lhs, rhs) = coerce_arithmetic_pair(lhs, rhs, *op, *span)?;
                let ty = lhs.ty;

                Ok(hir::TypedExpr::new(
                    hir::ExprKind::Binary {
                        op: *op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    },
                    ty,
                    *span,
                ))
            }
            Expr::IfElse {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let condition = self.lower_expr_in_scope(condition, scope)?;
                if condition.ty != Type::Bool {
                    return Err(CompileError::new(
                        *span,
                        format!(
                            "`if` condition must have type `bool`, got `{}`",
                            condition.ty.name()
                        ),
                    ));
                }

                let then_branch = self.lower_expr_in_scope(then_branch, scope)?;
                let else_branch = self.lower_expr_in_scope(else_branch, scope)?;
                let then_ty = then_branch.ty;
                let else_ty = else_branch.ty;
                let (then_branch, else_branch) =
                    match coerce_comparable_pair(then_branch, else_branch, *span) {
                        Ok(pair) => pair,
                        Err(_) => {
                            return Err(CompileError::new(
                                *span,
                                format!(
                                    "`if` branches must have the same type, got `{}` and `{}`",
                                    then_ty.name(),
                                    else_ty.name()
                                ),
                            ));
                        }
                    };
                if then_branch.ty != else_branch.ty {
                    return Err(CompileError::new(
                        *span,
                        format!(
                            "`if` branches must have the same type, got `{}` and `{}`",
                            then_branch.ty.name(),
                            else_branch.ty.name()
                        ),
                    ));
                }

                Ok(hir::TypedExpr {
                    kind: hir::ExprKind::IfElse {
                        condition: Box::new(condition),
                        then_branch: Box::new(then_branch.clone()),
                        else_branch: Box::new(else_branch),
                    },
                    ty: then_branch.ty,
                    span: *span,
                })
            }
        }
    }

    fn bind(&mut self, name: String, ty: Type, span: crate::span::Span) -> hir::Binding {
        let binding = hir::Binding {
            id: self.allocate_binding_id(),
            name: name.clone(),
            ty,
            span,
        };
        self.symbols.insert(name, binding.clone());
        binding
    }

    fn allocate_binding_id(&mut self) -> hir::BindingId {
        let id = self.next_binding_id;
        self.next_binding_id += 1;
        id
    }

    fn build_function(&mut self, function: &FunctionDecl) -> CompileResult<hir::FunctionDecl> {
        let mut local_scope = HashMap::new();
        let params = function
            .params
            .iter()
            .map(|param| self.bind_param(&mut local_scope, param))
            .collect::<CompileResult<Vec<_>>>()?;
        let raw_body = self.lower_expr_in_scope(&function.body, &local_scope)?;
        let body_ty = raw_body.ty;
        let body =
            coerce_expr_to_type(raw_body, function.return_type, function.span).map_err(|_| {
                CompileError::new(
                    function.span,
                    format!(
                        "function `{}` declares return type `{}` but body has type `{}`",
                        function.name,
                        function.return_type.name(),
                        body_ty.name()
                    ),
                )
            })?;

        Ok(hir::FunctionDecl {
            name: function.name.clone(),
            params,
            return_type: function.return_type,
            body,
            span: function.span,
        })
    }

    fn bind_param(
        &mut self,
        local_scope: &mut HashMap<String, hir::Binding>,
        param: &Param,
    ) -> CompileResult<hir::Binding> {
        if local_scope.contains_key(&param.name) {
            return Err(CompileError::new(
                param.span,
                format!("duplicate declaration for `{}`", param.name),
            ));
        }

        let binding = hir::Binding {
            id: self.allocate_binding_id(),
            name: param.name.clone(),
            ty: param.ty,
            span: param.span,
        };
        local_scope.insert(param.name.clone(), binding.clone());
        Ok(binding)
    }

    fn instantiate_function(
        &self,
        function: &hir::FunctionDecl,
        args: &[hir::TypedExpr],
        span: crate::span::Span,
    ) -> CompileResult<hir::TypedExpr> {
        let substitutions = function
            .params
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .map(|(param, arg)| (param.id, arg))
            .collect::<HashMap<_, _>>();

        let body = substitute_expr(&function.body, &substitutions)?;
        Ok(hir::TypedExpr {
            kind: body.kind,
            ty: function.return_type,
            span,
        })
    }
}

fn coerce_arithmetic_pair(
    lhs: hir::TypedExpr,
    rhs: hir::TypedExpr,
    op: crate::ast::BinaryOp,
    span: crate::span::Span,
) -> CompileResult<(hir::TypedExpr, hir::TypedExpr)> {
    let (lhs, rhs) = coerce_comparable_pair(lhs, rhs, span)?;
    if lhs.ty == Type::Field || lhs.ty.is_uint() {
        Ok((lhs, rhs))
    } else {
        Err(CompileError::new(
            span,
            format!(
                "operator `{}` expects `field` operands or matching unsigned integers; convert integer values with `into_field(...)` first when mixing domains, got `{}` and `{}`",
                op.mnemonic(),
                lhs.ty.name(),
                rhs.ty.name()
            ),
        ))
    }
}

fn coerce_comparable_pair(
    lhs: hir::TypedExpr,
    rhs: hir::TypedExpr,
    span: crate::span::Span,
) -> CompileResult<(hir::TypedExpr, hir::TypedExpr)> {
    if lhs.ty == rhs.ty {
        return Ok((lhs, rhs));
    }

    if rhs.ty.is_uint()
        && let Ok(lhs) = coerce_expr_to_type(lhs.clone(), rhs.ty, span)
    {
        return Ok((lhs, rhs));
    }

    if lhs.ty.is_uint()
        && let Ok(rhs) = coerce_expr_to_type(rhs.clone(), lhs.ty, span)
    {
        return Ok((lhs, rhs));
    }

    Err(CompileError::new(
        span,
        format!(
            "type mismatch between `{}` and `{}`",
            lhs.ty.name(),
            rhs.ty.name()
        ),
    ))
}

fn coerce_expr_to_type(
    expr: hir::TypedExpr,
    target: Type,
    span: crate::span::Span,
) -> CompileResult<hir::TypedExpr> {
    if expr.ty == target {
        return Ok(expr);
    }

    if target.is_uint()
        && expr.ty == Type::Field
        && let hir::ExprKind::Constant(value) = expr.kind
    {
        ensure_uint_literal_fits(value, target, span)?;
        return Ok(hir::TypedExpr::new(
            hir::ExprKind::Constant(value),
            target,
            expr.span,
        ));
    }

    Err(CompileError::new(
        span,
        format!("cannot coerce `{}` to `{}`", expr.ty.name(), target.name()),
    ))
}

fn ensure_uint_literal_fits(
    value: i128,
    target: Type,
    span: crate::span::Span,
) -> CompileResult<()> {
    let bits = target.uint_bits().unwrap_or(0);
    if value < 0 {
        return Err(CompileError::new(
            span,
            format!(
                "integer literal `{value}` cannot be represented as `{}`",
                target.name()
            ),
        ));
    }

    let max = (1i128 << bits) - 1;
    if value > max {
        return Err(CompileError::new(
            span,
            format!(
                "integer literal `{value}` cannot be represented as `{}`",
                target.name()
            ),
        ));
    }

    Ok(())
}

fn substitute_expr(
    expr: &hir::TypedExpr,
    substitutions: &HashMap<hir::BindingId, hir::TypedExpr>,
) -> CompileResult<hir::TypedExpr> {
    let kind = match &expr.kind {
        hir::ExprKind::Constant(value) => hir::ExprKind::Constant(*value),
        hir::ExprKind::BoolConstant(value) => hir::ExprKind::BoolConstant(*value),
        hir::ExprKind::Reference(binding) => {
            substitutions
                .get(&binding.id)
                .cloned()
                .unwrap_or_else(|| hir::TypedExpr {
                    kind: hir::ExprKind::Reference(binding.clone()),
                    ty: binding.ty,
                    span: expr.span,
                })
                .kind
        }
        hir::ExprKind::Unary { op, expr: inner } => hir::ExprKind::Unary {
            op: *op,
            expr: Box::new(substitute_expr(inner, substitutions)?),
        },
        hir::ExprKind::Binary { op, lhs, rhs } => hir::ExprKind::Binary {
            op: *op,
            lhs: Box::new(substitute_expr(lhs, substitutions)?),
            rhs: Box::new(substitute_expr(rhs, substitutions)?),
        },
        hir::ExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => hir::ExprKind::IfElse {
            condition: Box::new(substitute_expr(condition, substitutions)?),
            then_branch: Box::new(substitute_expr(then_branch, substitutions)?),
            else_branch: Box::new(substitute_expr(else_branch, substitutions)?),
        },
        hir::ExprKind::BoolNot { expr: inner } => hir::ExprKind::BoolNot {
            expr: Box::new(substitute_expr(inner, substitutions)?),
        },
        hir::ExprKind::BoolAnd { lhs, rhs } => hir::ExprKind::BoolAnd {
            lhs: Box::new(substitute_expr(lhs, substitutions)?),
            rhs: Box::new(substitute_expr(rhs, substitutions)?),
        },
        hir::ExprKind::BoolOr { lhs, rhs } => hir::ExprKind::BoolOr {
            lhs: Box::new(substitute_expr(lhs, substitutions)?),
            rhs: Box::new(substitute_expr(rhs, substitutions)?),
        },
        hir::ExprKind::BoolXor { lhs, rhs } => hir::ExprKind::BoolXor {
            lhs: Box::new(substitute_expr(lhs, substitutions)?),
            rhs: Box::new(substitute_expr(rhs, substitutions)?),
        },
        hir::ExprKind::Cast {
            expr: inner,
            target,
        } => hir::ExprKind::Cast {
            expr: Box::new(substitute_expr(inner, substitutions)?),
            target: *target,
        },
    };

    Ok(hir::TypedExpr {
        kind,
        ty: expr.ty,
        span: expr.span,
    })
}
