use std::collections::BTreeMap;
use std::fmt;

use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
use ark_relations::{
    lc,
    r1cs::{
        ConstraintSynthesizer, ConstraintSystemRef, LinearCombination, SynthesisError, Variable,
    },
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::{CircuitSpecificSetupSNARK, SNARK};
use rand::thread_rng;

use crate::error::{RuntimeError, RuntimeResult};
use crate::eval::RuntimeInputs;
use crate::field::FieldElement;
use crate::ir::{CircuitIr, Constraint, OpKind, Operand, Operation, RangeConstraint};
use crate::trace::{NamedValue, WitnessTrace, trace_execution};

pub const GROTH16_BACKEND_NAME: &str = "groth16-bn254";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Groth16ProofBundle {
    pub backend: String,
    pub circuit: String,
    pub public_inputs: Vec<NamedValue>,
    pub public_outputs: Vec<NamedValue>,
    pub proof_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Groth16VerificationReport {
    pub backend: String,
    pub circuit: String,
    pub public_inputs: usize,
    pub public_outputs: usize,
}

#[derive(Debug, Clone)]
struct Groth16Assignments {
    public_inputs: BTreeMap<String, FieldElement>,
    private_inputs: BTreeMap<String, FieldElement>,
    wires: Vec<FieldElement>,
    outputs: Vec<(String, FieldElement)>,
}

#[derive(Debug, Clone)]
struct Groth16Circuit {
    ir: CircuitIr,
    assignments: Option<Groth16Assignments>,
}

pub fn setup_groth16(ir: &CircuitIr) -> RuntimeResult<(Vec<u8>, Vec<u8>)> {
    let circuit = Groth16Circuit::blank(ir.clone());
    let mut rng = thread_rng();
    let (pk, vk) = Groth16::<Bn254>::setup(circuit, &mut rng)
        .map_err(|err| RuntimeError::new(format!("groth16 setup failed: {err}")))?;

    let pk_bytes = serialize_compressed(&pk)
        .map_err(|err| RuntimeError::new(format!("failed to serialize proving key: {err}")))?;
    let vk_bytes = serialize_compressed(&vk)
        .map_err(|err| RuntimeError::new(format!("failed to serialize verification key: {err}")))?;
    Ok((pk_bytes, vk_bytes))
}

pub fn prove_groth16(
    ir: &CircuitIr,
    inputs: &RuntimeInputs,
    proving_key_bytes: &[u8],
) -> RuntimeResult<Groth16ProofBundle> {
    let proving_key = ProvingKey::<Bn254>::deserialize_compressed(proving_key_bytes)
        .map_err(|err| RuntimeError::new(format!("failed to deserialize proving key: {err}")))?;
    let trace = trace_execution(ir, inputs)?;
    let circuit =
        Groth16Circuit::with_assignments(ir.clone(), Groth16Assignments::from_trace(&trace));
    let mut rng = thread_rng();
    let proof = Groth16::<Bn254>::prove(&proving_key, circuit, &mut rng)
        .map_err(|err| RuntimeError::new(format!("groth16 proving failed: {err}")))?;

    Ok(Groth16ProofBundle {
        backend: GROTH16_BACKEND_NAME.to_string(),
        circuit: ir.name.clone(),
        public_inputs: trace.public_inputs,
        public_outputs: trace
            .outputs
            .into_iter()
            .map(|output| NamedValue {
                name: output.name,
                value: output.value,
            })
            .collect(),
        proof_hex: bytes_to_hex(
            &serialize_compressed(&proof)
                .map_err(|err| RuntimeError::new(format!("failed to serialize proof: {err}")))?,
        ),
    })
}

pub fn verify_groth16(
    ir: &CircuitIr,
    verification_key_bytes: &[u8],
    bundle: &Groth16ProofBundle,
) -> RuntimeResult<Groth16VerificationReport> {
    if bundle.backend != GROTH16_BACKEND_NAME {
        return Err(RuntimeError::new(format!(
            "unsupported proof backend `{}`",
            bundle.backend
        )));
    }
    if bundle.circuit != ir.name {
        return Err(RuntimeError::new(format!(
            "proof targets circuit `{}` but expected `{}`",
            bundle.circuit, ir.name
        )));
    }

    ensure_public_shape(ir, bundle)?;

    let verification_key = VerifyingKey::<Bn254>::deserialize_compressed(verification_key_bytes)
        .map_err(|err| {
            RuntimeError::new(format!("failed to deserialize verification key: {err}"))
        })?;
    let proof_bytes = hex_to_bytes(&bundle.proof_hex)?;
    let proof = Proof::<Bn254>::deserialize_compressed(&proof_bytes[..])
        .map_err(|err| RuntimeError::new(format!("failed to deserialize proof: {err}")))?;

    let public_inputs = bundle
        .public_inputs
        .iter()
        .chain(bundle.public_outputs.iter())
        .map(|value| value.value.into_backend())
        .collect::<Vec<_>>();

    let verified = Groth16::<Bn254>::verify(&verification_key, &public_inputs, &proof)
        .map_err(|err| RuntimeError::new(format!("groth16 verification failed: {err}")))?;
    if !verified {
        return Err(RuntimeError::new("groth16 proof verification failed"));
    }

    Ok(Groth16VerificationReport {
        backend: bundle.backend.clone(),
        circuit: bundle.circuit.clone(),
        public_inputs: bundle.public_inputs.len(),
        public_outputs: bundle.public_outputs.len(),
    })
}

pub fn parse_groth16_proof_bundle(input: &str) -> RuntimeResult<Groth16ProofBundle> {
    let mut lines = input.lines();
    let Some(header) = lines.next() else {
        return Err(RuntimeError::new("proof artifact is empty"));
    };
    if header.trim() != "zkc-groth16-proof-v1" {
        return Err(RuntimeError::new(
            "unrecognized groth16 proof artifact header",
        ));
    }

    let mut backend = None;
    let mut circuit = None;
    let mut public_inputs = Vec::new();
    let mut public_outputs = Vec::new();
    let mut proof_hex = None;

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        let parts = line.split('|').collect::<Vec<_>>();
        match parts.as_slice() {
            ["backend", value] => backend = Some((*value).to_string()),
            ["circuit", value] => circuit = Some((*value).to_string()),
            ["public", name, value] => public_inputs.push(parse_named_value(name, value)?),
            ["output", name, value] => public_outputs.push(parse_named_value(name, value)?),
            ["proof", value] => proof_hex = Some((*value).to_string()),
            _ => {
                return Err(RuntimeError::new(format!(
                    "malformed groth16 proof artifact line `{line}`"
                )));
            }
        }
    }

    Ok(Groth16ProofBundle {
        backend: backend.ok_or_else(|| RuntimeError::new("missing `backend` record"))?,
        circuit: circuit.ok_or_else(|| RuntimeError::new("missing `circuit` record"))?,
        public_inputs,
        public_outputs,
        proof_hex: proof_hex.ok_or_else(|| RuntimeError::new("missing `proof` record"))?,
    })
}

impl Groth16ProofBundle {
    pub fn to_text(&self) -> String {
        let mut out = String::from("zkc-groth16-proof-v1\n");
        out.push_str(&format!("backend|{}\n", self.backend));
        out.push_str(&format!("circuit|{}\n", self.circuit));
        for input in &self.public_inputs {
            out.push_str(&format!("public|{}|{}\n", input.name, input.value));
        }
        for output in &self.public_outputs {
            out.push_str(&format!("output|{}|{}\n", output.name, output.value));
        }
        out.push_str(&format!("proof|{}\n", self.proof_hex));
        out
    }

    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"backend\":{},",
                "\"circuit\":{},",
                "\"public_inputs\":{},",
                "\"public_outputs\":{},",
                "\"proof_hex\":{}",
                "}}"
            ),
            json_string(&self.backend),
            json_string(&self.circuit),
            json_array(&self.public_inputs, named_value_json),
            json_array(&self.public_outputs, named_value_json),
            json_string(&self.proof_hex)
        )
    }
}

