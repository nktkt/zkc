use crate::backend::Backend;
use crate::error::RuntimeResult;
use crate::eval::{ExecutionResult, RuntimeInputs};
use crate::ir::CircuitIr;
use crate::trace::trace_execution;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InterpreterBackend;

impl Backend for InterpreterBackend {
    fn name(&self) -> &'static str {
        "interpreter"
    }

    fn execute(
        &self,
        circuit: &CircuitIr,
        inputs: &RuntimeInputs,
    ) -> RuntimeResult<ExecutionResult> {
        execute_interpreted(circuit, inputs)
    }
}

pub fn execute_interpreted(
    ir: &CircuitIr,
    inputs: &RuntimeInputs,
) -> RuntimeResult<ExecutionResult> {
    let trace = trace_execution(ir, inputs)?;
    let outputs = trace
        .outputs
        .into_iter()
        .map(|output| (output.name, output.value))
        .collect::<Vec<_>>();
    Ok(ExecutionResult { outputs })
}
