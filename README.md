# zkc

`zkc` stands for `Zero-Knowledge Compiler`.

It is a small zero-knowledge-oriented language and compiler written from scratch in Rust. The
current project is intentionally narrow: it focuses on a tiny arithmetic DSL with public inputs,
private inputs, `field` and `bool` values, expressions over a finite field, equality constraints,
and named public outputs.

Today, `zkc` is best viewed as a compact compiler prototype for ZK language and circuit design. It
parses source code, builds a typed high-level IR, lowers programs into a wire-based arithmetic
circuit IR, and evaluates witness assignments locally to check whether the constraint system is
satisfied.

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

- `public` and `private` inputs over `field` and `bool`
- compiler-provided arithmetic and boolean built-ins
- pure user-defined functions with typed parameters and typed return values
- arithmetic expressions with `+`, `-`, `*`, and parentheses
- `if cond { then } else { otherwise }` expressions
- file-based composition with `include "path.zk";`
- namespaced module composition with `import "path.zk" as alias;`
- experimental standard-library fragments through `include "@std/...";`
- `let` bindings
- `constrain lhs == rhs;` equality constraints
- `expose expr;` and `expose expr as name;` public outputs
- constant folding for literal-only expressions
- lowering into a simple arithmetic circuit IR
- local witness execution and constraint checking from the CLI
- human-readable and JSON witness traces for interpreter execution

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

### Validate a circuit without executing it

```bash
cargo run -- check examples/product.zk
```

### Inspect include dependencies

```bash
cargo run -- deps examples/includes/main.zk
```

### Resolve imports and includes into a flattened source view

```bash
cargo run -- resolve examples/imports/main.zk
```

### Emit the lowered IR as JSON

```bash
cargo run -- compile-json examples/product.zk
```

### Analyze circuit size and shape

```bash
cargo run -- analyze examples/product.zk
```

### List available built-ins

```bash
cargo run -- list-builtins
```

### List standard-library modules

```bash
cargo run -- list-stdlib
```

### Verify lowered IR invariants

```bash
cargo run -- verify-ir examples/product.zk
```

### Optimize a circuit

```bash
cargo run -- optimize examples/optimize.zk
```

### Trace witness execution

```bash
cargo run -- trace examples/product.zk --public x=5 --private y=7
```

### Emit witness data as JSON

```bash
cargo run -- witness-json examples/product.zk --public x=5 --private y=7
```

### Run a circuit with witness values

```bash
cargo run -- run examples/product.zk --public x=5 --private y=7
```

### Run a circuit using `@std` includes

```bash
cargo run -- run examples/stdlib_demo.zk --public expected=24 --public primary=true --public secondary=true --private x=2 --private y=3 --private z=4
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

## Function Example

```zk
circuit functions_demo {
    fn sqr(value: field) -> field {
        value * value
    }

    fn transform(value: field, offset: field) -> field {
        sqr(value) + offset
    }

    public expected: field;
    private x: field;

    let result = transform(x, 7);
    constrain result == expected;
    expose result;
}
```

## Boolean Example

```zk
circuit booleans_demo {
    public expected: field;
    public gate: bool;
    private x: field;
    private y: field;

    let selected = if gate { x + 5 } else { y + 1 };
    constrain selected == expected;
    expose selected;
    expose gate as gate_value;
}
```

## Include Example

```zk
circuit include_demo {
    include "lib/math.zk";
    include "lib/logic.zk";

    public expected: field;
    public gate: bool;
    private x: field;
    private y: field;

    let base = shifted_square(x);
    let fallback = adjusted(y);
    let result = choose_bonus(gate, base, fallback);

    constrain result == expected;
    expose result;
}
```

## Standard Library Example

```zk
circuit stdlib_demo {
    include "@std/patterns/pipeline.zk";

    public expected: field;
    public primary: bool;
    public secondary: bool;
    private x: field;
    private y: field;
    private z: field;

    let result = pipeline_route(primary, secondary, x, y, z);
    constrain result == expected;
    expose result;
}
```

## Import Example

```zk
circuit imports_demo {
    import "modules/left.zk" as left;
    import "modules/right.zk" as right;

    public expected: field;
    public gate: bool;
    private x: field;
    private y: field;

    let left_score = left::score(x, gate);
    let right_score = right::score(y, gate);
    let result = left_score + right_score;

    constrain result == expected;
    expose result;
}
```

## Language Sketch

```text
program        := "circuit" IDENT "{" item* "}"
item           := input_decl | function_decl | stmt
               | include_decl | import_decl