impl Groth16VerificationReport {
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"backend\":{},",
                "\"circuit\":{},",
                "\"public_inputs\":{},",
                "\"public_outputs\":{}",
                "}}"
            ),
            json_string(&self.backend),
            json_string(&self.circuit),
            self.public_inputs,
            self.public_outputs
        )
    }
}

impl fmt::Display for Groth16ProofBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_text())
    }
}

impl fmt::Display for Groth16VerificationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "verified groth16 proof for {} via {}",
            self.circuit, self.backend
        )?;
        writeln!(f, "public inputs: {}", self.public_inputs)?;
        writeln!(f, "public outputs: {}", self.public_outputs)
    }
}

impl Groth16Assignments {
    fn from_trace(trace: &WitnessTrace) -> Self {
        Self {
            public_inputs: trace
                .public_inputs
                .iter()
                .map(|value| (value.name.clone(), value.value))
                .collect(),
            private_inputs: trace
                .private_inputs
                .iter()
                .map(|value| (value.name.clone(), value.value))
                .collect(),
            wires: trace.wires.iter().map(|wire| wire.value).collect(),
            outputs: trace
                .outputs
                .iter()
                .map(|output| (output.name.clone(), output.value))
                .collect(),
        }
    }

    fn public_input_value(&self, name: &str) -> Result<FieldElement, SynthesisError> {
        self.public_inputs
            .get(name)
            .copied()
            .ok_or(SynthesisError::AssignmentMissing)
    }

