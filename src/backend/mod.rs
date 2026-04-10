pub mod interpreter;

use crate::error::RuntimeResult;
use crate::eval::{ExecutionResult, RuntimeInputs};
use crate::ir::CircuitIr;

pub trait Backend {
    fn name(&self) -> &'static str;

    fn execute(
        &self,
        circuit: &CircuitIr,
        inputs: &RuntimeInputs,
    ) -> RuntimeResult<ExecutionResult>;
}
