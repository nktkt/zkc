use std::fmt;

use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub span: Span,
    pub message: String,
}

impl CompileError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.message, self.span)
    }
}

impl std::error::Error for CompileError {}

pub type CompileResult<T> = Result<T, CompileError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub message: String,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RuntimeError {}

pub type RuntimeResult<T> = Result<T, RuntimeError>;