    fn private_input_value(&self, name: &str) -> Result<FieldElement, SynthesisError> {
        self.private_inputs
            .get(name)
            .copied()
            .ok_or(SynthesisError::AssignmentMissing)
    }

    fn wire_value(&self, wire: usize) -> Result<FieldElement, SynthesisError> {
        self.wires
            .get(wire)
            .copied()
            .ok_or(SynthesisError::AssignmentMissing)
    }

    fn output_value(&self, name: &str) -> Result<FieldElement, SynthesisError> {
        self.outputs
            .iter()
            .find(|(output_name, _)| output_name == name)
            .map(|(_, value)| *value)
            .ok_or(SynthesisError::AssignmentMissing)
    }
}

impl Groth16Circuit {
    fn blank(ir: CircuitIr) -> Self {
        Self {
            ir,
            assignments: None,
        }
    }

    fn with_assignments(ir: CircuitIr, assignments: Groth16Assignments) -> Self {
        Self {
            ir,
            assignments: Some(assignments),
        }
    }

    fn assignment_or_zero(
        &self,
        value: Result<FieldElement, SynthesisError>,
    ) -> Result<Fr, SynthesisError> {
        match (&self.assignments, value) {
            (Some(_), Ok(value)) => Ok(value.into_backend()),
            (Some(_), Err(err)) => Err(err),
            (None, _) => Ok(Fr::from(0u64)),
        }
    }
}