include_decl   := "include" STRING ";"
import_decl    := "import" STRING "as" IDENT ";"
input_decl     := ("public" | "private") IDENT ":" type ";"
function_decl  := "fn" IDENT "(" params? ")" "->" type "{" expr "}"
params         := param ("," param)*
param          := IDENT ":" type
type           := "field" | "bool"
stmt           := let_stmt | constrain_stmt | expose_stmt
let_stmt       := "let" IDENT "=" expr ";"
constrain_stmt := "constrain" expr "==" expr ";"
expose_stmt    := "expose" expr ("as" IDENT)? ";"
expr           := term (("+" | "-") term)*
term           := unary ("*" unary)*
unary          := "-" unary | primary
primary        := NUMBER | "true" | "false" | IDENT | call | if_expr | "(" expr ")"
if_expr        := "if" expr "{" expr "}" "else" "{" expr "}"
call           := callee "(" args? ")"
callee         := IDENT ("::" IDENT)*
args           := expr ("," expr)*
```

### Semantics

- `field` values live in the prime field with modulus `18446744073709551557`.
- `bool` values lower to field elements constrained to `0` or `1`.
- Public and private inputs are assigned to wires in the lowered circuit.
- `include` directives are resolved relative to the including file, with `@std/...` mapped to the repository `stdlib/` tree.
- `import "path" as alias;` loads the target as a namespaced module rooted at `alias::...`.
- imported modules may only contain functions, includes, and further imports.
- already-expanded include files are skipped on subsequent includes, while cycles are still rejected.
- Functions are pure expression-bodied helpers and currently do not capture circuit bindings.
- Boolean built-ins expand before lowering and remain part of the typed IR.
- `let` bindings name intermediate expressions.
- `constrain` emits equality checks over lowered operands.
- `expose` emits named public outputs.

## Compiler Pipeline

```text
source
  -> lexer
  -> parser
  -> typed HIR
  -> arithmetic circuit IR
  -> analysis / serialization
  -> backend execution / constraint checking
```

Key files:

- `src/main.rs`: CLI entrypoint
- `src/lexer.rs`: lexer
- `src/parser.rs`: parser
- `src/source.rs`: include/import resolution and dependency graph generation
- `src/pretty.rs`: flattened source rendering for resolved programs
- `stdlib/`: experimental standard-library fragments built on the same include mechanism
- `src/typecheck.rs`: name resolution and typed HIR construction
- `src/hir.rs`: typed high-level IR
- `src/ir.rs`: arithmetic circuit IR and lowering
- `src/analysis.rs`: circuit metrics and reporting
- `src/optimize.rs`: simplification and dead code elimination
- `src/serialize.rs`: JSON artifact serialization
- `src/trace.rs`: witness tracing and execution artifacts
- `src/verify.rs`: lowered IR invariant checks
- `src/backend/`: backend boundary and interpreter backend
- `src/eval.rs`: backend-facing execution API

## CLI Commands

```text
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
zkc run <file.zk> [--public name=value]... [--private name=value]...
```

## Included Examples

- `examples/product.zk`: basic multiplication plus constraint
- `examples/balance.zk`: linear combination with offset
- `examples/builtins.zk`: built-in arithmetic helper catalog in use
- `examples/booleans.zk`: boolean inputs and `if` expressions
- `examples/cubic.zk`: cubic expression constraint
- `examples/functions.zk`: pure helper functions and call expressions
- `examples/imports/main.zk`: namespaced imports layered over stdlib includes
- `examples/includes/main.zk`: multi-file circuit entrypoint using nested includes
- `examples/logic.zk`: boolean built-ins over `bool` values
- `examples/optimize.zk`: simplifiable circuit for optimizer demos
- `examples/stdlib_bools.zk`: boolean stdlib fragments loaded through `@std`
- `examples/stdlib_demo.zk`: field pipeline assembled from `@std` modules
- `examples/trace.zk`: witness tracing demo circuit
- `examples/weighted_sum.zk`: weighted linear aggregation

## Limitations

- no proof generation
- no verification backend
- only scalar data types: `field` and `bool`
- no modules, arrays, or structs
- no package manifest, visibility control, or typed module interfaces
- no package manager
- standard library is experimental and source-only
- no proof-aware optimization beyond arithmetic simplification and DCE

## Development Direction

The next major steps are:

- extend the typed intermediate representation with richer values
- add typed modules, arrays, structs, and richer language features
- target a real backend such as `R1CS`, `AIR`, or `PLONKish`
- implement `keygen`, `prove`, and `verify`
- improve diagnostics, fuzzing, benchmarks, and compatibility tests

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
