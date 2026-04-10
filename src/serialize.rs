use crate::ir::{CircuitIr, Constraint, NamedInput, OpKind, Operand, Operation, Output};

pub fn ir_to_json(ir: &CircuitIr) -> String {
    let mut out = String::new();
    out.push('{');
    push_field(&mut out, "name", &json_string(&ir.name));
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
        &json_array(&ir.public_inputs, named_input_json),
    );
    out.push(',');
    push_field(
        &mut out,
        "private_inputs",
        &json_array(&ir.private_inputs, named_input_json),
    );
    out.push(',');
    push_field(
        &mut out,
        "operations",
        &json_array(&ir.operations, operation_json),
    );
    out.push(',');
    push_field(
        &mut out,
        "constraints",
        &json_array(&ir.constraints, constraint_json),
    );
    out.push(',');
    push_field(&mut out, "outputs", &json_array(&ir.outputs, output_json));
    out.push(',');
    push_field(&mut out, "next_wire", &ir.next_wire.to_string());
    out.push('}');
    out
}

fn named_input_json(input: &NamedInput) -> String {
    format!(
        concat!(
            "{{",
            "\"binding\":{},",
            "\"name\":{},",
            "\"type\":\"{}\",",
            "\"wire\":{}",
            "}}"
        ),
        input.binding,
        json_string(&input.name),
        match input.ty {
            crate::ast::Type::Field => "field",
            crate::ast::Type::Bool => "bool",
        },
        input.wire
    )
}

fn operation_json(operation: &Operation) -> String {
    let (opcode, lhs, rhs) = match operation.kind {
        OpKind::Add(lhs, rhs) => ("add", lhs, rhs),
        OpKind::Sub(lhs, rhs) => ("sub", lhs, rhs),
        OpKind::Mul(lhs, rhs) => ("mul", lhs, rhs),
    };

    format!(
        concat!(
            "{{",
            "\"out\":{},",
            "\"op\":\"{}\",",
            "\"lhs\":{},",
            "\"rhs\":{}",
            "}}"
        ),
        operation.out,
        opcode,
        operand_json(lhs),
        operand_json(rhs)
    )
}

fn constraint_json(constraint: &Constraint) -> String {
    format!(
        "{{\"lhs\":{},\"rhs\":{}}}",
        operand_json(constraint.lhs),
        operand_json(constraint.rhs)
    )
}

fn output_json(output: &Output) -> String {
    format!(
        "{{\"name\":{},\"value\":{}}}",
        json_string(&output.name),
        operand_json(output.value)
    )
}

fn operand_json(operand: Operand) -> String {
    match operand {
        Operand::Wire(wire) => format!("{{\"kind\":\"wire\",\"wire\":{wire}}}"),
        Operand::Const(value) => {
            format!(
                "{{\"kind\":\"const\",\"value\":{}}}",
                json_string(&value.to_string())
            )
        }
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
