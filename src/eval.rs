use std::collections::{BTreeMap, HashSet};

use crate::error::{RuntimeError, RuntimeResult};
use crate::field::FieldElement;
use crate::ir::{CircuitIr, OpKind, Operand};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeInputs {
    pub public: BTreeMap<String, FieldElement>,
    pub private: BTreeMap<String, FieldElement>,
}

impl RuntimeInputs {
    pub fn insert_public(&mut self, name: impl Into<String>, value: FieldElement) {
        self.public.insert(name.into(), value);
    }

    pub fn insert_private(&mut self, name: impl Into<String>, value: FieldElement) {
        self.private.insert(name.into(), value);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub outputs: Vec<(String, FieldElement)>,
}

pub fn execute(ir: &CircuitIr, inputs: &RuntimeInputs) -> RuntimeResult<ExecutionResult> {
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

    let mut wires = vec![FieldElement::zero(); ir.next_wire];
    for input in &ir.public_inputs {
        let value = inputs
            .public
            .get(&input.name)
            .ok_or_else(|| RuntimeError::new(format!("missing public input `{}`", input.name)))?;
        wires[input.wire] = *value;
    }

    for input in &ir.private_inputs {
        let value = inputs
            .private
            .get(&input.name)
            .ok_or_else(|| RuntimeError::new(format!("missing private input `{}`", input.name)))?;
        wires[input.wire] = *value;
    }

    for op in &ir.operations {
        let value = match op.kind {
            OpKind::Add(lhs, rhs) => resolve(lhs, &wires)?.add(resolve(rhs, &wires)?),
            OpKind::Sub(lhs, rhs) => resolve(lhs, &wires)?.sub(resolve(rhs, &wires)?),
            OpKind::Mul(lhs, rhs) => resolve(lhs, &wires)?.mul(resolve(rhs, &wires)?),
        };
        wires[op.out] = value;
    }

    for constraint in &ir.constraints {
        let lhs = resolve(constraint.lhs, &wires)?;
        let rhs = resolve(constraint.rhs, &wires)?;
        if lhs != rhs {
            return Err(RuntimeError::new(format!(
                "constraint failed: {} != {}",
                lhs, rhs
            )));
        }
    }

    let mut outputs = Vec::with_capacity(ir.outputs.len());
    for output in &ir.outputs {
        outputs.push((output.name.clone(), resolve(output.value, &wires)?));
    }

    Ok(ExecutionResult { outputs })
}

fn resolve(operand: Operand, wires: &[FieldElement]) -> RuntimeResult<FieldElement> {
    match operand {
        Operand::Const(value) => Ok(value),
        Operand::Wire(id) => wires
            .get(id)
            .copied()
            .ok_or_else(|| RuntimeError::new(format!("missing value for wire w{id}"))),
    }
}
