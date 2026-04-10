use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_zkc")
}

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn temp_program(name: &str, source: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    path.push(format!("zkc-{name}-{nonce}.zk"));
    fs::write(&path, source).expect("should write temporary source file");
    path
}

fn temp_tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let mut root = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    root.push(format!("zkc-tree-{name}-{nonce}"));
    fs::create_dir_all(&root).expect("temp tree root should be created");

    for (relative, source) in files {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("temp tree parent should be created");
        }
        fs::write(path, source).expect("temp tree file should be written");
    }

    root
}

#[test]
fn compile_command_emits_lowered_ir() {
    let output = Command::new(binary())
        .arg("compile")
        .arg(repo_path("examples/product.zk"))
        .output()
        .expect("compile command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("circuit product_check"));
    assert!(stdout.contains("w2 = mul w0, w1"));
}

#[test]
fn check_command_reports_summary() {
    let output = Command::new(binary())
        .arg("check")
        .arg(repo_path("examples/product.zk"))
        .output()
        .expect("check command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("ok: product_check"));
    assert!(stdout.contains("1 constraints"));
}

#[test]
fn deps_command_reports_include_graph() {
    let output = Command::new(binary())
        .arg("deps")
        .arg(repo_path("examples/includes/main.zk"))
        .output()
        .expect("deps command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("dependency graph for"));
    assert!(stdout.contains("examples/includes/main.zk"));
    assert!(stdout.contains("lib/math.zk"));
    assert!(stdout.contains("shared/bools.zk"));
    assert!(stdout.contains("[include"));
}

#[test]
fn resolve_command_emits_flattened_source() {
    let output = Command::new(binary())
        .arg("resolve")
        .arg(repo_path("examples/imports/main.zk"))
        .output()
        .expect("resolve command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("fn left::score(value: field, gate: bool) -> field"));
    assert!(stdout.contains("fn right::arith::scale2(value: field) -> field"));
    assert!(stdout.contains("let right_score = right::score(y, gate);"));
}

#[test]
fn resolve_command_can_emit_json() {
    let output = Command::new(binary())
        .arg("resolve")
        .arg(repo_path("examples/imports/main.zk"))
        .arg("--json")
        .output()
        .expect("resolve command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("\"graph\""));
    assert!(stdout.contains("\"kind\":\"import\""));
    assert!(stdout.contains("\"namespace\":\"left::select\""));
    assert!(stdout.contains("\"source\""));
}

#[test]
fn list_stdlib_command_prints_catalog() {
    let output = Command::new(binary())
        .arg("list-stdlib")
        .output()
        .expect("list-stdlib command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("stdlib catalog"));
    assert!(stdout.contains("arith/constants.zk"));
    assert!(stdout.contains("patterns/pipeline.zk"));
}

#[test]
fn compile_json_command_emits_json_artifact() {
    let output = Command::new(binary())
        .arg("compile-json")
        .arg(repo_path("examples/product.zk"))
        .output()
        .expect("compile-json command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("\"name\":\"product_check\""));
    assert!(stdout.contains("\"public_inputs\""));
    assert!(stdout.contains("\"operations\""));
}

#[test]
fn run_command_supports_stdlib_modules() {
    let output = Command::new(binary())
        .arg("run")
        .arg(repo_path("examples/stdlib_demo.zk"))
        .args([
            "--public",
            "expected=24",
            "--public",
            "primary=true",
            "--public",
            "secondary=true",
            "--private",
            "x=2",
            "--private",
            "y=3",
            "--private",
            "z=4",
        ])
        .output()
        .expect("run command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("result = 24"));
}

#[test]
fn run_command_supports_namespaced_imports() {
    let output = Command::new(binary())
        .arg("run")
        .arg(repo_path("examples/imports/main.zk"))
        .args([
            "--public",
            "expected=13",
            "--public",
            "gate=true",
            "--private",
            "x=5",
            "--private",
            "y=1",
        ])
        .output()
        .expect("run command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("result = 13"));
}

#[test]
fn run_command_supports_included_fragments() {
    let output = Command::new(binary())
        .arg("run")
        .arg(repo_path("examples/includes/main.zk"))
        .args([
            "--public",
            "expected=10",
            "--public",
            "gate=true",
            "--private",
            "x=3",
            "--private",
            "y=10",
        ])
        .output()
        .expect("run command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("result = 10"));
    assert!(stdout.contains("gate_value = 1"));
}

#[test]
fn compile_command_rejects_statements_in_imported_modules() {
    let root = temp_tree(
        "bad-import-module",
        &[
            (
                "main.zk",
                r#"
circuit broken {
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
        ],
    );

    let output = Command::new(binary())
        .arg("compile")
        .arg(root.join("main.zk"))
        .output()
        .expect("compile command should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf-8");
    assert!(stderr.contains("imported modules may only contain functions"));
}

#[test]
fn list_builtins_command_prints_catalog() {
    let output = Command::new(binary())
        .arg("list-builtins")
        .output()
        .expect("list-builtins command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("square(value: field) -> field"));
    assert!(stdout.contains("not(value: bool) -> bool"));
    assert!(stdout.contains("weighted_sum3("));
}

#[test]
fn verify_ir_command_reports_success() {
    let output = Command::new(binary())
        .arg("verify-ir")
        .arg(repo_path("examples/product.zk"))
        .output()
        .expect("verify-ir command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("verified: product_check"));
}

#[test]
fn analyze_command_can_emit_json() {
    let output = Command::new(binary())
        .arg("analyze")
        .arg(repo_path("examples/product.zk"))
        .arg("--json")
        .output()
        .expect("analyze command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("\"name\":\"product_check\""));
    assert!(stdout.contains("\"constraints\":1"));
    assert!(stdout.contains("\"operations\":{\"add\":1,\"sub\":0,\"mul\":1,\"total\":2}"));
}

#[test]
fn trace_command_emits_human_readable_witness_trace() {
    let output = Command::new(binary())
        .arg("trace")
        .arg(repo_path("examples/product.zk"))
        .args(["--public", "x=5", "--private", "y=7"])
        .output()
        .expect("trace command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("execution trace for product_check"));
    assert!(stdout.contains("w2 = mul w0 (5) , w1 (7) => 35"));
    assert!(stdout.contains("#0: w3 (38) == 38 (38) [ok]"));
}

#[test]
fn witness_json_command_emits_structured_trace() {
    let output = Command::new(binary())
        .arg("witness-json")
        .arg(repo_path("examples/product.zk"))
        .args(["--public", "x=5", "--private", "y=7"])
        .output()
        .expect("witness-json command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("\"circuit\":\"product_check\""));
    assert!(stdout.contains("\"backend\":\"interpreter\""));
    assert!(stdout.contains("\"wires\":["));
    assert!(stdout.contains("\"constraints\":["));
}

#[test]
fn run_command_supports_bool_inputs_and_if_expressions() {
    let output = Command::new(binary())
        .arg("run")
        .arg(repo_path("examples/booleans.zk"))
        .args([
            "--public",
            "expected=12",
            "--public",
            "gate=true",
            "--private",
            "x=7",
            "--private",
            "y=20",
        ])
        .output()
        .expect("run command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("selected = 12"));
    assert!(stdout.contains("gate_value = 1"));
}

#[test]
fn compile_command_rejects_non_bool_if_condition() {
    let program = temp_program(
        "bad-if-condition",
        r#"
circuit broken {
    public x: field;
    let y = if x { 1 } else { 0 };
    expose y;
}
"#,
    );

    let output = Command::new(binary())
        .arg("compile")
        .arg(&program)
        .output()
        .expect("compile command should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf-8");
    assert!(stderr.contains("`if` condition must have type `bool`"));

    let _ = fs::remove_file(program);
}

#[test]
fn run_command_supports_user_defined_functions() {
    let output = Command::new(binary())
        .arg("run")
        .arg(repo_path("examples/functions.zk"))
        .args(["--public", "expected=16", "--private", "x=3"])
        .output()
        .expect("run command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("result = 16"));
}

#[test]
fn run_command_supports_builtins() {
    let output = Command::new(binary())
        .arg("run")
        .arg(repo_path("examples/builtins.zk"))
        .args([
            "--public",
            "expected=43",
            "--private",
            "a=2",
            "--private",
            "b=3",
            "--private",
            "c=4",
        ])
        .output()
        .expect("run command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("result = 43"));
}

#[test]
fn optimize_command_reduces_simple_circuit() {
    let program = temp_program(
        "optimize",
        r#"
circuit optimize_demo {
    public x: field;

    let a = x + 0;
    let b = a * 1;
    let c = b * 0;
    expose a;
    expose c;
}
"#,
    );

    let output = Command::new(binary())
        .arg("optimize")
        .arg(&program)
        .output()
        .expect("optimize command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(!stdout.contains("add"));
    assert!(!stdout.contains("mul"));
    assert!(stdout.contains("a = w0") || stdout.contains("out0 = w0") || stdout.contains("a ="));

    let _ = fs::remove_file(program);
}

#[test]
fn run_command_reports_outputs() {
    let output = Command::new(binary())
        .arg("run")
        .arg(repo_path("examples/product.zk"))
        .args(["--public", "x=5", "--private", "y=7"])
        .output()
        .expect("run command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("product = 35"));
    assert!(stdout.contains("shifted_value = 38"));
}

#[test]
fn run_command_fails_when_private_input_is_missing() {
    let output = Command::new(binary())
        .arg("run")
        .arg(repo_path("examples/product.zk"))
        .args(["--public", "x=5"])
        .output()
        .expect("run command should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf-8");
    assert!(stderr.contains("missing private input `y`"));
}

#[test]
fn compile_command_rejects_duplicate_bindings() {
    let program = temp_program(
        "duplicate",
        r#"
circuit broken {
    public x: field;
    let x = 1;
    expose x;
}
"#,
    );

    let output = Command::new(binary())
        .arg("compile")
        .arg(&program)
        .output()
        .expect("compile command should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf-8");
    assert!(stderr.contains("duplicate declaration for `x`"));

    let _ = fs::remove_file(program);
}

#[test]
fn run_command_surfaces_constraint_failure() {
    let program = temp_program(
        "constraint-failure",
        r#"
circuit failure {
    public x: field;
    private y: field;
    let product = x * y;
    constrain product == 9;
    expose product;
}
"#,
    );

    let output = Command::new(binary())
        .arg("run")
        .arg(&program)
        .args(["--public", "x=2", "--private", "y=3"])
        .output()
        .expect("run command should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf-8");
    assert!(stderr.contains("constraint failed"));

    let _ = fs::remove_file(program);
}

#[test]
fn compile_command_rejects_unknown_functions() {
    let program = temp_program(
        "unknown-function",
        r#"
circuit broken {
    public x: field;
    let y = unknown(x);
    expose y;
}
"#,
    );

    let output = Command::new(binary())
        .arg("compile")
        .arg(&program)
        .output()
        .expect("compile command should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf-8");
    assert!(stderr.contains("undeclared function `unknown`"));

    let _ = fs::remove_file(program);
}
