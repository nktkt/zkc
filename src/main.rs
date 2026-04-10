use std::env;
use std::fs;
use std::process;

use zkc::eval::{RuntimeInputs, execute};
use zkc::field::FieldElement;

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(|| usage().to_string())?;

    match command.as_str() {
        "compile" => {
            let path = args.next().ok_or_else(|| usage().to_string())?;
            if args.next().is_some() {
                return Err("unexpected arguments after source path".to_string());
            }

            let source = fs::read_to_string(&path)
                .map_err(|err| format!("failed to read `{path}`: {err}"))?;
            let ir = zkc::compile_source(&source).map_err(|err| err.to_string())?;
            println!("{ir}");
            Ok(())
        }
        "run" => {
            let path = args.next().ok_or_else(|| usage().to_string())?;
            let source = fs::read_to_string(&path)
                .map_err(|err| format!("failed to read `{path}`: {err}"))?;
            let ir = zkc::compile_source(&source).map_err(|err| err.to_string())?;
            let inputs = parse_runtime_inputs(args.collect())?;
            let result = execute(&ir, &inputs).map_err(|err| err.to_string())?;

            println!(
                "constraint system satisfied over field modulus {}",
                zkc::field::MODULUS
            );
            if result.outputs.is_empty() {
                println!("no exposed outputs");
            } else {
                for (name, value) in result.outputs {
                    println!("{name} = {value}");
                }
            }
            Ok(())
        }
        _ => Err(format!("unknown command `{command}`\n\n{}", usage())),
    }
}

fn parse_runtime_inputs(args: Vec<String>) -> Result<RuntimeInputs, String> {
    let mut inputs = RuntimeInputs::default();
    let mut index = 0;

    while index < args.len() {
        let flag = &args[index];
        let visibility = match flag.as_str() {
            "--public" => InputVisibility::Public,
            "--private" => InputVisibility::Private,
            _ => return Err(format!("unknown argument `{flag}`")),
        };

        index += 1;
        if index >= args.len() {
            return Err(format!("missing assignment after `{flag}`"));
        }

        let (name, value) = parse_assignment(&args[index])?;
        match visibility {
            InputVisibility::Public => inputs.insert_public(name, value),
            InputVisibility::Private => inputs.insert_private(name, value),
        }
        index += 1;
    }

    Ok(inputs)
}

fn parse_assignment(raw: &str) -> Result<(String, FieldElement), String> {
    let (name, value) = raw
        .split_once('=')
        .ok_or_else(|| format!("expected NAME=VALUE assignment, got `{raw}`"))?;

    if name.is_empty() {
        return Err(format!("missing input name in `{raw}`"));
    }

    let parsed_value = FieldElement::parse(value)?;
    Ok((name.to_string(), parsed_value))
}

fn usage() -> &'static str {
    "usage:
  zkc compile <file.zk>
  zkc run <file.zk> [--public name=value]... [--private name=value]..."
}

enum InputVisibility {
    Public,
    Private,
}
