# zkc

`zkc` is a small zero-knowledge-oriented language and compiler written from scratch in Rust.

It is designed as a minimal foundation for a proving DSL: the language supports public and private
field inputs, arithmetic expressions, equality constraints, and exposed outputs. The current
implementation compiles source programs into a simple arithmetic-circuit IR and can execute witness
assignments locally to check whether all constraints are satisfied.

## Status

This project is currently experimental.

It is useful as:

- a compact reference implementation of a tiny ZK DSL
- a starting point for adding a real proving backend
- a base for experimenting with language design, circuit IR, and witness evaluation

It is not yet suitable for production use because it does not generate proofs or verification keys.

## Features

- `public` and `private` field inputs
- `let` bindings over arithmetic expressions
- `constrain lhs == rhs;` equality constraints
- `expose expr;` and `expose expr as name;` public outputs
- constant folding for literal-only expressions
- lowering into a wire-based arithmetic circuit IR
- witness execution and constraint checking from the CLI

## Quick Start

### Requirements

- Rust
- Cargo

### Build

```bash
cargo build
```

### Run tests

```bash
cargo test
```

### Compile an example circuit

```bash
cargo run -- compile examples/product.zk
```

### Run the circuit with inputs

```bash
cargo run -- run examples/product.zk --public x=5 --private y=7
```

Expected output:

```text
constraint system satisfied over field modulus 18446744073709551557
product = 35
shifted_value = 38
```

## Example Program

```zk
circuit product_check {
    public x: field;
    private y: field;

    let product = x * y;
    let shifted = product + 3;
    constrain shifted == 38;
    expose product;
    expose shifted as shifted_value;
}
```

## Language Overview

### Grammar

```text
program        := "circuit" IDENT "{" item* "}"
item           := input_decl | stmt
input_decl     := ("public" | "private") IDENT ":" "field" ";"
stmt           := let_stmt | constrain_stmt | expose_stmt
let_stmt       := "let" IDENT "=" expr ";"
constrain_stmt := "constrain" expr "==" expr ";"
expose_stmt    := "expose" expr ("as" IDENT)? ";"
expr           := term (("+" | "-") term)*
term           := unary ("*" unary)*
unary          := "-" unary | primary
primary        := NUMBER | IDENT | "(" expr ")"
```

### Semantics

- All values live in the prime field with modulus `18446744073709551557`.
- Expressions currently support `+`, `-`, `*`, parentheses, identifiers, and integer literals.
- Public and private inputs are mapped to wires in the lowered circuit.
- Each `constrain` statement becomes an equality check over lowered operands.
- Each `expose` statement becomes a named public output.

## CLI

```text
zkc compile <file.zk>
zkc run <file.zk> [--public name=value]... [--private name=value]...
```

## Repository Layout

```text
src/main.rs       CLI entrypoint
src/parser.rs     Parser
src/lexer.rs      Lexer
src/typecheck.rs  Name resolution and basic validation
src/ir.rs         Arithmetic circuit IR and lowering
src/eval.rs       Witness execution and constraint checking
examples/         Example circuits
```

## Current Limitations

- No proof generation
- No verification backend
- Only one data type: `field`
- No functions, modules, arrays, structs, or control flow
- No package manager or standard library
- No optimization pipeline beyond simple constant folding

## Development Direction

The natural next steps are:

- introduce a typed intermediate representation
- add richer types and structured language features
- target a real backend such as `R1CS`, `AIR`, or `PLONKish` constraints
- implement `keygen`, `prove`, and `verify`
- add better diagnostics, fuzzing, benchmarks, and compatibility tests

## License

No license file has been added yet.
