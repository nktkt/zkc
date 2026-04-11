use std::collections::HashSet;

use crate::error::{CompileError, CompileResult};
use crate::ir::{CircuitIr, Operand};
use crate::span::Span;

pub fn verify(ir: &CircuitIr) -> CompileResult<()> {
    let mut defined = HashSet::new();
    let mut next_expected_wire = 0usize;

    for input in ir.public_inputs.iter().chain(ir.private_inputs.iter()) {
        if input.wire >= ir.next_wire {
            return Err(CompileError::new(
                Span::default(),
                format!(
                    "input `{}` references out-of-range wire w{}",
                    input.name, input.wire
                ),
            ));
        }
        if input.wire != next_expected_wire {
            return Err(CompileError::new(
                Span::default(),
                format!(
                    "wire numbering gap or reorder detected: expected w{} but found w{}",
                    next_expected_wire, input.wire
                ),
            ));
        }
        if !defined.insert(input.wire) {
            return Err(CompileError::new(
                Span::default(),
                format!("duplicate wire assignment for input `{}`", input.name),
            ));
        }
        next_expected_wire += 1;
    }

    for operation in &ir.operations {
        ensure_operand_defined(operation.kind.lhs(), &defined)?;
        ensure_operand_defined(operation.kind.rhs(), &defined)?;

        if operation.out >= ir.next_wire {
            return Err(CompileError::new(
                Span::default(),
                format!("operation output w{} is out of range", operation.out),
            ));
        }
        if operation.out != next_expected_wire {
            return Err(CompileError::new(
                Span::default(),
                format!(
                    "wire numbering gap or reorder detected: expected w{} but found w{}",
                    next_expected_wire, operation.out
                ),
            ));
        }
        if !defined.insert(operation.out) {
            return Err(CompileError::new(
                Span::default(),
                format!("duplicate operation output wire w{}", operation.out),
            ));
        }
        next_expected_wire += 1;
    }

    for constraint in &ir.constraints {
        ensure_operand_defined(constraint.lhs, &defined)?;
        ensure_operand_defined(constraint.rhs, &defined)?;
    }

    for constraint in &ir.range_constraints {
        ensure_operand_defined(constraint.value, &defined)?;
    }

    for output in &ir.outputs {
        ensure_operand_defined(output.value, &defined)?;
    }

    if next_expected_wire != ir.next_wire {
        return Err(CompileError::new(
            Span::default(),
            format!(
                "next_wire mismatch: verifier derived {} but circuit stores {}",
                next_expected_wire, ir.next_wire
            ),
        ));
    }

    Ok(())
}

fn ensure_operand_defined(operand: Operand, defined: &HashSet<usize>) -> CompileResult<()> {
    match operand {
        Operand::Const(_) => Ok(()),
        Operand::Wire(wire) if defined.contains(&wire) => Ok(()),
        Operand::Wire(wire) => Err(CompileError::new(
            Span::default(),
            format!("wire w{} is referenced before definition", wire),
        )),
    }
}
