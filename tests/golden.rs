use std::fs;
use std::path::{Path, PathBuf};

use zkc::compile_path;
use zkc::eval::{RuntimeInputs, execute};
use zkc::field::FieldElement;

#[test]
fn golden_ok_programs_compile_and_run() {
    for path in list_files("tests/golden/ok") {
        let source = fs::read_to_string(&path).expect("should read golden source");
        let directives = parse_directives(&source);
        let ir = compile_path(&path)
            .unwrap_or_else(|err| panic!("{} should compile, got {err}", path.display()));

        if let Some(run_args) = directives.run {
            let inputs = parse_runtime_inputs(&run_args).unwrap_or_else(|err| {
                panic!("{} has invalid RUN directive: {err}", path.display())
            });
            let result = execute(&ir, &inputs)
                .unwrap_or_else(|err| panic!("{} should execute, got {err}", path.display()));

            for (name, value) in directives.expected_outputs {
                let actual = result
                    .outputs
                    .iter()
                    .find(|(output_name, _)| output_name == &name)
                    .unwrap_or_else(|| panic!("{} missing output `{name}`", path.display()));
                assert_eq!(
                    actual.1,
                    FieldElement::parse(&value).expect("expected output should parse"),
                    "{} output `{name}` mismatch",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn golden_error_programs_fail_with_expected_messages() {
    for path in list_files("tests/golden/err") {
        let source = fs::read_to_string(&path).expect("should read golden source");
        let directives = parse_directives(&source);
        let err = match compile_path(&path) {
            Ok(_) => panic!("{} should fail compilation", path.display()),
            Err(err) => err,
        };
        let expected = directives
            .expected_error
            .unwrap_or_else(|| panic!("{} missing EXPECT-ERROR directive", path.display()));
        assert!(
            err.message.contains(&expected),
            "{} expected error containing `{expected}`, got `{}`",
            path.display(),
            err.message
        );
    }
}

#[derive(Debug, Default)]
struct Directives {
    run: Option<Vec<String>>,
    expected_outputs: Vec<(String, String)>,
    expected_error: Option<String>,
}

fn list_files(relative: &str) -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    let mut files = fs::read_dir(&root)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", root.display()))
        .map(|entry| entry.expect("directory entry should be readable").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "zk"))
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn parse_directives(source: &str) -> Directives {
    let mut directives = Directives::default();

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(payload) = trimmed.strip_prefix("# RUN:") {
            directives.run = Some(split_args(payload.trim()));
        } else if let Some(payload) = trimmed.strip_prefix("# EXPECT-OUTPUT:") {
            let (name, value) = payload
                .trim()
                .split_once('=')
                .expect("EXPECT-OUTPUT should be NAME=VALUE");
            directives
                .expected_outputs
                .push((name.trim().to_string(), value.trim().to_string()));
        } else if let Some(payload) = trimmed.strip_prefix("# EXPECT-ERROR:") {
            directives.expected_error = Some(payload.trim().to_string());
        }
    }

    directives
}

fn split_args(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
}

fn parse_runtime_inputs(args: &[String]) -> Result<RuntimeInputs, String> {
    let mut inputs = RuntimeInputs::default();
    let mut index = 0;

    while index < args.len() {
        let flag = &args[index];
        let public = match flag.as_str() {
            "--public" => true,
            "--private" => false,
            _ => return Err(format!("unknown flag `{flag}`")),
        };

        index += 1;
        let assignment = args
            .get(index)
            .ok_or_else(|| format!("missing assignment after `{flag}`"))?;
        let (name, value) = assignment
            .split_once('=')
            .ok_or_else(|| format!("invalid assignment `{assignment}`"))?;
        let field = match value {
            "true" => FieldElement::from_i128(1),
            "false" => FieldElement::zero(),
            _ => FieldElement::parse(value)?,
        };
        if public {
            inputs.insert_public(name, field);
        } else {
            inputs.insert_private(name, field);
        }
        index += 1;
    }

    Ok(inputs)
}

#[allow(dead_code)]
fn _path_label(path: &Path) -> String {
    path.display().to_string()
}
