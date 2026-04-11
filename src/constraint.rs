use std::fmt;

use crate::ast::Type;
use crate::field::FieldElement;
use crate::ir::{CircuitIr, Constraint, OpKind, Operand, Operation, Output};

pub type VariableId = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintIr {
    pub name: String,
    pub public_inputs: Vec<Variable>,
    pub private_inputs: Vec<Variable>,
    pub witnesses: Vec<Variable>,
    pub equations: Vec<Equation>,
    pub range_assertions: Vec<RangeAssertion>,
    pub outputs: Vec<ConstraintOutput>,
    pub next_variable: VariableId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub id: VariableId,
    pub name: String,
    pub role: VariableRole,
    pub ty: Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableRole {
    PublicInput,
    PrivateInput,
    Witness,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Equation {
    pub kind: EquationKind,
    pub lhs: Expr,
    pub rhs: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquationKind {
    Definition,
    Assertion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintOutput {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeAssertion {
    pub value: Expr,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Var(VariableId),
    Const(FieldElement),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}

pub fn lower(ir: &CircuitIr) -> ConstraintIr {
    let mut public_inputs = Vec::new();
    let mut private_inputs = Vec::new();
    let mut witnesses = Vec::new();

    for input in &ir.public_inputs {
        public_inputs.push(Variable {
            id: input.wire,
            name: input.name.clone(),
            role: VariableRole::PublicInput,
            ty: input.ty,
        });
    }

    for input in &ir.private_inputs {
        private_inputs.push(Variable {
            id: input.wire,
            name: input.name.clone(),
            role: VariableRole::PrivateInput,
            ty: input.ty,
        });
    }

    let mut equations = Vec::new();
    for operation in &ir.operations {
        witnesses.push(witness_variable(operation));
        equations.push(operation_equation(operation));
    }

    for constraint in &ir.constraints {
        equations.push(assertion_equation(constraint));
    }

    let outputs = ir.outputs.iter().map(output_value).collect::<Vec<_>>();
    let range_assertions = ir
        .range_constraints
        .iter()
        .map(|constraint| RangeAssertion {
            value: expr_from_operand(constraint.value),
            ty: constraint.ty,
        })
        .collect::<Vec<_>>();

    ConstraintIr {
        name: ir.name.clone(),
        public_inputs,
        private_inputs,
        witnesses,
        equations,
        range_assertions,
        outputs,
        next_variable: ir.next_wire,
    }
}

impl ConstraintIr {
    pub fn to_json(&self) -> String {
        let mut out = String::from("{");
        push_field(&mut out, "name", &json_string(&self.name));
        out.push(',');
        push_field(
            &mut out,
            "field_modulus",
            &crate::field::MODULUS.to_string(),
        );
        out.push(',');
        push_field(
            &mut out,
            "public_inputs",
            &json_array(&self.public_inputs, variable_json),
        );
        out.push(',');
        push_field(
            &mut out,
            "private_inputs",
            &json_array(&self.private_inputs, variable_json),
        );
        out.push(',');
        push_field(
            &mut out,
            "witnesses",
            &json_array(&self.witnesses, variable_json),
        );
        out.push(',');
        push_field(
            &mut out,
            "equations",
            &json_array(&self.equations, equation_json),
        );
        out.push(',');
        push_field(
            &mut out,
            "range_assertions",
            &json_array(&self.range_assertions, range_assertion_json),
        );
        out.push(',');
        push_field(&mut out, "outputs", &json_array(&self.outputs, output_json));
        out.push(',');
        push_field(&mut out, "next_variable", &self.next_variable.to_string());
        out.push('}');
        out
    }
}

impl fmt::Display for ConstraintIr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "constraint system {}", self.name)?;
        writeln!(f, "field modulus {}", crate::field::MODULUS)?;
        writeln!(f, "public_inputs:")?;
        render_variables(f, &self.public_inputs)?;
        writeln!(f, "private_inputs:")?;
        render_variables(f, &self.private_inputs)?;
        writeln!(f, "witnesses:")?;
        render_variables(f, &self.witnesses)?;
        writeln!(f, "equations:")?;
        if self.equations.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for equation in &self.equations {
                writeln!(f, "  {}", equation)?;
            }
        }
        writeln!(f, "range assertions:")?;
        if self.range_assertions.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for assertion in &self.range_assertions {
                writeln!(f, "  {} in {}", assertion.value, assertion.ty.name())?;
            }
        }
        writeln!(f, "outputs:")?;
        if self.outputs.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for output in &self.outputs {
                writeln!(f, "  {} = {}", output.name, output.value)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for Equation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} == {}", self.kind.label(), self.lhs, self.rhs)
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        render_expr(self, 0, f)
    }
}

impl EquationKind {
    fn label(self) -> &'static str {
        match self {
            Self::Definition => "def",
            Self::Assertion => "assert",
        }
    }

    fn json_label(self) -> &'static str {
        match self {
            Self::Definition => "definition",
            Self::Assertion => "assertion",
        }
    }
}

fn witness_variable(operation: &Operation) -> Variable {
    Variable {
        id: operation.out,
        name: format!("w{}", operation.out),
        role: VariableRole::Witness,
        ty: Type::Field,
    }
}

