use std::collections::BTreeMap;

use crate::backend::Backend;
use crate::backend::interpreter::InterpreterBackend;
use crate::error::RuntimeResult;
use crate::field::FieldElement;
use crate::ir::CircuitIr;

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
    execute_with_backend(&InterpreterBackend, ir, inputs)
}

pub fn execute_with_backend<B: Backend>(
    backend: &B,
    ir: &CircuitIr,
    inputs: &RuntimeInputs,
) -> RuntimeResult<ExecutionResult> {
    backend.execute(ir, inputs)
}
