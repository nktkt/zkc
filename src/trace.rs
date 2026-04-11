use std::collections::HashSet;
use std::fmt;

use crate::error::{RuntimeError, RuntimeResult};
use crate::eval::RuntimeInputs;
use crate::field::FieldElement;
use crate::ir::{CircuitIr, Constraint, OpKind, Operand, Output, RangeConstraint};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedValue {
    pub name: String,
    pub value: FieldElement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireValue {
    pub wire: usize,
    pub value: FieldElement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOperand {
    pub operand: Operand,
    pub value: FieldElement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationTrace {
    pub out: usize,
    pub opcode: &'static str,
    pub lhs: ResolvedOperand,
    pub rhs: ResolvedOperand,
    pub value: FieldElement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintTrace {
    pub index: usize,
    pub lhs: ResolvedOperand,
    pub rhs: ResolvedOperand,
    pub satisfied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeConstraintTrace {
    pub index: usize,
    pub value: ResolvedOperand,
    pub ty: crate::ast::Type,
    pub satisfied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputTrace {
    pub name: String,
    pub source: Operand,
    pub value: FieldElement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessTrace {
    pub circuit: String,
    pub backend: &'static str,
    pub field_modulus: &'static str,
    pub public_inputs: Vec<NamedValue>,
    pub private_inputs: Vec<NamedValue>,
    pub wires: Vec<WireValue>,
    pub operations: Vec<OperationTrace>,
    pub constraints: Vec<ConstraintTrace>,
    pub range_constraints: Vec<RangeConstraintTrace>,
    pub outputs: Vec<OutputTrace>,
}

pub fn trace_execution(ir: &CircuitIr, inputs: &RuntimeInputs) -> RuntimeResult<WitnessTrace> {
    validate_input_names(ir, inputs)?;

    let mut wires = vec![FieldElement::zero(); ir.next_wire];
    let public_inputs = assign_inputs(&ir.public_inputs, &inputs.public, &mut wires, "public")?;
    let private_inputs = assign_inputs(&ir.private_inputs, &inputs.private, &mut wires, "private")?;

    let mut operations = Vec::with_capacity(ir.operations.len());
    for operation in &ir.operations {
        let lhs = resolve_operand(operation.kind.lhs(), &wires)?;
        let rhs = resolve_operand(operation.kind.rhs(), &wires)?;
        let value = apply_operation(&operation.kind, lhs.value, rhs.value);
        wires[operation.out] = value;

        operations.push(OperationTrace {
            out: operation.out,
            opcode: opcode_name(&operation.kind),
            lhs,
            rhs,
            value,
        });
    }

    let constraints = trace_constraints(&ir.constraints, &wires)?;
    for constraint in &constraints {
        if !constraint.satisfied {
            return Err(RuntimeError::new(format!(
                "constraint failed at #{}: {} != {}",
                constraint.index, constraint.lhs.value, constraint.rhs.value
            )));
        }
    }

    let range_constraints = trace_range_constraints(&ir.range_constraints, &wires)?;
    for constraint in &range_constraints {
        if !constraint.satisfied {
            return Err(RuntimeError::new(format!(
                "range check failed at #{}: {} is not in {}",
                constraint.index,
                constraint.value.value,
                constraint.ty.name()
            )));
        }
    }

    let outputs = trace_outputs(&ir.outputs, &wires)?;
    let wires = wires
        .into_iter()
        .enumerate()
        .map(|(wire, value)| WireValue { wire, value })
        .collect::<Vec<_>>();

    Ok(WitnessTrace {
        circuit: ir.name.clone(),
        backend: "interpreter",
        field_modulus: crate::field::MODULUS,
        public_inputs,
        private_inputs,
        wires,
        operations,
        constraints,
        range_constraints,
        outputs,
    })
}

impl WitnessTrace {
    pub fn to_json(&self) -> String {
        let mut out = String::from("{");
        push_field(&mut out, "circuit", &json_string(&self.circuit));
        out.push(',');
        push_field(&mut out, "backend", &json_string(self.backend));
        out.push(',');
        push_field(&mut out, "field_modulus", &json_string(self.field_modulus));
        out.push(',');
        push_field(
            &mut out,
            "public_inputs",
            &json_array(&self.public_inputs, named_value_json),
        );
        out.push(',');
        push_field(
            &mut out,
            "private_inputs",
            &json_array(&self.private_inputs, named_value_json),
        );
        out.push(',');
        push_field(&mut out, "wires", &json_array(&self.wires, wire_value_json));
        out.push(',');
        push_field(
            &mut out,
            "operations",
            &json_array(&self.operations, operation_trace_json),
        );
        out.push(',');
        push_field(
            &mut out,
            "constraints",
            &json_array(&self.constraints, constraint_trace_json),
        );
        out.push(',');
        push_field(
            &mut out,
            "range_constraints",
            &json_array(&self.range_constraints, range_constraint_trace_json),
        );
        out.push(',');
        push_field(
            &mut out,
            "outputs",
            &json_array(&self.outputs, output_trace_json),
        );
        out.push('}');
        out
    }
}

impl fmt::Display for WitnessTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "execution trace for {} via {} over field modulus {}",
            self.circuit, self.backend, self.field_modulus
        )?;
        writeln!(f, "public inputs:")?;
        format_named_values(f, &self.public_inputs)?;
        writeln!(f, "private inputs:")?;
        format_named_values(f, &self.private_inputs)?;

        writeln!(f, "operations:")?;
        if self.operations.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for operation in &self.operations {
                writeln!(
                    f,
                    "  w{} = {} {} ({}) , {} ({}) => {}",
                    operation.out,
                    operation.opcode,
                    operation.lhs.operand,
                    operation.lhs.value,
                    operation.rhs.operand,
                    operation.rhs.value,
                    operation.value
                )?;
            }
        }

        writeln!(f, "constraints:")?;
        if self.constraints.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for constraint in &self.constraints {
                let status = if constraint.satisfied { "ok" } else { "failed" };
                writeln!(
                    f,
                    "  #{}: {} ({}) == {} ({}) [{}]",
                    constraint.index,
                    constraint.lhs.operand,
                    constraint.lhs.value,
                    constraint.rhs.operand,
                    constraint.rhs.value,
                    status
                )?;
            }
        }

        writeln!(f, "range constraints:")?;
        if self.range_constraints.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for constraint in &self.range_constraints {
                let status = if constraint.satisfied { "ok" } else { "failed" };
                writeln!(
                    f,
                    "  #{}: {} ({}) in {} [{}]",
                    constraint.index,
                    constraint.value.operand,
                    constraint.value.value,
                    constraint.ty.name(),
                    status
                )?;
            }
        }

        writeln!(f, "outputs:")?;
        if self.outputs.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for output in &self.outputs {
                writeln!(
                    f,
                    "  {} = {} ({})",
                    output.name, output.source, output.value
                )?;
            }
        }

        writeln!(f, "wires:")?;
        if self.wires.is_empty() {
            writeln!(f, "  <none>")?;
        } else {
            for wire in &self.wires {
                writeln!(f, "  w{} = {}", wire.wire, wire.value)?;
            }
        }

        Ok(())
    }
}

