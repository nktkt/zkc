use std::fmt;

use crate::ast::Type;
use crate::constraint;
use crate::error::{RuntimeError, RuntimeResult};
use crate::eval::RuntimeInputs;
use crate::field::FieldElement;
use crate::ir::CircuitIr;
use crate::trace::{NamedValue, WireValue, trace_execution};

pub const DEBUG_BACKEND_NAME: &str = "debug-non-zk";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugVerificationKey {
    pub backend: &'static str,
    pub circuit: String,
    pub circuit_digest: String,
    pub constraint_digest: String,
    pub public_inputs: Vec<InputSpec>,
    pub private_inputs: Vec<InputSpec>,
    pub outputs: Vec<String>,
    pub equations: usize,
    pub range_assertions: usize,
    pub wires: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugProof {
    pub backend: String,
    pub circuit: String,
    pub circuit_digest: String,
    pub constraint_digest: String,
    pub trace_digest: String,
    pub public_inputs: Vec<NamedValue>,
    pub private_inputs: Vec<NamedValue>,
    pub outputs: Vec<NamedValue>,
    pub wires: Vec<WireValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugVerificationReport {
    pub backend: String,
    pub circuit: String,
    pub public_inputs: usize,
    pub private_inputs: usize,
    pub outputs: usize,
    pub wires: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSpec {
    pub name: String,
    pub ty: Type,
}

pub fn debug_keygen(ir: &CircuitIr) -> DebugVerificationKey {
    let constraints = constraint::lower(ir);
    DebugVerificationKey {
        backend: DEBUG_BACKEND_NAME,
        circuit: ir.name.clone(),
        circuit_digest: digest_text(&circuit_identity_text(ir)),
        constraint_digest: digest_text(&constraints.to_json()),
        public_inputs: ir
            .public_inputs
            .iter()
            .map(|input| InputSpec {
                name: input.name.clone(),
                ty: input.ty,
            })
            .collect(),
        private_inputs: ir
            .private_inputs
            .iter()
            .map(|input| InputSpec {
                name: input.name.clone(),
                ty: input.ty,
            })
            .collect(),
        outputs: ir
            .outputs
            .iter()
            .map(|output| output.name.clone())
            .collect(),
        equations: constraints.equations.len(),
        range_assertions: constraints.range_assertions.len(),
        wires: ir.next_wire,
    }
}

pub fn debug_prove(ir: &CircuitIr, inputs: &RuntimeInputs) -> RuntimeResult<DebugProof> {
    let key = debug_keygen(ir);
    let trace = trace_execution(ir, inputs)?;
    let outputs = trace
        .outputs
        .iter()
        .map(|output| NamedValue {
            name: output.name.clone(),
            value: output.value,
        })
        .collect::<Vec<_>>();

    Ok(DebugProof {
        backend: DEBUG_BACKEND_NAME.to_string(),
        circuit: ir.name.clone(),
        circuit_digest: key.circuit_digest,
        constraint_digest: key.constraint_digest,
        trace_digest: digest_text(&trace.to_json()),
        public_inputs: trace.public_inputs,
        private_inputs: trace.private_inputs,
        outputs,
        wires: trace.wires,
    })
}

pub fn verify_debug_proof(
    ir: &CircuitIr,
    proof: &DebugProof,
) -> RuntimeResult<DebugVerificationReport> {
    let key = debug_keygen(ir);

    if proof.backend != DEBUG_BACKEND_NAME {
        return Err(RuntimeError::new(format!(
            "unsupported proof backend `{}`",
            proof.backend
        )));
    }
    if proof.circuit != ir.name {
        return Err(RuntimeError::new(format!(
            "proof targets circuit `{}` but expected `{}`",
            proof.circuit, ir.name
        )));
    }
    if proof.circuit_digest != key.circuit_digest {
        return Err(RuntimeError::new(
            "proof circuit digest does not match compiled circuit",
        ));
    }
    if proof.constraint_digest != key.constraint_digest {
        return Err(RuntimeError::new(
            "proof constraint digest does not match compiled circuit",
        ));
    }

    let mut inputs = RuntimeInputs::default();
    for input in &proof.public_inputs {
        inputs.insert_public(&input.name, input.value);
    }
    for input in &proof.private_inputs {
        inputs.insert_private(&input.name, input.value);
    }

    let expected = debug_prove(ir, &inputs)?;
    if proof != &expected {
        return Err(RuntimeError::new(
            "debug proof artifact does not match re-executed circuit trace",
        ));
    }

    Ok(DebugVerificationReport {
        backend: proof.backend.clone(),
        circuit: proof.circuit.clone(),
        public_inputs: proof.public_inputs.len(),
        private_inputs: proof.private_inputs.len(),
        outputs: proof.outputs.len(),
        wires: proof.wires.len(),
    })
}

pub fn parse_debug_proof(input: &str) -> RuntimeResult<DebugProof> {
    let mut lines = input.lines();
    let Some(header) = lines.next() else {
        return Err(RuntimeError::new("proof artifact is empty"));
    };
    if header.trim() != "zkc-debug-proof-v1" {
        return Err(RuntimeError::new("unrecognized proof artifact header"));
    }

    let mut backend = None;
    let mut circuit = None;
    let mut circuit_digest = None;
    let mut constraint_digest = None;
    let mut trace_digest = None;
    let mut public_inputs = Vec::new();
    let mut private_inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut wires = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        let parts = line.split('|').collect::<Vec<_>>();
        match parts.as_slice() {
            ["backend", value] => backend = Some((*value).to_string()),
            ["circuit", value] => circuit = Some((*value).to_string()),
            ["circuit_digest", value] => circuit_digest = Some((*value).to_string()),
            ["constraint_digest", value] => constraint_digest = Some((*value).to_string()),
            ["trace_digest", value] => trace_digest = Some((*value).to_string()),
            ["public", name, value] => public_inputs.push(parse_named_value(name, value)?),
            ["private", name, value] => private_inputs.push(parse_named_value(name, value)?),
            ["output", name, value] => outputs.push(parse_named_value(name, value)?),
            ["wire", wire, value] => wires.push(WireValue {
                wire: wire.parse::<usize>().map_err(|err| {
                    RuntimeError::new(format!("invalid wire index `{wire}`: {err}"))
                })?,
                value: FieldElement::parse(value).map_err(RuntimeError::new)?,
            }),
            _ => {
                return Err(RuntimeError::new(format!(
                    "malformed proof artifact line `{line}`"
                )));
            }
        }
    }

    Ok(DebugProof {
        backend: backend.ok_or_else(|| RuntimeError::new("missing `backend` record"))?,
        circuit: circuit.ok_or_else(|| RuntimeError::new("missing `circuit` record"))?,
        circuit_digest: circuit_digest
            .ok_or_else(|| RuntimeError::new("missing `circuit_digest` record"))?,
        constraint_digest: constraint_digest
            .ok_or_else(|| RuntimeError::new("missing `constraint_digest` record"))?,
        trace_digest: trace_digest
            .ok_or_else(|| RuntimeError::new("missing `trace_digest` record"))?,
        public_inputs,
        private_inputs,
        outputs,
        wires,
    })
}

impl DebugVerificationKey {
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"backend\":{},",
                "\"circuit\":{},",
                "\"circuit_digest\":{},",
                "\"constraint_digest\":{},",
                "\"public_inputs\":{},",
                "\"private_inputs\":{},",
                "\"outputs\":{},",
                "\"equations\":{},",
                "\"range_assertions\":{},",
                "\"wires\":{}",
                "}}"
            ),
            json_string(self.backend),
            json_string(&self.circuit),
            json_string(&self.circuit_digest),
            json_string(&self.constraint_digest),
            json_array(&self.public_inputs, input_spec_json),
            json_array(&self.private_inputs, input_spec_json),
            json_array(&self.outputs, |name| json_string(name)),
            self.equations,
            self.range_assertions,
            self.wires
        )
    }

    pub fn to_text(&self) -> String {
        let mut out = String::from("zkc-debug-key-v1\n");
        out.push_str(&format!("backend|{}\n", self.backend));
        out.push_str(&format!("circuit|{}\n", self.circuit));
        out.push_str(&format!("circuit_digest|{}\n", self.circuit_digest));
        out.push_str(&format!("constraint_digest|{}\n", self.constraint_digest));
        out.push_str(&format!("equations|{}\n", self.equations));
        out.push_str(&format!("range_assertions|{}\n", self.range_assertions));
        out.push_str(&format!("wires|{}\n", self.wires));
        for input in &self.public_inputs {
            out.push_str(&format!(
                "public_input|{}|{}\n",
                input.name,
                input.ty.name()
            ));
        }
        for input in &self.private_inputs {
            out.push_str(&format!(
                "private_input|{}|{}\n",
                input.name,
                input.ty.name()
            ));
        }
        for output in &self.outputs {
            out.push_str(&format!("output|{output}\n"));
        }
        out
    }
}

