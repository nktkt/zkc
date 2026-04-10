use std::path::Path;

use crate::ast::Program;
use crate::error::CompileResult;
use crate::hir;
use crate::ir::{CircuitIr, lower};
use crate::parser;
use crate::source::{DependencyGraph, resolve_program};
use crate::typecheck;

pub fn parse_and_validate(source: &str) -> CompileResult<Program> {
    let program = parser::parse(source)?;
    typecheck::validate(&program)?;
    Ok(program)
}

pub fn compile_source(source: &str) -> CompileResult<CircuitIr> {
    let program = parse_and_typecheck(source)?;
    lower(&program)
}

pub fn compile_path(path: impl AsRef<Path>) -> CompileResult<CircuitIr> {
    let program = parse_and_typecheck_path(path)?;
    lower(&program)
}

pub fn parse_and_typecheck(source: &str) -> CompileResult<hir::Program> {
    let program = parser::parse(source)?;
    typecheck::typecheck(&program)
}

pub fn parse_and_typecheck_path(path: impl AsRef<Path>) -> CompileResult<hir::Program> {
    let resolved = resolve_program(path)?;
    typecheck::typecheck(&resolved.program)
}

pub fn parse_and_validate_path(path: impl AsRef<Path>) -> CompileResult<Program> {
    let resolved = resolve_program(path)?;
    typecheck::validate(&resolved.program)?;
    Ok(resolved.program)
}

pub fn dependency_graph(path: impl AsRef<Path>) -> CompileResult<DependencyGraph> {
    crate::source::dependency_graph(path)
}