fn validate_input_names(ir: &CircuitIr, inputs: &RuntimeInputs) -> RuntimeResult<()> {
    let mut seen_names = HashSet::new();
    for input in &ir.public_inputs {
        seen_names.insert(input.name.clone());
    }
    for input in &ir.private_inputs {
        seen_names.insert(input.name.clone());
    }

    for provided in inputs.public.keys().chain(inputs.private.keys()) {
        if !seen_names.contains(provided) {
            return Err(RuntimeError::new(format!(
                "unexpected input assignment `{provided}`"
            )));
        }
    }

    Ok(())
}

fn assign_inputs(
    inputs: &[crate::ir::NamedInput],
    provided: &std::collections::BTreeMap<String, FieldElement>,
    wires: &mut [FieldElement],
    visibility: &str,
) -> RuntimeResult<Vec<NamedValue>> {
    let mut assigned = Vec::with_capacity(inputs.len());
    for input in inputs {
        let value = provided.get(&input.name).ok_or_else(|| {
            RuntimeError::new(format!("missing {visibility} input `{}`", input.name))
        })?;
        wires[input.wire] = *value;
        assigned.push(NamedValue {
            name: input.name.clone(),
            value: *value,
        });
    }
    Ok(assigned)
}

fn trace_constraints(
    constraints: &[Constraint],
    wires: &[FieldElement],
) -> RuntimeResult<Vec<ConstraintTrace>> {
    let mut traced = Vec::with_capacity(constraints.len());
    for (index, constraint) in constraints.iter().enumerate() {
        let lhs = resolve_operand(constraint.lhs, wires)?;
        let rhs = resolve_operand(constraint.rhs, wires)?;
        traced.push(ConstraintTrace {
            index,
            satisfied: lhs.value == rhs.value,
            lhs,
            rhs,
        });
    }
    Ok(traced)
}

fn trace_outputs(outputs: &[Output], wires: &[FieldElement]) -> RuntimeResult<Vec<OutputTrace>> {
    let mut traced = Vec::with_capacity(outputs.len());
    for output in outputs {
        let resolved = resolve_operand(output.value, wires)?;
        traced.push(OutputTrace {
            name: output.name.clone(),
            source: output.value,
            value: resolved.value,
        });
    }
    Ok(traced)
}

