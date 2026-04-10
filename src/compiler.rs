use crate::ast::Program;
use crate::error::CompileResult;
use crate::ir::{CircuitIr, lower};
use crate::parser;
use crate::typecheck;

pub fn parse_and_validate(source: &str) -> CompileResult<Program> {
    let program = parser::parse(source)?;
    typecheck::validate(&program)?;
    Ok(program)
}

pub fn compile_source(source: &str) -> CompileResult<CircuitIr> {
    let program = parse_and_validate(source)?;
    lower(&program)
}
