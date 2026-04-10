pub mod ast;
pub mod compiler;
pub mod error;
pub mod eval;
pub mod field;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod typecheck;

pub use compiler::{compile_source, parse_and_validate};

#[cfg(test)]
mod tests {
    use crate::compile_source;
    use crate::eval::{RuntimeInputs, execute};
    use crate::field::FieldElement;

    const SAMPLE: &str = r#"
circuit product_check {
    public x: field;
    private y: field;

    let product = x * y;
    let shifted = product + 3;
    constrain shifted == 38;
    expose product;
    expose shifted as shifted_value;
}
"#;

    #[test]
    fn compiles_and_executes_sample_program() {
        let ir = compile_source(SAMPLE).expect("sample program should compile");
        let mut inputs = RuntimeInputs::default();
        inputs.insert_public("x", FieldElement::from_i128(5));
        inputs.insert_private("y", FieldElement::from_i128(7));

        let result = execute(&ir, &inputs).expect("sample witness should satisfy constraints");
        assert_eq!(result.outputs.len(), 2);
        assert_eq!(result.outputs[0].0, "product");
        assert_eq!(result.outputs[0].1, FieldElement::from_i128(35));
        assert_eq!(result.outputs[1].0, "shifted_value");
        assert_eq!(result.outputs[1].1, FieldElement::from_i128(38));
    }

    #[test]
    fn rejects_undeclared_variables() {
        let source = r#"
circuit broken {
    public x: field;
    let z = x + y;
    expose z;
}
"#;

        let err = compile_source(source).expect_err("undeclared names should fail");
        assert!(err.message.contains("undeclared identifier `y`"));
    }
}