impl ConstraintSynthesizer<Fr> for Groth16Circuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let mut wires = vec![None; self.ir.next_wire];

        for input in &self.ir.public_inputs {
            let value = self.assignment_or_zero(
                self.assignments
                    .as_ref()
                    .map(|assignments| assignments.public_input_value(&input.name))
                    .unwrap_or(Ok(FieldElement::zero())),
            )?;
            let variable = cs.new_input_variable(|| Ok(value))?;
            wires[input.wire] = Some(variable);
        }

        for input in &self.ir.private_inputs {
            let value = self.assignment_or_zero(
                self.assignments
                    .as_ref()
                    .map(|assignments| assignments.private_input_value(&input.name))
                    .unwrap_or(Ok(FieldElement::zero())),
            )?;
            let variable = cs.new_witness_variable(|| Ok(value))?;
            wires[input.wire] = Some(variable);
        }

        for operation in &self.ir.operations {
            let output_value = self.assignment_or_zero(
                self.assignments
                    .as_ref()
                    .map(|assignments| assignments.wire_value(operation.out))
                    .unwrap_or(Ok(FieldElement::zero())),
            )?;
            let output_variable = cs.new_witness_variable(|| Ok(output_value))?;
            wires[operation.out] = Some(output_variable);
            enforce_operation(&cs, operation, output_variable, &wires)?;
        }

        for constraint in &self.ir.constraints {
            enforce_equality(&cs, constraint, &wires)?;
        }

        for range in &self.ir.range_constraints {
            enforce_range(&cs, range, &wires, self.assignments.as_ref())?;
        }

        for output in &self.ir.outputs {
            let output_value = self.assignment_or_zero(
                self.assignments
                    .as_ref()
                    .map(|assignments| assignments.output_value(&output.name))
                    .unwrap_or(Ok(FieldElement::zero())),
            )?;
            let public_output = cs.new_input_variable(|| Ok(output_value))?;
            let value = operand_lc(output.value, &wires)?;
            cs.enforce_constraint(value, lc!() + Variable::One, lc!() + public_output)?;
        }

        Ok(())
    }
}

fn enforce_operation(
    cs: &ConstraintSystemRef<Fr>,
    operation: &Operation,
    output_variable: Variable,
    wires: &[Option<Variable>],
) -> Result<(), SynthesisError> {
    let output = lc!() + output_variable;
    match operation.kind {
        OpKind::Add(lhs, rhs) => {
            let sum = operand_lc(lhs, wires)? + operand_lc(rhs, wires)?;
            cs.enforce_constraint(sum, lc!() + Variable::One, output)?;
        }
        OpKind::Sub(lhs, rhs) => {
            let diff = operand_lc(lhs, wires)? - operand_lc(rhs, wires)?;
            cs.enforce_constraint(diff, lc!() + Variable::One, output)?;
        }
        OpKind::Mul(lhs, rhs) => {
            cs.enforce_constraint(operand_lc(lhs, wires)?, operand_lc(rhs, wires)?, output)?;
        }
    }
    Ok(())
}

fn enforce_equality(
    cs: &ConstraintSystemRef<Fr>,
    constraint: &Constraint,
    wires: &[Option<Variable>],
) -> Result<(), SynthesisError> {
    let diff = operand_lc(constraint.lhs, wires)? - operand_lc(constraint.rhs, wires)?;
    cs.enforce_constraint(diff, lc!() + Variable::One, lc!())?;
    Ok(())
}

fn enforce_range(
    cs: &ConstraintSystemRef<Fr>,
    range: &RangeConstraint,
    wires: &[Option<Variable>],
    assignments: Option<&Groth16Assignments>,
) -> Result<(), SynthesisError> {
    let Some(bits) = range.ty.uint_bits() else {
        return Ok(());
    };

    if let Operand::Const(value) = range.value {
        if value.fits_in_bits(bits) {
            return Ok(());
        }
    }

    let value_lc = operand_lc(range.value, wires)?;
    let assigned_value = match (assignments, range.value) {
        (Some(assignments), Operand::Wire(wire)) => assignments.wire_value(wire)?,
        (_, Operand::Const(value)) => value,
        (None, Operand::Wire(_)) => FieldElement::zero(),
    };
    let assigned_biguint = assigned_value.to_biguint();

    let mut recomposed = lc!();
    for bit_index in 0..usize::from(bits) {
        let bit_is_set = assigned_biguint.bit(bit_index as u64);
        let bit_value = if bit_is_set {
            Fr::from(1u64)
        } else {
            Fr::from(0u64)
        };
        let bit_variable = cs.new_witness_variable(|| Ok(bit_value))?;
        let bit_lc = lc!() + bit_variable;

        cs.enforce_constraint(
            bit_lc.clone(),
            (lc!() + Variable::One) - bit_variable,
            lc!(),
        )?;

        let coefficient = Fr::from(1u64 << bit_index);
        recomposed = recomposed + (coefficient, bit_variable);
    }

    cs.enforce_constraint(recomposed - value_lc, lc!() + Variable::One, lc!())?;
    Ok(())
}

