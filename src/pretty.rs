use crate::ast::{
    BinaryOp, Expr, FunctionDecl, Item, Program, Statement, UnaryOp, Visibility, format_callee,
};

pub fn render_program(program: &Program) -> String {
    let mut out = String::new();
    out.push_str("circuit ");
    out.push_str(&program.circuit.name);
    out.push_str(" {\n");

    for item in &program.circuit.items {
        render_item(item, 1, &mut out);
    }

    out.push('}');
    out.push('\n');
    out
}

fn render_item(item: &Item, indent_level: usize, out: &mut String) {
    match item {
        Item::Include(include) => {
            indent(indent_level, out);
            out.push_str("include ");
            push_string_literal(out, &include.path);
            out.push_str(";\n");
        }
        Item::Import(import) => {
            indent(indent_level, out);
            out.push_str("import ");
            push_string_literal(out, &import.path);
            out.push_str(" as ");
            out.push_str(&import.alias);
            out.push_str(";\n");
        }
        Item::Input(input) => {
            indent(indent_level, out);
            out.push_str(match input.visibility {
                Visibility::Public => "public ",
                Visibility::Private => "private ",
            });
            out.push_str(&input.name);
            out.push_str(": ");
            out.push_str(input.ty.name());
            out.push_str(";\n");
        }
        Item::Function(function) => render_function(function, indent_level, out),
        Item::Statement(statement) => render_statement(statement, indent_level, out),
    }
}

fn render_function(function: &FunctionDecl, indent_level: usize, out: &mut String) {
    indent(indent_level, out);
    out.push_str("fn ");
    out.push_str(&function.name);
    out.push('(');
    for (index, param) in function.params.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&param.name);
        out.push_str(": ");
        out.push_str(param.ty.name());
    }
    out.push_str(") -> ");
    out.push_str(function.return_type.name());
    out.push_str(" {\n");
    indent(indent_level + 1, out);
    render_expr(&function.body, 0, out);
    out.push('\n');
    indent(indent_level, out);
    out.push_str("}\n");
}

fn render_statement(statement: &Statement, indent_level: usize, out: &mut String) {
    indent(indent_level, out);
    match statement {
        Statement::Let(stmt) => {
            out.push_str("let ");
            out.push_str(&stmt.name);
            out.push_str(" = ");
            render_expr(&stmt.expr, 0, out);
            out.push_str(";\n");
        }
        Statement::Constrain(stmt) => {
            out.push_str("constrain ");
            render_expr(&stmt.lhs, 0, out);
            out.push_str(" == ");
            render_expr(&stmt.rhs, 0, out);
            out.push_str(";\n");
        }
        Statement::Expose(stmt) => {
            out.push_str("expose ");
            render_expr(&stmt.expr, 0, out);
            if let Some(label) = &stmt.label {
                out.push_str(" as ");
                out.push_str(label);
            }
            out.push_str(";\n");
        }
    }
}

fn render_expr(expr: &Expr, parent_precedence: u8, out: &mut String) {
    match expr {
        Expr::Number { value, .. } => out.push_str(&value.to_string()),
        Expr::Bool { value, .. } => out.push_str(if *value { "true" } else { "false" }),
        Expr::Ident { name, .. } => out.push_str(name),
        Expr::Call { callee, args, .. } => {
            out.push_str(&format_callee(callee));
            out.push('(');
            for (index, arg) in args.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                render_expr(arg, 0, out);
            }
            out.push(')');
        }
        Expr::Unary { op, expr, .. } => {
            let precedence = unary_precedence(*op);
            let needs_parens = precedence < parent_precedence;
            if needs_parens {
                out.push('(');
            }
            match op {
                UnaryOp::Neg => out.push('-'),
            }
            render_expr(expr, precedence, out);
            if needs_parens {
                out.push(')');
            }
        }
        Expr::Binary { op, lhs, rhs, .. } => {
            let precedence = binary_precedence(*op);
            let needs_parens = precedence < parent_precedence;
            if needs_parens {
                out.push('(');
            }
            render_expr(lhs, precedence, out);
            out.push(' ');
            out.push_str(match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
            });
            out.push(' ');
            render_expr(rhs, precedence + 1, out);
            if needs_parens {
                out.push(')');
            }
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            let needs_parens = parent_precedence > 0;
            if needs_parens {
                out.push('(');
            }
            out.push_str("if ");
            render_expr(condition, 0, out);
            out.push_str(" { ");
            render_expr(then_branch, 0, out);
            out.push_str(" } else { ");
            render_expr(else_branch, 0, out);
            out.push_str(" }");
            if needs_parens {
                out.push(')');
            }
        }
    }
}

fn unary_precedence(_op: UnaryOp) -> u8 {
    3
}

fn binary_precedence(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Mul => 2,
        BinaryOp::Add | BinaryOp::Sub => 1,
    }
}

fn indent(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str("    ");
    }
}

fn push_string_literal(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
}