impl DebugProof {
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"backend\":{},",
                "\"circuit\":{},",
                "\"circuit_digest\":{},",
                "\"constraint_digest\":{},",
                "\"trace_digest\":{},",
                "\"public_inputs\":{},",
                "\"private_inputs\":{},",
                "\"outputs\":{},",
                "\"wires\":{}",
                "}}"
            ),
            json_string(&self.backend),
            json_string(&self.circuit),
            json_string(&self.circuit_digest),
            json_string(&self.constraint_digest),
            json_string(&self.trace_digest),
            json_array(&self.public_inputs, named_value_json),
            json_array(&self.private_inputs, named_value_json),
            json_array(&self.outputs, named_value_json),
            json_array(&self.wires, wire_value_json)
        )
    }

    pub fn to_text(&self) -> String {
        let mut out = String::from("zkc-debug-proof-v1\n");
        out.push_str(&format!("backend|{}\n", self.backend));
        out.push_str(&format!("circuit|{}\n", self.circuit));
        out.push_str(&format!("circuit_digest|{}\n", self.circuit_digest));
        out.push_str(&format!("constraint_digest|{}\n", self.constraint_digest));
        out.push_str(&format!("trace_digest|{}\n", self.trace_digest));
        for input in &self.public_inputs {
            out.push_str(&format!("public|{}|{}\n", input.name, input.value));
        }
        for input in &self.private_inputs {
            out.push_str(&format!("private|{}|{}\n", input.name, input.value));
        }
        for output in &self.outputs {
            out.push_str(&format!("output|{}|{}\n", output.name, output.value));
        }
        for wire in &self.wires {
            out.push_str(&format!("wire|{}|{}\n", wire.wire, wire.value));
        }
        out
    }
}

