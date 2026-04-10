use std::collections::HashMap;
use std::fmt;

use crate::ast::{BinaryOp, Expr, Item, Program, Statement, UnaryOp, Visibility};
use crate::error::CompileResult;
use crate::field::FieldElement;

pub type WireId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    Wire(WireId),
    Const(FieldElement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedInput {
    pub name: String,
    pub wire: WireId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    pub out: WireId,
    pub kind: OpKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpKind {
    Add(Operand, Operand),
    Sub(Operand, Operand),
    Mul(Operand, Operand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraint {
    pub lhs: Operand,
    pub rhs: Operand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub name: String,
    pub value: Operand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitIr {
    pub name: String,
    pub public_inputs: Vec<NamedInput>,
    pub private_inputs: Vec<NamedInput>,
    pub operations: Vec<Operation>,
    pub constraints: Vec<Constraint>,
    pub outputs: Vec<Output>,
    pub next_wire: WireId,
}

pub fn lower(program: &Program) -> CompileResult<CircuitIr> {
    let mut lowerer = Lowerer::new(&program.circuit.name);

    for item in &program.circuit.items {
        match item {
            Item::Input(input) => lowerer.lower_input(input),
            Item::Statement(Statement::Let(stmt)) => {
                let value = lowerer.lower_expr(&stmt.expr);
                lowerer.bind(stmt.name.clone(), value);
            }
            Item::Statement(Statement::Constrain(stmt)) => {
                let lhs = lowerer.lower_expr(&stmt.lhs);
                let rhs = lowerer.lower_expr(&stmt.rhs);
                lowerer.ir.constraints.push(Constraint { lhs, rhs });
            }
            Item::Statement(Statement::Expose(stmt)) => {
                let value = lowerer.lower_expr(&stmt.expr);
                let name = stmt
                    .label
                    .clone()
                    .unwrap_or_else(|| lowerer.default_output_label(&stmt.expr));
                lowerer.ir.outputs.push(Output { name, value });
            }
        }
    }

    lowerer.ir.next_wire = lowerer.next_wire;
    Ok(lowerer.ir)
}

impl fmt::Display for CircuitIr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "circuit {}", self.name)?;
        writeln!(f, "field modulus {}", crate::field::MODULUS)?;
        writeln!(f, "public_inputs:")?;
        if self.public_inputs.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for input in &self.public_inputs {
                writeln!(f, "  {} -> w{}", input.name, input.wire)?;
            }
        }

        writeln!(f, "private_inputs:")?;
        if self.private_inputs.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for input in &self.private_inputs {
                writeln!(f, "  {} -> w{}", input.name, input.wire)?;
            }
        }

        writeln!(f, "operations:")?;
        if self.operations.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for op in &self.operations {
                writeln!(f, "  {}", op)?;
            }
        }

        writeln!(f, "constraints:")?;
        if self.constraints.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for constraint in &self.constraints {
                writeln!(f, "  {} == {}", constraint.lhs, constraint.rhs)?;
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

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            OpKind::Add(lhs, rhs) => write!(f, "w{} = add {}, {}", self.out, lhs, rhs),
            OpKind::Sub(lhs, rhs) => write!(f, "w{} = sub {}, {}", self.out, lhs, rhs),
            OpKind::Mul(lhs, rhs) => write!(f, "w{} = mul {}, {}", self.out, lhs, rhs),
        }
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wire(id) => write!(f, "w{id}"),
            Self::Const(value) => write!(f, "{value}"),
        }
    }
}

struct Lowerer {
    ir: CircuitIr,
    symbols: HashMap<String, Operand>,
    next_wire: WireId,
    output_counter: usize,
}

impl Lowerer {
    fn new(name: &str) -> Self {
        Self {
            ir: CircuitIr {
                name: name.to_string(),
                public_inputs: Vec::new(),
                private_inputs: Vec::new(),
                operations: Vec::new(),
                constraints: Vec::new(),
                outputs: Vec::new(),
                next_wire: 0,
            },
            symbols: HashMap::new(),
            next_wire: 0,
            output_counter: 0,
        }
    }

    fn lower_input(&mut self, input: &crate::ast::InputDecl) {
        let wire = self.allocate_wire();
        self.bind(input.name.clone(), Operand::Wire(wire));
        let named_input = NamedInput {
            name: input.name.clone(),
            wire,
        };
        match input.visibility {
            Visibility::Public => self.ir.public_inputs.push(named_input),
            Visibility::Private => self.ir.private_inputs.push(named_input),
        }
    }

    fn bind(&mut self, name: String, value: Operand) {
        self.symbols.insert(name, value);
    }

    fn lower_expr(&mut self, expr: &Expr) -> Operand {
        match expr {
            Expr::Number { value, .. } => Operand::Const(FieldElement::from_i128(*value)),
            Expr::Ident { name, .. } => *self
                .symbols
                .get(name)
                .expect("typecheck should guarantee all identifiers are declared"),
            Expr::Unary { op, expr, .. } => match op {
                UnaryOp::Neg => match self.lower_expr(expr) {
                    Operand::Const(value) => Operand::Const(value.neg()),
                    value => self.emit(OpKind::Sub(Operand::Const(FieldElement::zero()), value)),
                },
            },
            Expr::Binary { op, lhs, rhs, .. } => {
                let lhs = self.lower_expr(lhs);
                let rhs = self.lower_expr(rhs);
                match (op, lhs, rhs) {
                    (BinaryOp::Add, Operand::Const(a), Operand::Const(b)) => {
                        Operand::Const(a.add(b))
                    }
                    (BinaryOp::Sub, Operand::Const(a), Operand::Const(b)) => {
                        Operand::Const(a.sub(b))
                    }
                    (BinaryOp::Mul, Operand::Const(a), Operand::Const(b)) => {
                        Operand::Const(a.mul(b))
                    }
                    (BinaryOp::Add, lhs, rhs) => self.emit(OpKind::Add(lhs, rhs)),
                    (BinaryOp::Sub, lhs, rhs) => self.emit(OpKind::Sub(lhs, rhs)),
                    (BinaryOp::Mul, lhs, rhs) => self.emit(OpKind::Mul(lhs, rhs)),
                }
            }
        }
    }

    fn emit(&mut self, kind: OpKind) -> Operand {
        let wire = self.allocate_wire();
        self.ir.operations.push(Operation { out: wire, kind });
        Operand::Wire(wire)
    }

    fn allocate_wire(&mut self) -> WireId {
        let wire = self.next_wire;
        self.next_wire += 1;
        wire
    }

    fn default_output_label(&mut self, expr: &Expr) -> String {
        if let Expr::Ident { name, .. } = expr {
            return name.clone();
        }

        let label = format!("out{}", self.output_counter);
        self.output_counter += 1;
        label
    }
}