fn operation_equation(operation: &Operation) -> Equation {
    Equation {
        kind: EquationKind::Definition,
        lhs: Expr::Var(operation.out),
        rhs: match operation.kind {
            OpKind::Add(lhs, rhs) => Expr::Add(
                Box::new(expr_from_operand(lhs)),
                Box::new(expr_from_operand(rhs)),
            ),
            OpKind::Sub(lhs, rhs) => Expr::Sub(
                Box::new(expr_from_operand(lhs)),
                Box::new(expr_from_operand(rhs)),
            ),
            OpKind::Mul(lhs, rhs) => Expr::Mul(
                Box::new(expr_from_operand(lhs)),
                Box::new(expr_from_operand(rhs)),
            ),
        },
    }
}

fn assertion_equation(constraint: &Constraint) -> Equation {
    Equation {
        kind: EquationKind::Assertion,
        lhs: expr_from_operand(constraint.lhs),
        rhs: expr_from_operand(constraint.rhs),
    }
}

fn output_value(output: &Output) -> ConstraintOutput {
    ConstraintOutput {
        name: output.name.clone(),
        value: expr_from_operand(output.value),
    }
}

fn expr_from_operand(operand: Operand) -> Expr {
    match operand {
        Operand::Wire(wire) => Expr::Var(wire),
        Operand::Const(value) => Expr::Const(value),
    }
}

fn render_variables(f: &mut fmt::Formatter<'_>, variables: &[Variable]) -> fmt::Result {
    if variables.is_empty() {
        writeln!(f, "  <none>")
    } else {
        for variable in variables {
            writeln!(
                f,
                "  {}: {} -> v{}",
                variable.name,
                variable.ty.name(),
                variable.id
            )?;
        }
        Ok(())
    }
}

fn render_expr(expr: &Expr, parent_precedence: u8, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match expr {
        Expr::Var(id) => write!(f, "v{id}"),
        Expr::Const(value) => write!(f, "{value}"),
        Expr::Add(lhs, rhs) => render_binary(lhs, rhs, "+", 1, parent_precedence, f),
        Expr::Sub(lhs, rhs) => render_binary(lhs, rhs, "-", 1, parent_precedence, f),
        Expr::Mul(lhs, rhs) => render_binary(lhs, rhs, "*", 2, parent_precedence, f),
    }
}

fn render_binary(
    lhs: &Expr,
    rhs: &Expr,
    op: &str,
    precedence: u8,
    parent_precedence: u8,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    let needs_parens = precedence < parent_precedence;
    if needs_parens {
        write!(f, "(")?;
    }
    render_expr(lhs, precedence, f)?;
    write!(f, " {op} ")?;
    render_expr(rhs, precedence + 1, f)?;
    if needs_parens {
        write!(f, ")")?;
    }
    Ok(())
}

fn variable_json(variable: &Variable) -> String {
    format!(
        concat!(
            "{{",
            "\"id\":{},",
            "\"name\":{},",
            "\"role\":{},",
            "\"type\":{}",
            "}}"
        ),
        variable.id,
        json_string(&variable.name),
        json_string(match variable.role {
            VariableRole::PublicInput => "public",
            VariableRole::PrivateInput => "private",
            VariableRole::Witness => "witness",
        }),
        json_string(variable.ty.name()),
    )
}

fn equation_json(equation: &Equation) -> String {
    format!(
        concat!("{{", "\"kind\":{},", "\"lhs\":{},", "\"rhs\":{}", "}}"),
        json_string(equation.kind.json_label()),
        expr_json(&equation.lhs),
        expr_json(&equation.rhs),
    )
}

fn output_json(output: &ConstraintOutput) -> String {
    format!(
        "{{\"name\":{},\"value\":{}}}",
        json_string(&output.name),
        expr_json(&output.value)
    )
}

fn range_assertion_json(assertion: &RangeAssertion) -> String {
    format!(
        "{{\"value\":{},\"type\":{}}}",
        expr_json(&assertion.value),
        json_string(assertion.ty.name())
    )
}

fn expr_json(expr: &Expr) -> String {
    match expr {
        Expr::Var(id) => format!("{{\"kind\":\"var\",\"id\":{id}}}"),
        Expr::Const(value) => format!(
            "{{\"kind\":\"const\",\"value\":{}}}",
            json_string(&value.to_string())
        ),
        Expr::Add(lhs, rhs) => binary_expr_json("add", lhs, rhs),
        Expr::Sub(lhs, rhs) => binary_expr_json("sub", lhs, rhs),
        Expr::Mul(lhs, rhs) => binary_expr_json("mul", lhs, rhs),
    }
}

fn binary_expr_json(op: &str, lhs: &Expr, rhs: &Expr) -> String {
    format!(
        concat!("{{", "\"kind\":\"{}\",", "\"lhs\":{},", "\"rhs\":{}", "}}"),
        op,
        expr_json(lhs),
        expr_json(rhs)
    )
}

fn push_field(out: &mut String, key: &str, value: &str) {
    out.push_str(&json_string(key));
    out.push(':');
    out.push_str(value);
}

fn json_array<T>(items: &[T], encode: fn(&T) -> String) -> String {
    let mut out = String::from("[");
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&encode(item));
    }
    out.push(']');
    out
}

fn json_string(input: &str) -> String {
    let mut out = String::from("\"");
    for ch in input.chars() {
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
    out
}
