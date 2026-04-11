use std::fmt;

use crate::ir::{CircuitIr, OpKind, Operand};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitReport {
    pub name: String,
    pub public_inputs: usize,
    pub private_inputs: usize,
    pub outputs: usize,
    pub constraints: usize,
    pub range_constraints: usize,
    pub wires: usize,
    pub operations: OperationCounts,
    pub constant_operands: usize,
    pub wire_operands: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OperationCounts {
    pub add: usize,
    pub sub: usize,
    pub mul: usize,
}

impl OperationCounts {
    pub fn total(self) -> usize {
        self.add + self.sub + self.mul
    }
}

pub fn analyze(ir: &CircuitIr) -> CircuitReport {
    let mut operations = OperationCounts::default();
    let mut constant_operands = 0;
    let mut wire_operands = 0;

    for operation in &ir.operations {
        match operation.kind {
            OpKind::Add(lhs, rhs) => {
                operations.add += 1;
                count_operand(lhs, &mut constant_operands, &mut wire_operands);
                count_operand(rhs, &mut constant_operands, &mut wire_operands);
            }
            OpKind::Sub(lhs, rhs) => {
                operations.sub += 1;
                count_operand(lhs, &mut constant_operands, &mut wire_operands);
                count_operand(rhs, &mut constant_operands, &mut wire_operands);
            }
            OpKind::Mul(lhs, rhs) => {
                operations.mul += 1;
                count_operand(lhs, &mut constant_operands, &mut wire_operands);
                count_operand(rhs, &mut constant_operands, &mut wire_operands);
            }
        }
    }

    for constraint in &ir.constraints {
        count_operand(constraint.lhs, &mut constant_operands, &mut wire_operands);
        count_operand(constraint.rhs, &mut constant_operands, &mut wire_operands);
    }

    for constraint in &ir.range_constraints {
        count_operand(constraint.value, &mut constant_operands, &mut wire_operands);
    }

    for output in &ir.outputs {
        count_operand(output.value, &mut constant_operands, &mut wire_operands);
    }

    CircuitReport {
        name: ir.name.clone(),
        public_inputs: ir.public_inputs.len(),
        private_inputs: ir.private_inputs.len(),
        outputs: ir.outputs.len(),
        constraints: ir.constraints.len(),
        range_constraints: ir.range_constraints.len(),
        wires: ir.next_wire,
        operations,
        constant_operands,
        wire_operands,
    }
}

impl CircuitReport {
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"name\":\"{}\",",
                "\"public_inputs\":{},",
                "\"private_inputs\":{},",
                "\"outputs\":{},",
                "\"constraints\":{},",
                "\"range_constraints\":{},",
                "\"wires\":{},",
                "\"operations\":{{\"add\":{},\"sub\":{},\"mul\":{},\"total\":{}}},",
                "\"operands\":{{\"constant\":{},\"wire\":{}}}",
                "}}"
            ),
            escape_json(&self.name),
            self.public_inputs,
            self.private_inputs,
            self.outputs,
            self.constraints,
            self.range_constraints,
            self.wires,
            self.operations.add,
            self.operations.sub,
            self.operations.mul,
            self.operations.total(),
            self.constant_operands,
            self.wire_operands
        )
    }
}

impl fmt::Display for CircuitReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "circuit analysis for {}", self.name)?;
        writeln!(
            f,
            "inputs: public={} private={}",
            self.public_inputs, self.private_inputs
        )?;
        writeln!(f, "outputs: {}", self.outputs)?;
        writeln!(f, "constraints: {}", self.constraints)?;
        writeln!(f, "range constraints: {}", self.range_constraints)?;
        writeln!(f, "wires: {}", self.wires)?;
        writeln!(
            f,
            "operations: add={} sub={} mul={} total={}",
            self.operations.add,
            self.operations.sub,
            self.operations.mul,
            self.operations.total()
        )?;
        writeln!(
            f,
            "operands: const={} wire={}",
            self.constant_operands, self.wire_operands
        )
    }
}

fn count_operand(operand: Operand, constant_operands: &mut usize, wire_operands: &mut usize) {
    match operand {
        Operand::Const(_) => *constant_operands += 1,
        Operand::Wire(_) => *wire_operands += 1,
    }
}

fn escape_json(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
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
    out
}