impl DebugVerificationReport {
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"backend\":{},",
                "\"circuit\":{},",
                "\"public_inputs\":{},",
                "\"private_inputs\":{},",
                "\"outputs\":{},",
                "\"wires\":{}",
                "}}"
            ),
            json_string(&self.backend),
            json_string(&self.circuit),
            self.public_inputs,
            self.private_inputs,
            self.outputs,
            self.wires
        )
    }
}

impl fmt::Display for DebugVerificationKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_text())
    }
}

impl fmt::Display for DebugProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_text())
    }
}

impl fmt::Display for DebugVerificationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "verified debug proof for {} via {}",
            self.circuit, self.backend
        )?;
        writeln!(
            f,
            "inputs: public={} private={}",
            self.public_inputs, self.private_inputs
        )?;
        writeln!(f, "outputs: {}", self.outputs)?;
        writeln!(f, "wires: {}", self.wires)
    }
}

fn parse_named_value(name: &str, value: &str) -> RuntimeResult<NamedValue> {
    Ok(NamedValue {
        name: name.to_string(),
        value: FieldElement::parse(value).map_err(RuntimeError::new)?,
    })
}

fn circuit_identity_text(ir: &CircuitIr) -> String {
    let mut out = String::new();
    out.push_str(&ir.name);
    out.push('|');
    out.push_str(&ir.next_wire.to_string());
    out.push('|');
    out.push_str(&constraint::lower(ir).to_json());
    out
}

fn digest_text(input: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn input_spec_json(input: &InputSpec) -> String {
    format!(
        "{{\"name\":{},\"type\":{}}}",
        json_string(&input.name),
        json_string(input.ty.name())
    )
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
