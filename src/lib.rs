#![forbid(unsafe_code)]

pub mod analysis;
pub mod ast;
pub mod backend;
pub mod builtins;
pub mod compiler;
pub mod error;
pub mod eval;
pub mod field;
pub mod hir;
pub mod ir;
pub mod lexer;
pub mod optimize;
pub mod parser;
pub mod pretty;
pub mod serialize;
pub mod source;
pub mod span;
pub mod trace;
pub mod typecheck;
pub mod verify;

pub use compiler::{
    compile_path, compile_source, dependency_graph, parse_and_typecheck, parse_and_typecheck_path,
    parse_and_validate, parse_and_validate_path,
};
pub use source::stdlib_catalog;

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::compile_path;
    use crate::compile_source;
    use crate::dependency_graph;
    use crate::eval::{RuntimeInputs, execute};
    use crate::field::FieldElement;
    use crate::ir::{CircuitIr, Constraint, NamedInput, Operand, Output};
    use crate::optimize::optimize;
    use crate::pretty::render_program;
    use crate::source::resolve_program;
    use crate::stdlib_catalog;
    use crate::trace::trace_execution;
    use crate::verify::verify;

    static NEXT_TEMP_TREE_ID: AtomicU64 = AtomicU64::new(0);

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

    #[test]
    fn rejects_duplicate_declarations() {
        let source = r#"
circuit broken {
    public x: field;
    private x: field;
    expose x;
}
"#;

        let err = compile_source(source).expect_err("duplicate declarations should fail");
        assert!(err.message.contains("duplicate declaration for `x`"));
    }

    #[test]
    fn detects_constraint_failure() {
        let source = r#"
circuit bad_math {
    public x: field;
    private y: field;
    let z = x * y;
    constrain z == 10;
    expose z;
}
"#;

        let ir = compile_source(source).expect("program should compile");
        let mut inputs = RuntimeInputs::default();
        inputs.insert_public("x", FieldElement::from_i128(2));
        inputs.insert_private("y", FieldElement::from_i128(3));

        let err = execute(&ir, &inputs).expect_err("witness should fail constraints");
        assert!(err.message.contains("constraint failed"));
    }

    #[test]
    fn builds_typed_hir_with_resolved_bindings() {
        let source = r#"
circuit typed {
    public x: field;
    let y = x + 2;
    expose y;
}
"#;

        let typed = crate::parse_and_typecheck(source).expect("program should typecheck");
        assert_eq!(typed.circuit.items.len(), 3);

        let let_stmt = match &typed.circuit.items[1] {
            crate::hir::Item::Statement(crate::hir::Statement::Let(stmt)) => stmt,
            _ => panic!("expected let statement"),
        };

        match &let_stmt.expr.kind {
            crate::hir::ExprKind::Binary { lhs, rhs, .. } => {
                match &lhs.kind {
                    crate::hir::ExprKind::Reference(binding) => assert_eq!(binding.name, "x"),
                    _ => panic!("expected resolved reference"),
                }
                match &rhs.kind {
                    crate::hir::ExprKind::Constant(value) => assert_eq!(*value, 2),
                    _ => panic!("expected constant"),
                }
            }
            _ => panic!("expected binary expression"),
        }
    }

    #[test]
    fn compiles_and_executes_function_calls() {
        let source = r#"
circuit functions {
    fn sqr(value: field) -> field {
        value * value
    }

    fn adjust(value: field, offset: field) -> field {
        sqr(value) + offset
    }

    public expected: field;
    private x: field;

    let result = adjust(x, 5);
    constrain result == expected;
    expose result;
}
"#;

        let ir = compile_source(source).expect("function program should compile");
        let mut inputs = RuntimeInputs::default();
        inputs.insert_public("expected", FieldElement::from_i128(21));
        inputs.insert_private("x", FieldElement::from_i128(4));

        let result = execute(&ir, &inputs).expect("function witness should satisfy constraints");
        assert_eq!(result.outputs[0].1, FieldElement::from_i128(21));
    }

    #[test]
    fn rejects_undeclared_functions() {
        let source = r#"
circuit broken {
    public x: field;
    let y = missing(x);
    expose y;
}
"#;

        let err = compile_source(source).expect_err("unknown functions should fail");
        assert!(err.message.contains("undeclared function `missing`"));
    }

    #[test]
    fn rejects_function_arity_mismatches() {
        let source = r#"
circuit broken {
    fn dbl(value: field) -> field {
        value + value
    }

    public x: field;
    let y = dbl(x, 1);
    expose y;
}
"#;

        let err = compile_source(source).expect_err("wrong argument count should fail");
        assert!(err.message.contains("expects 1 arguments but got 2"));
    }

    #[test]
    fn compiles_and_executes_builtins() {
        let source = r#"
circuit builtins {
    public expected: field;
    private x: field;
    private y: field;

    let result = sum3(square(x), double(y), 1);
    constrain result == expected;
    expose result;
}
"#;

        let ir = compile_source(source).expect("builtin program should compile");
        let mut inputs = RuntimeInputs::default();
        inputs.insert_public("expected", FieldElement::from_i128(18));
        inputs.insert_private("x", FieldElement::from_i128(3));
        inputs.insert_private("y", FieldElement::from_i128(4));

        let result = execute(&ir, &inputs).expect("builtin witness should satisfy constraints");
        assert_eq!(result.outputs[0].1, FieldElement::from_i128(18));
    }

    #[test]
    fn rejects_duplicate_builtin_names() {
        let source = r#"
circuit broken {
    fn square(value: field) -> field {
        value
    }

    public x: field;
    expose x;
}
"#;

        let err = compile_source(source).expect_err("builtin names should be reserved");
        assert!(err.message.contains("duplicate declaration for `square`"));
    }

    #[test]
    fn compiles_and_executes_boolean_conditionals() {
        let source = r#"
circuit booleans {
    public expected: field;
    public gate: bool;
    private x: field;
    private y: field;

    let selected = if gate { x + 5 } else { y + 1 };
    constrain selected == expected;
    expose selected;
    expose gate as gate_value;
}
"#;

        let ir = compile_source(source).expect("boolean program should compile");
        let mut inputs = RuntimeInputs::default();
        inputs.insert_public("expected", FieldElement::from_i128(12));
        inputs.insert_public("gate", FieldElement::from_i128(1));
        inputs.insert_private("x", FieldElement::from_i128(7));
        inputs.insert_private("y", FieldElement::from_i128(20));

        let result = execute(&ir, &inputs).expect("boolean witness should satisfy constraints");
        assert_eq!(result.outputs[0].1, FieldElement::from_i128(12));
        assert_eq!(result.outputs[1].1, FieldElement::from_i128(1));
    }

    #[test]
    fn compiles_and_executes_boolean_builtins() {
        let source = r#"
circuit bool_logic {
    public expected: bool;
    private a: bool;
    private b: bool;

    let result = xor(or(a, b), and(a, b));
    constrain result == expected;
    expose result;
}
"#;

        let ir = compile_source(source).expect("boolean builtin program should compile");
        let mut inputs = RuntimeInputs::default();
        inputs.insert_public("expected", FieldElement::from_i128(1));
        inputs.insert_private("a", FieldElement::from_i128(1));
        inputs.insert_private("b", FieldElement::from_i128(0));

        let result = execute(&ir, &inputs).expect("boolean builtin witness should satisfy");
        assert_eq!(result.outputs[0].1, FieldElement::from_i128(1));
    }

    #[test]
    fn rejects_arithmetic_on_bool_values() {
        let source = r#"
circuit broken {
    public gate: bool;
    let y = gate + 1;
    expose y;
}
"#;

        let err = compile_source(source).expect_err("bool arithmetic should fail");
        assert!(err.message.contains("expects `field` operands"));
    }

    #[test]
    fn rejects_if_branch_type_mismatches() {
        let source = r#"
circuit broken {
    public gate: bool;
    public x: field;
    let y = if gate { x } else { false };
    expose y;
}
"#;

        let err = compile_source(source).expect_err("if branch mismatch should fail");
        assert!(err.message.contains("branches must have the same type"));
    }

    #[test]
    fn optimizer_eliminates_redundant_operations() {
        let source = r#"
circuit optimize_demo {
    public x: field;

    let a = x + 0;
    let b = a * 1;
    let c = b * 0;
    expose a;
    expose c;
}
"#;

        let ir = compile_source(source).expect("program should compile");
        let optimized = optimize(&ir);
        assert!(optimized.operations.is_empty());
        assert_eq!(optimized.outputs.len(), 2);
        assert_eq!(optimized.outputs[0].value, Operand::Wire(0));
        assert_eq!(
            optimized.outputs[1].value,
            Operand::Const(FieldElement::zero())
        );
        verify(&optimized).expect("optimized circuit should verify");
    }

    #[test]
    fn verifier_rejects_forward_references() {
        let malformed = CircuitIr {
            name: "malformed".to_string(),
            public_inputs: vec![NamedInput {
                binding: 0,
                name: "x".to_string(),
                ty: crate::ast::Type::Field,
                wire: 0,
            }],
            private_inputs: Vec::new(),
            operations: vec![crate::ir::Operation {
                out: 1,
                kind: crate::ir::OpKind::Add(
                    Operand::Wire(2),
                    Operand::Const(FieldElement::zero()),
                ),
            }],
            constraints: vec![Constraint {
                lhs: Operand::Wire(1),
                rhs: Operand::Const(FieldElement::zero()),
            }],
            outputs: vec![Output {
                name: "out".to_string(),
                value: Operand::Wire(1),
            }],
            next_wire: 2,
        };

        let err = verify(&malformed).expect_err("forward references should fail verification");
        assert!(err.message.contains("referenced before definition"));
    }

    #[test]
    fn trace_execution_captures_wires_and_constraints() {
        let ir = compile_source(SAMPLE).expect("sample program should compile");
        let mut inputs = RuntimeInputs::default();
        inputs.insert_public("x", FieldElement::from_i128(5));
        inputs.insert_private("y", FieldElement::from_i128(7));

        let trace = trace_execution(&ir, &inputs).expect("sample program should trace");
        assert_eq!(trace.operations.len(), 2);
        assert_eq!(trace.constraints.len(), 1);
        assert_eq!(trace.outputs.len(), 2);
        assert_eq!(trace.wires.len(), ir.next_wire);
        assert_eq!(trace.wires[2].value, FieldElement::from_i128(35));
        assert!(trace.constraints[0].satisfied);
        assert_eq!(trace.outputs[0].value, FieldElement::from_i128(35));
        assert!(trace.to_json().contains("\"circuit\":\"product_check\""));
    }

    #[test]
    fn compile_path_resolves_included_fragments() {
        let root = temp_tree(&[
            (
                "main.zk",
                r#"
circuit included {
    include "helpers.zk";
    public expected: field;
    private x: field;

    let y = sqr(x) + 1;
    constrain y == expected;
    expose y;
}
"#,
            ),
            (
                "helpers.zk",
                r#"
fn sqr(value: field) -> field {
    value * value
}
"#,
            ),
        ]);

        let ir = compile_path(root.join("main.zk")).expect("include tree should compile");
        let mut inputs = RuntimeInputs::default();
        inputs.insert_public("expected", FieldElement::from_i128(10));
        inputs.insert_private("x", FieldElement::from_i128(3));

        let result = execute(&ir, &inputs).expect("included circuit should execute");
        assert_eq!(result.outputs[0].1, FieldElement::from_i128(10));
    }

    #[test]
    fn dependency_graph_tracks_nested_includes() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/includes/main.zk");
        let graph = dependency_graph(&root).expect("graph should resolve");
        let rendered = graph.to_string();
        assert!(rendered.contains("examples/includes/main.zk"));
        assert!(rendered.contains("examples/includes/lib/math.zk"));
        assert!(rendered.contains("examples/includes/shared/offsets.zk"));
        assert!(graph.to_json().contains("\"edges\""));
        assert!(graph.to_json().contains("\"kind\":\"include\""));
    }

    #[test]
    fn compile_path_rejects_circular_includes() {
        let root = temp_tree(&[
            (
                "main.zk",
                r#"
circuit circular {
    include "a.zk";
    public x: field;
    expose x;
}
"#,
            ),
            (
                "a.zk",
                r#"
include "b.zk";

fn left(value: field) -> field {
    value
}
"#,
            ),
            (
                "b.zk",
                r#"
include "a.zk";

fn right(value: field) -> field {
    value
}
"#,
            ),
        ]);

        let err = compile_path(root.join("main.zk")).expect_err("cycle should fail");
        assert!(err.message.contains("circular include detected"));
    }

    #[test]
    fn compile_path_supports_namespaced_imports() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/imports/main.zk");
        let ir = compile_path(&root).expect("import example should compile");
        let mut inputs = RuntimeInputs::default();
        inputs.insert_public("expected", FieldElement::from_i128(13));
        inputs.insert_public("gate", FieldElement::from_i128(1));
        inputs.insert_private("x", FieldElement::from_i128(5));
        inputs.insert_private("y", FieldElement::from_i128(1));

        let result = execute(&ir, &inputs).expect("import example should execute");
        assert_eq!(result.outputs[0].1, FieldElement::from_i128(13));
    }

    #[test]
    fn resolve_program_renders_namespaced_imports() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/imports/main.zk");
        let resolved = resolve_program(&root).expect("import example should resolve");
        let rendered = render_program(&resolved.program);

        assert!(rendered.contains("fn left::score(value: field, gate: bool) -> field"));
        assert!(rendered.contains("fn right::arith::scale3(value: field) -> field"));
        assert!(rendered.contains("let left_score = left::score(x, gate);"));
        assert!(resolved.to_json().contains("\"kind\":\"import\""));
        assert!(
            resolved
                .to_json()
                .contains("\"namespace\":\"left::select\"")
        );
    }

    #[test]
    fn compile_path_allows_same_module_under_different_aliases() {
        let root = temp_tree(&[
            (
                "main.zk",
                r#"
circuit aliases {
    import "helpers.zk" as left;
    import "helpers.zk" as right;
    public expected: field;
    private x: field;

    let result = left::twice(x) + right::twice(x);
    constrain result == expected;
    expose result;
}
"#,
            ),
            (
                "helpers.zk",
                r#"
fn twice(value: field) -> field {
    value + value
}
"#,
            ),
        ]);

        let ir = compile_path(root.join("main.zk")).expect("aliased imports should compile");
        let mut inputs = RuntimeInputs::default();
        inputs.insert_public("expected", FieldElement::from_i128(12));
        inputs.insert_private("x", FieldElement::from_i128(3));

        let result = execute(&ir, &inputs).expect("aliased imports should execute");
        assert_eq!(result.outputs[0].1, FieldElement::from_i128(12));
    }

    #[test]
    fn compile_path_rejects_statements_in_imported_modules() {
        let root = temp_tree(&[
            (
                "main.zk",
                r#"
circuit bad_import {
    import "module.zk" as mod1;
    public x: field;
    expose x;
}
"#,
            ),
            (
                "module.zk",
                r#"
let y = 1;
"#,
            ),
        ]);

        let err = compile_path(root.join("main.zk")).expect_err("imported statements should fail");
        assert!(
            err.message
                .contains("imported modules may only contain functions")
        );
    }

    #[test]
    fn stdlib_catalog_lists_known_modules() {
        let catalog = stdlib_catalog().expect("stdlib catalog should load");
        assert!(!catalog.modules.is_empty());
        assert!(
            catalog
                .modules
                .iter()
                .any(|module| module.logical_path == "arith/constants.zk")
        );
        assert!(
            catalog
                .modules
                .iter()
                .any(|module| module.logical_path == "patterns/pipeline.zk")
        );
        assert!(catalog.total_lines > 0);
        assert!(catalog.to_json().contains("\"total_modules\""));
    }

    fn temp_tree(files: &[(&str, &str)]) -> PathBuf {
        let mut root = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let sequence = NEXT_TEMP_TREE_ID.fetch_add(1, Ordering::Relaxed);
        root.push(format!("zkc-tree-{nonce}-{sequence}"));
        fs::create_dir_all(&root).expect("temp tree root should be created");

        for (relative, contents) in files {
            let path = root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("temp tree directory should be created");
            }
            fs::write(&path, contents).expect("temp tree file should be written");
        }

        root
    }

    #[allow(dead_code)]
    fn _path_label(path: &Path) -> String {
        path.display().to_string()
    }
}