fn trace_range_constraints(
    constraints: &[RangeConstraint],
    wires: &[FieldElement],
) -> RuntimeResult<Vec<RangeConstraintTrace>> {
    let mut traced = Vec::with_capacity(constraints.len());
    for (index, constraint) in constraints.iter().enumerate() {
        let value = resolve_operand(constraint.value, wires)?;
        traced.push(RangeConstraintTrace {
            index,
            satisfied: value
                .value
                .fits_in_bits(constraint.ty.uint_bits().unwrap_or(0)),
            value,
            ty: constraint.ty,
        });
    }
    Ok(traced)
}

fn resolve_operand(operand: Operand, wires: &[FieldElement]) -> RuntimeResult<ResolvedOperand> {
    Ok(ResolvedOperand {
        operand,
        value: match operand {
            Operand::Const(value) => value,
            Operand::Wire(id) => wires
                .get(id)
                .copied()
                .ok_or_else(|| RuntimeError::new(format!("missing value for wire w{id}")))?,
        },
    })
}

fn apply_operation(kind: &OpKind, lhs: FieldElement, rhs: FieldElement) -> FieldElement {
    match kind {
        OpKind::Add(_, _) => lhs + rhs,
        OpKind::Sub(_, _) => lhs - rhs,
        OpKind::Mul(_, _) => lhs * rhs,
    }
}

fn opcode_name(kind: &OpKind) -> &'static str {
    match kind {
        OpKind::Add(_, _) => "add",
        OpKind::Sub(_, _) => "sub",
        OpKind::Mul(_, _) => "mul",
    }
}

fn format_named_values(f: &mut fmt::Formatter<'_>, values: &[NamedValue]) -> fmt::Result {
    if values.is_empty() {
        writeln!(f, "  <none>")
    } else {
        for value in values {
            writeln!(f, "  {} = {}", value.name, value.value)?;
        }
        Ok(())
    }
}

fn named_value_json(value: &NamedValue) -> String {
    format!(
        "{{\"name\":{},\"value\":{}}}",
        json_string(&value.name),
        json_string(&value.value.to_string())
    )
}

fn wire_value_json(value: &WireValue) -> String {
    format!(
        "{{\"wire\":{},\"value\":{}}}",
        value.wire,
        json_string(&value.value.to_string())
    )
}

fn resolved_operand_json(value: &ResolvedOperand) -> String {
    format!(
        "{{\"operand\":{},\"value\":{}}}",
        operand_json(value.operand),
        json_string(&value.value.to_string())
    )
}

fn operation_trace_json(operation: &OperationTrace) -> String {
    format!(
        concat!(
            "{{",
            "\"out\":{},",
            "\"op\":{},",
            "\"lhs\":{},",
            "\"rhs\":{},",
            "\"value\":{}",
            "}}"
        ),
        operation.out,
        json_string(operation.opcode),
        resolved_operand_json(&operation.lhs),
        resolved_operand_json(&operation.rhs),
        json_string(&operation.value.to_string())
    )
}

fn constraint_trace_json(constraint: &ConstraintTrace) -> String {
    format!(
        concat!(
            "{{",
            "\"index\":{},",
            "\"lhs\":{},",
            "\"rhs\":{},",
            "\"satisfied\":{}",
            "}}"
        ),
        constraint.index,
        resolved_operand_json(&constraint.lhs),
        resolved_operand_json(&constraint.rhs),
        constraint.satisfied
    )
}

fn range_constraint_trace_json(constraint: &RangeConstraintTrace) -> String {
    format!(
        concat!(
            "{{",
            "\"index\":{},",
            "\"value\":{},",
            "\"type\":{},",
            "\"satisfied\":{}",
            "}}"
        ),
        constraint.index,
        resolved_operand_json(&constraint.value),
        json_string(constraint.ty.name()),
        constraint.satisfied
    )
}

fn output_trace_json(output: &OutputTrace) -> String {
    format!(
        concat!("{{", "\"name\":{},", "\"source\":{},", "\"value\":{}", "}}"),
        json_string(&output.name),
        operand_json(output.source),
        json_string(&output.value.to_string())
    )
}

fn operand_json(operand: Operand) -> String {
    match operand {
        Operand::Wire(wire) => format!("{{\"kind\":\"wire\",\"wire\":{wire}}}"),
        Operand::Const(value) => format!(
            "{{\"kind\":\"const\",\"value\":{}}}",
            json_string(&value.to_string())
        ),
    }
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
