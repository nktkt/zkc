use std::collections::HashMap;
use std::fmt;

use crate::ast::{BinaryOp, Type, UnaryOp, Visibility};
use crate::error::CompileResult;
use crate::field::FieldElement;
use crate::hir::{self, ExprKind, TypedExpr};

pub type WireId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    Wire(WireId),
    Const(FieldElement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedInput {
    pub binding: hir::BindingId,
    pub name: String,
    pub ty: Type,
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

impl OpKind {
    pub fn lhs(&self) -> Operand {
        match *self {
            Self::Add(lhs, _) | Self::Sub(lhs, _) | Self::Mul(lhs, _) => lhs,
        }
    }

    pub fn rhs(&self) -> Operand {
        match *self {
            Self::Add(_, rhs) | Self::Sub(_, rhs) | Self::Mul(_, rhs) => rhs,
        }
    }

    pub fn with_operands(&self, lhs: Operand, rhs: Operand) -> Self {
        match *self {
            Self::Add(_, _) => Self::Add(lhs, rhs),
            Self::Sub(_, _) => Self::Sub(lhs, rhs),
            Self::Mul(_, _) => Self::Mul(lhs, rhs),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraint {
    pub lhs: Operand,
    pub rhs: Operand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeConstraint {
    pub value: Operand,
    pub ty: Type,
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
    pub range_constraints: Vec<RangeConstraint>,
    pub outputs: Vec<Output>,
    pub next_wire: WireId,
}

pub fn lower(program: &hir::Program) -> CompileResult<CircuitIr> {
    let mut lowerer = Lowerer::new(&program.circuit.name);

    for item in &program.circuit.items {
        match item {
            hir::Item::Input(input) => lowerer.lower_input(input),
            hir::Item::Function(_) => {}
            hir::Item::Statement(hir::Statement::Let(stmt)) => {
                let value = lowerer.lower_expr(&stmt.expr);
                lowerer.bind(stmt.binding.id, value);
                if stmt.binding.ty.is_uint() {
                    lowerer.enforce_range(value, stmt.binding.ty);
                }
            }
            hir::Item::Statement(hir::Statement::Constrain(stmt)) => {
                let lhs = lowerer.lower_expr(&stmt.lhs);
                let rhs = lowerer.lower_expr(&stmt.rhs);
                lowerer.ir.constraints.push(Constraint { lhs, rhs });
            }
            hir::Item::Statement(hir::Statement::Expose(stmt)) => {
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

        writeln!(f, "range_constraints:")?;
        if self.range_constraints.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for constraint in &self.range_constraints {
                writeln!(f, "  {} in {}", constraint.value, constraint.ty.name())?;
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
    symbols: HashMap<hir::BindingId, Operand>,
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
                range_constraints: Vec::new(),
                outputs: Vec::new(),
                next_wire: 0,
            },
            symbols: HashMap::new(),
            next_wire: 0,
            output_counter: 0,
        }
    }

    fn lower_input(&mut self, input: &hir::InputDecl) {
        let wire = self.allocate_wire();
        let operand = Operand::Wire(wire);
        if input.binding.ty == Type::Bool {
            self.enforce_boolean(operand);
        } else if input.binding.ty.is_uint() {
            self.enforce_range(operand, input.binding.ty);
        }
        self.bind(input.binding.id, operand);
        let named_input = NamedInput {
            binding: input.binding.id,
            name: input.binding.name.clone(),
            ty: input.binding.ty,
            wire,
        };
        match input.visibility {
            Visibility::Public => self.ir.public_inputs.push(named_input),
            Visibility::Private => self.ir.private_inputs.push(named_input),
        }
    }

    fn bind(&mut self, binding_id: hir::BindingId, value: Operand) {
        self.symbols.insert(binding_id, value);
    }

    fn lower_expr(&mut self, expr: &TypedExpr) -> Operand {
        match &expr.kind {
            ExprKind::Constant(value) => Operand::Const(FieldElement::from_i128(*value)),
            ExprKind::BoolConstant(value) => Operand::Const(if *value {
                FieldElement::from_i128(1)
            } else {
                FieldElement::zero()
            }),
            ExprKind::Reference(binding) => *self
                .symbols
                .get(&binding.id)
                .expect("typecheck should guarantee all identifiers are declared"),
            ExprKind::Unary { op, expr } => {
                let value = self.lower_expr(expr);
                match op {
                    UnaryOp::Neg => self.sub_operands(Operand::Const(FieldElement::zero()), value),
                }
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let lhs = self.lower_expr(lhs);
                let rhs = self.lower_expr(rhs);
                let value = match op {
                    BinaryOp::Add => self.add_operands(lhs, rhs),
                    BinaryOp::Sub => self.sub_operands(lhs, rhs),
                    BinaryOp::Mul => self.mul_operands(lhs, rhs),
                };
                if expr.ty.is_uint() {
                    self.enforce_range(value, expr.ty);
                }
                value
            }
            ExprKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = self.lower_expr(condition);
                let then_branch = self.lower_expr(then_branch);
                let else_branch = self.lower_expr(else_branch);
                self.select_operands(condition, then_branch, else_branch)
            }
            ExprKind::BoolNot { expr } => {
                let value = self.lower_expr(expr);
                self.sub_operands(Operand::Const(FieldElement::from_i128(1)), value)
            }
            ExprKind::BoolAnd { lhs, rhs } => {
                let lhs = self.lower_expr(lhs);
                let rhs = self.lower_expr(rhs);
                self.mul_operands(lhs, rhs)
            }
            ExprKind::BoolOr { lhs, rhs } => {
                let lhs = self.lower_expr(lhs);
                let rhs = self.lower_expr(rhs);
                let sum = self.add_operands(lhs, rhs);
                let overlap = self.mul_operands(lhs, rhs);
                self.sub_operands(sum, overlap)
            }
            ExprKind::BoolXor { lhs, rhs } => {
                let lhs = self.lower_expr(lhs);
                let rhs = self.lower_expr(rhs);
                let sum = self.add_operands(lhs, rhs);
                let overlap = self.mul_operands(lhs, rhs);
                let doubled = self.add_operands(overlap, overlap);
                self.sub_operands(sum, doubled)
            }
            ExprKind::Cast { expr, target } => {
                let value = self.lower_expr(expr);
                if target.is_uint() {
                    self.enforce_range(value, *target);
                }
                value
            }
        }
    }

    fn add_operands(&mut self, lhs: Operand, rhs: Operand) -> Operand {
        match (lhs, rhs) {
            (Operand::Const(a), Operand::Const(b)) => Operand::Const(a + b),
            (lhs, rhs) => self.emit(OpKind::Add(lhs, rhs)),
        }
    }

    fn sub_operands(&mut self, lhs: Operand, rhs: Operand) -> Operand {
        match (lhs, rhs) {
            (Operand::Const(a), Operand::Const(b)) => Operand::Const(a - b),
            (lhs, rhs) => self.emit(OpKind::Sub(lhs, rhs)),
        }
    }

    fn mul_operands(&mut self, lhs: Operand, rhs: Operand) -> Operand {
        match (lhs, rhs) {
            (Operand::Const(a), Operand::Const(b)) => Operand::Const(a * b),
            (lhs, rhs) => self.emit(OpKind::Mul(lhs, rhs)),
        }
    }

    fn select_operands(
        &mut self,
        condition: Operand,
        then_branch: Operand,
        else_branch: Operand,
    ) -> Operand {
        match condition {
            Operand::Const(value) if value == FieldElement::zero() => else_branch,
            Operand::Const(value) if value == FieldElement::from_i128(1) => then_branch,
            condition => {
                let delta = self.sub_operands(then_branch, else_branch);
                let scaled = self.mul_operands(condition, delta);
                self.add_operands(else_branch, scaled)
            }
        }
    }

    fn enforce_boolean(&mut self, operand: Operand) {
        match operand {
            Operand::Const(value)
                if value == FieldElement::zero() || value == FieldElement::from_i128(1) => {}
            Operand::Const(_) => {
                panic!("typecheck should guarantee boolean constants lower to 0 or 1")
            }
            Operand::Wire(_) => {
                let one = Operand::Const(FieldElement::from_i128(1));
                let shifted = self.sub_operands(operand, one);
                let product = self.mul_operands(operand, shifted);
                self.ir.constraints.push(Constraint {
                    lhs: product,
                    rhs: Operand::Const(FieldElement::zero()),
                });
            }
        }
    }

    fn enforce_range(&mut self, operand: Operand, ty: Type) {
        if !ty.is_uint() {
            return;
        }

        match operand {
            Operand::Const(value) if value.fits_in_bits(ty.uint_bits().unwrap_or(0)) => {}
            Operand::Const(_) | Operand::Wire(_) => {
                let constraint = RangeConstraint { value: operand, ty };
                if !self.ir.range_constraints.contains(&constraint) {
                    self.ir.range_constraints.push(constraint);
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

    fn default_output_label(&mut self, expr: &TypedExpr) -> String {
        if let ExprKind::Reference(binding) = &expr.kind {
            return binding.name.clone();
        }

        let label = format!("out{}", self.output_counter);
        self.output_counter += 1;
        label
    }
}
