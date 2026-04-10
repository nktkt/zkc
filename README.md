# zkc

`zkc` stands for `Zero-Knowledge Compiler`.

It is a small zero-knowledge-oriented language and compiler written from scratch in Rust. The
current project is intentionally narrow: it focuses on a tiny arithmetic DSL with public inputs,
private inputs, expressions over a finite field, equality constraints, and named public outputs.

Today, `zkc` is best viewed as a compact compiler prototype for ZK language and circuit design. It
parses source code, performs basic validation, lowers programs into a wire-based arithmetic circuit
IR, and evaluates witness assignments locally to check whether the constraint system is satisfied.

## Status

This repository is experimental.

What it is:

- a tiny reference implementation of a ZK-oriented DSL
- a clean starting point for adding a real proving backend
- a playground for compiler passes, circuit IR design, and witness evaluation

What it is not yet:

- a prover
- a verifier
- a production-ready language runtime

## Current Capabilities

- `public` and `private` field inputs
- arithmetic expressions with `+`, `-`, `*`, and parentheses
- `let` bindings
- `constrain lhs == rhs;` equality constraints
- `expose expr;` and `expose expr as name;` public outputs
- constant folding for literal-only expressions
- lowering into a simple arithmetic circuit IR
- local witness execution and constraint checking from the CLI

## Quick Start

### Requirements

- Rust
- Cargo

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

### Compile a circuit

```bash
cargo run -- compile examples/product.zk
```

### Run a circuit with witness values

```bash
cargo run -- run examples/product.zk --public x=5 --private y=7
```

Expected output:

```text
constraint system satisfied over field modulus 18446744073709551557
product = 35
shifted_value = 38
```

## Example

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

## Language Sketch

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
- Public and private inputs are assigned to wires in the lowered circuit.
- `let` bindings name intermediate expressions.
- `constrain` emits equality checks over lowered operands.
- `expose` emits named public outputs.

## Compiler Pipeline

```text
source
  -> lexer
  -> parser
  -> basic validation
  -> arithmetic circuit IR
  -> witness execution / constraint checking
```

Key files:

- `src/main.rs`: CLI entrypoint
- `src/lexer.rs`: lexer
- `src/parser.rs`: parser
- `src/typecheck.rs`: name resolution and basic validation
- `src/ir.rs`: arithmetic circuit IR and lowering
- `src/eval.rs`: witness execution and constraint checking

## Limitations

- no proof generation
- no verification backend
- only one data type: `field`
- no functions, modules, arrays, structs, or control flow
- no package manager or standard library
- no optimization pipeline beyond simple constant folding

## Development Direction

The next major steps are:

- introduce a typed intermediate representation
- add richer types and structured language features
- target a real backend such as `R1CS`, `AIR`, or `PLONKish`
- implement `keygen`, `prove`, and `verify`
- improve diagnostics, fuzzing, benchmarks, and compatibility tests

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
