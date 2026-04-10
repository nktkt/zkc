use std::collections::{BTreeSet, HashMap};

use crate::field::FieldElement;
use crate::ir::{CircuitIr, Constraint, NamedInput, OpKind, Operand, Operation, Output, WireId};

pub fn optimize(ir: &CircuitIr) -> CircuitIr {
    let mut replacements = HashMap::new();
    let mut simplified_ops = Vec::new();

    for operation in &ir.operations {
        let lhs = rewrite_operand(operation.kind.lhs(), &replacements);
        let rhs = rewrite_operand(operation.kind.rhs(), &replacements);

        match simplify(operation.kind.with_operands(lhs, rhs)) {
            Simplified::Alias(operand) => {
                replacements.insert(operation.out, operand);
            }
            Simplified::Operation(kind) => {
                simplified_ops.push(Operation {
                    out: operation.out,
                    kind,
                });
            }
        }
    }

    let constraints = ir
        .constraints
        .iter()
        .map(|constraint| Constraint {
            lhs: rewrite_operand(constraint.lhs, &replacements),
            rhs: rewrite_operand(constraint.rhs, &replacements),
        })
        .collect::<Vec<_>>();

    let outputs = ir
        .outputs
        .iter()
        .map(|output| Output {
            name: output.name.clone(),
            value: rewrite_operand(output.value, &replacements),
        })
        .collect::<Vec<_>>();

    let live_ops = dead_code_eliminate(&simplified_ops, &constraints, &outputs);
    compact_wires(ir, live_ops, constraints, outputs)
}

fn dead_code_eliminate(
    operations: &[Operation],
    constraints: &[Constraint],
    outputs: &[Output],
) -> Vec<Operation> {
    let mut needed = BTreeSet::new();

    for constraint in constraints {
        mark_operand(constraint.lhs, &mut needed);
        mark_operand(constraint.rhs, &mut needed);
    }
    for output in outputs {
        mark_operand(output.value, &mut needed);
    }

    let mut kept = Vec::new();
    for operation in operations.iter().rev() {
        if needed.contains(&operation.out) {
            mark_operand(operation.kind.lhs(), &mut needed);
            mark_operand(operation.kind.rhs(), &mut needed);
            kept.push(operation.clone());
        }
    }
    kept.reverse();
    kept
}

fn compact_wires(
    original: &CircuitIr,
    operations: Vec<Operation>,
    constraints: Vec<Constraint>,
    outputs: Vec<Output>,
) -> CircuitIr {
    let mut used = BTreeSet::new();

    for input in original
        .public_inputs
        .iter()
        .chain(original.private_inputs.iter())
    {
        used.insert(input.wire);
    }
    for operation in &operations {
        used.insert(operation.out);
        mark_operand(operation.kind.lhs(), &mut used);
        mark_operand(operation.kind.rhs(), &mut used);
    }
    for constraint in &constraints {
        mark_operand(constraint.lhs, &mut used);
        mark_operand(constraint.rhs, &mut used);
    }
    for output in &outputs {
        mark_operand(output.value, &mut used);
    }

    let remap = used
        .iter()
        .copied()
        .enumerate()
        .map(|(new_wire, old_wire)| (old_wire, new_wire))
        .collect::<HashMap<_, _>>();

    let public_inputs = original
        .public_inputs
        .iter()
        .map(|input| remap_input(input, &remap))
        .collect::<Vec<_>>();
    let private_inputs = original
        .private_inputs
        .iter()
        .map(|input| remap_input(input, &remap))
        .collect::<Vec<_>>();
    let operations = operations
        .into_iter()
        .map(|operation| Operation {
            out: remap[&operation.out],
            kind: operation.kind.with_operands(
                remap_operand(operation.kind.lhs(), &remap),
                remap_operand(operation.kind.rhs(), &remap),
            ),
        })
        .collect::<Vec<_>>();
    let constraints = constraints
        .into_iter()
        .map(|constraint| Constraint {
            lhs: remap_operand(constraint.lhs, &remap),
            rhs: remap_operand(constraint.rhs, &remap),
        })
        .collect::<Vec<_>>();
    let outputs = outputs
        .into_iter()
        .map(|output| Output {
            name: output.name,
            value: remap_operand(output.value, &remap),
        })
        .collect::<Vec<_>>();

    CircuitIr {
        name: original.name.clone(),
        public_inputs,
        private_inputs,
        operations,
        constraints,
        outputs,
        next_wire: remap.len(),
    }
}

fn remap_input(input: &NamedInput, remap: &HashMap<WireId, WireId>) -> NamedInput {
    NamedInput {
        binding: input.binding,
        name: input.name.clone(),
        ty: input.ty,
        wire: remap[&input.wire],
    }
}

fn remap_operand(operand: Operand, remap: &HashMap<WireId, WireId>) -> Operand {
    match operand {
        Operand::Const(value) => Operand::Const(value),
        Operand::Wire(wire) => Operand::Wire(remap[&wire]),
    }
}

fn mark_operand(operand: Operand, used: &mut BTreeSet<WireId>) {
    if let Operand::Wire(wire) = operand {
        used.insert(wire);
    }
}

fn rewrite_operand(operand: Operand, replacements: &HashMap<WireId, Operand>) -> Operand {
    let mut current = operand;
    loop {
        match current {
            Operand::Const(_) => return current,
            Operand::Wire(wire) => match replacements.get(&wire).copied() {
                Some(next) => current = next,
                None => return current,
            },
        }
    }
}

enum Simplified {
    Alias(Operand),
    Operation(OpKind),
}

fn simplify(kind: OpKind) -> Simplified {
    match kind {
        OpKind::Add(lhs, rhs) => match (lhs, rhs) {
            (Operand::Const(a), Operand::Const(b)) => Simplified::Alias(Operand::Const(a + b)),
            (Operand::Const(value), other) | (other, Operand::Const(value))
                if value == FieldElement::zero() =>
            {
                Simplified::Alias(other)
            }
            (lhs, rhs) => Simplified::Operation(OpKind::Add(lhs, rhs)),
        },
        OpKind::Sub(lhs, rhs) => match (lhs, rhs) {
            (Operand::Const(a), Operand::Const(b)) => Simplified::Alias(Operand::Const(a - b)),
            (lhs, Operand::Const(value)) if value == FieldElement::zero() => Simplified::Alias(lhs),
            (Operand::Wire(lhs), Operand::Wire(rhs)) if lhs == rhs => {
                Simplified::Alias(Operand::Const(FieldElement::zero()))
            }
            (lhs, rhs) => Simplified::Operation(OpKind::Sub(lhs, rhs)),
        },
        OpKind::Mul(lhs, rhs) => match (lhs, rhs) {
            (Operand::Const(a), Operand::Const(b)) => Simplified::Alias(Operand::Const(a * b)),
            (Operand::Const(value), _) | (_, Operand::Const(value))
                if value == FieldElement::zero() =>
            {
                Simplified::Alias(Operand::Const(FieldElement::zero()))
            }
            (Operand::Const(value), other) | (other, Operand::Const(value))
                if value == FieldElement::from_i128(1) =>
            {
                Simplified::Alias(other)
            }
            (lhs, rhs) => Simplified::Operation(OpKind::Mul(lhs, rhs)),
        },
    }
}