fn operand_lc(
    operand: Operand,
    wires: &[Option<Variable>],
) -> Result<LinearCombination<Fr>, SynthesisError> {
    Ok(match operand {
        Operand::Wire(wire) => {
            let variable = wires
                .get(wire)
                .and_then(|variable| *variable)
                .ok_or(SynthesisError::AssignmentMissing)?;
            lc!() + variable
        }
        Operand::Const(value) if value == FieldElement::zero() => lc!(),
        Operand::Const(value) => lc!() + (value.into_backend(), Variable::One),
    })
}

fn ensure_public_shape(ir: &CircuitIr, bundle: &Groth16ProofBundle) -> RuntimeResult<()> {
    if bundle.public_inputs.len() != ir.public_inputs.len() {
        return Err(RuntimeError::new(format!(
            "proof has {} public inputs but circuit expects {}",
            bundle.public_inputs.len(),
            ir.public_inputs.len()
        )));
    }
    if bundle.public_outputs.len() != ir.outputs.len() {
        return Err(RuntimeError::new(format!(
            "proof has {} public outputs but circuit expects {}",
            bundle.public_outputs.len(),
            ir.outputs.len()
        )));
    }

    for (actual, expected) in bundle.public_inputs.iter().zip(&ir.public_inputs) {
        if actual.name != expected.name {
            return Err(RuntimeError::new(format!(
                "proof public input `{}` does not match circuit input `{}`",
                actual.name, expected.name
            )));
        }
    }
    for (actual, expected) in bundle.public_outputs.iter().zip(&ir.outputs) {
        if actual.name != expected.name {
            return Err(RuntimeError::new(format!(
                "proof public output `{}` does not match circuit output `{}`",
                actual.name, expected.name
            )));
        }
    }

    Ok(())
}

fn parse_named_value(name: &str, value: &str) -> RuntimeResult<NamedValue> {
    Ok(NamedValue {
        name: name.to_string(),
        value: FieldElement::parse(value).map_err(RuntimeError::new)?,
    })
}

fn serialize_compressed<T: CanonicalSerialize>(
    value: &T,
) -> Result<Vec<u8>, ark_serialize::SerializationError> {
    let mut bytes = Vec::new();
    value.serialize_compressed(&mut bytes)?;
    Ok(bytes)
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn hex_to_bytes(input: &str) -> RuntimeResult<Vec<u8>> {
    if input.len() % 2 != 0 {
        return Err(RuntimeError::new("hex payload must have even length"));
    }

    let mut out = Vec::with_capacity(input.len() / 2);
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let high = decode_hex_nibble(bytes[index] as char)?;
        let low = decode_hex_nibble(bytes[index + 1] as char)?;
        out.push((high << 4) | low);
        index += 2;
    }
    Ok(out)
}

fn decode_hex_nibble(ch: char) -> RuntimeResult<u8> {
    match ch {
        '0'..='9' => Ok((ch as u8) - b'0'),
        'a'..='f' => Ok((ch as u8) - b'a' + 10),
        'A'..='F' => Ok((ch as u8) - b'A' + 10),
        _ => Err(RuntimeError::new(format!("invalid hex character `{ch}`"))),
    }
}

fn named_value_json(value: &NamedValue) -> String {
    format!(
        "{{\"name\":{},\"value\":{}}}",
        json_string(&value.name),
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
