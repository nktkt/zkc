#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::path::Path;
use std::process;

use zkc::analysis::analyze;
use zkc::builtins::all as all_builtins;
use zkc::dependency_graph;
use zkc::eval::{RuntimeInputs, execute};
use zkc::field::FieldElement;
use zkc::optimize::optimize;
use zkc::pretty::render_program;
use zkc::serialize::ir_to_json;
use zkc::source::resolve_program;
use zkc::stdlib_catalog;
use zkc::trace::trace_execution;
use zkc::verify::verify;

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
        "list-builtins" => {
            if args.next().is_some() {
                return Err("unexpected arguments after command".to_string());
            }
            for builtin in all_builtins() {
                println!("{}  {}", builtin.signature, builtin.description);
            }
            Ok(())
        }
        "list-stdlib" => {
            let json = match args.next() {
                None => false,
                Some(flag) if flag == "--json" => {
                    if args.next().is_some() {
                        return Err("unexpected arguments after `--json`".to_string());
                    }
                    true
                }
                Some(flag) => return Err(format!("unknown argument `{flag}`")),
            };

            let catalog = stdlib_catalog().map_err(|err| err.to_string())?;
            if json {
                println!("{}", catalog.to_json());
            } else {
                println!("{catalog}");
            }
            Ok(())
        }
        "check" => {
            let path = args.next().ok_or_else(|| usage().to_string())?;
            if args.next().is_some() {
                return Err("unexpected arguments after source path".to_string());
            }

            let ir = compile_file(&path)?;
            println!(
                "ok: {} ({} constraints, {} operations)",
                ir.name,
                ir.constraints.len(),
                ir.operations.len()
            );
            Ok(())
        }
        "deps" => {
            let path = args.next().ok_or_else(|| usage().to_string())?;
            let json = match args.next() {
                None => false,
                Some(flag) if flag == "--json" => {
                    if args.next().is_some() {
                        return Err("unexpected arguments after `--json`".to_string());
                    }
                    true
                }
                Some(flag) => return Err(format!("unknown argument `{flag}`")),
            };

            let graph = dependency_graph(Path::new(&path)).map_err(|err| err.to_string())?;
            if json {
                println!("{}", graph.to_json());
            } else {
                println!("{graph}");
            }
            Ok(())
        }
        "resolve" => {
            let path = args.next().ok_or_else(|| usage().to_string())?;
            let json = match args.next() {
                None => false,
                Some(flag) if flag == "--json" => {
                    if args.next().is_some() {
                        return Err("unexpected arguments after `--json`".to_string());
                    }
                    true
                }
                Some(flag) => return Err(format!("unknown argument `{flag}`")),
            };

            let resolved = resolve_program(Path::new(&path)).map_err(|err| err.to_string())?;
            if json {
                println!("{}", resolved.to_json());
            } else {
                print!("{}", render_program(&resolved.program));
            }
            Ok(())
        }
        "verify-ir" => {
            let path = args.next().ok_or_else(|| usage().to_string())?;
            if args.next().is_some() {
                return Err("unexpected arguments after source path".to_string());
            }

            let ir = compile_file(&path)?;
            verify(&ir).map_err(|err| err.to_string())?;
            println!("verified: {} ({} wires)", ir.name, ir.next_wire);
            Ok(())
        }
        "compile" => {
            let path = args.next().ok_or_else(|| usage().to_string())?;
            if args.next().is_some() {
                return Err("unexpected arguments after source path".to_string());
            }

            let ir = compile_file(&path)?;
            println!("{ir}");
            Ok(())
        }
        "compile-json" => {
            let path = args.next().ok_or_else(|| usage().to_string())?;
            if args.next().is_some() {
                return Err("unexpected arguments after source path".to_string());
            }

            let ir = compile_file(&path)?;
            println!("{}", ir_to_json(&ir));
            Ok(())
        }
        "analyze" => {
            let path = args.next().ok_or_else(|| usage().to_string())?;
            let json = match args.next() {
                None => false,
                Some(flag) if flag == "--json" => {
                    if args.next().is_some() {
                        return Err("unexpected arguments after `--json`".to_string());
                    }
                    true
                }
                Some(flag) => return Err(format!("unknown argument `{flag}`")),
            };

            let ir = compile_file(&path)?;
            let report = analyze(&ir);
            if json {
                println!("{}", report.to_json());
            } else {
                println!("{report}");
            }
            Ok(())
        }
        "optimize" => {
            let path = args.next().ok_or_else(|| usage().to_string())?;
            let json = match args.next() {
                None => false,
                Some(flag) if flag == "--json" => {
                    if args.next().is_some() {
                        return Err("unexpected arguments after `--json`".to_string());
                    }
                    true
                }
                Some(flag) => return Err(format!("unknown argument `{flag}`")),
            };

            let ir = compile_file(&path)?;
            let optimized = optimize(&ir);
            verify(&optimized).map_err(|err| err.to_string())?;
            if json {
                println!("{}", ir_to_json(&optimized));
            } else {
                println!("{optimized}");
            }
            Ok(())
        }
        "trace" => {
            let path = args.next().ok_or_else(|| usage().to_string())?;
            let mut remaining = args.collect::<Vec<_>>();
            let json = matches!(remaining.first().map(String::as_str), Some("--json"));
            if json {
                remaining.remove(0);
            }

            let ir = compile_file(&path)?;
            let inputs = parse_runtime_inputs(remaining)?;
            let trace = trace_execution(&ir, &inputs).map_err(|err| err.to_string())?;
            if json {
                println!("{}", trace.to_json());
            } else {
                println!("{trace}");
            }
            Ok(())
        }
        "witness-json" => {
            let path = args.next().ok_or_else(|| usage().to_string())?;
            let ir = compile_file(&path)?;
            let inputs = parse_runtime_inputs(args.collect())?;
            let trace = trace_execution(&ir, &inputs).map_err(|err| err.to_string())?;
            println!("{}", trace.to_json());
            Ok(())
        }
        "run" => {
            let path = args.next().ok_or_else(|| usage().to_string())?;
            let ir = compile_file(&path)?;
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

fn compile_file(path: &str) -> Result<zkc::ir::CircuitIr, String> {
    if fs::metadata(path).is_err() {
        return Err(format!("failed to read `{path}`: file does not exist"));
    }
    zkc::compile_path(Path::new(path)).map_err(|err| err.to_string())
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

    let parsed_value = match value {
        "true" => FieldElement::from_i128(1),
        "false" => FieldElement::zero(),
        _ => FieldElement::parse(value)?,
    };
    Ok((name.to_string(), parsed_value))
}

fn usage() -> &'static str {
    "usage:
  zkc list-builtins
  zkc list-stdlib [--json]
  zkc check <file.zk>
  zkc deps <file.zk> [--json]
  zkc resolve <file.zk> [--json]
  zkc verify-ir <file.zk>
  zkc compile <file.zk>
  zkc compile-json <file.zk>
  zkc analyze <file.zk> [--json]
  zkc optimize <file.zk> [--json]
  zkc trace <file.zk> [--json] [--public name=value]... [--private name=value]...
  zkc witness-json <file.zk> [--public name=value]... [--private name=value]...
  zkc run <file.zk> [--public name=value]... [--private name=value]..."
}

enum InputVisibility {
    Public,
    Private,
}
