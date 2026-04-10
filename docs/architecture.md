# Architecture

## Overview

`zkc` is structured as a small compiler pipeline with explicit stages:

```text
source
  -> lexer
  -> parser
  -> AST
  -> typed HIR
  -> arithmetic circuit IR
  -> analysis / serialization
  -> backend execution
```

## Components

### Lexer

`src/lexer.rs`

Responsible for tokenizing the source stream into keywords, punctuation, identifiers, numbers, and
end-of-file markers. The lexer also strips supported comment forms.

### Parser

`src/parser.rs`

Responsible for building the AST and enforcing surface syntax rules such as precedence, statement
structure, include/import directives, and the single-circuit program shape.

### AST

`src/ast.rs`

Represents the source-level structure of circuits, declarations, statements, and expressions.

### Source Resolution

`src/source.rs`

This stage resolves filesystem-backed `include` and `import` directives before typechecking.

Current responsibilities:

- relative path resolution
- virtual `@std` path resolution
- nested include expansion
- namespaced import expansion
- include-once expansion for already-resolved files
- circular include detection
- dependency graph generation for CLI tooling
- resolved source rendering for debugging

### Typechecking and Typed HIR

`src/typecheck.rs`

This stage resolves bindings and builds a typed high-level IR. At the moment the language still has
two concrete scalar types, and the compiler now has an explicit place to represent typed
expressions and declarations before backend lowering.

Current responsibilities:

- declaration ordering
- duplicate detection
- use-before-declaration rejection
- typed expression construction
- scalar typechecking for `field` and `bool`
- function-call validation and inlining
- conditional typing and branch compatibility checks

The resulting IR is defined in `src/hir.rs`.

### Lowering

`src/ir.rs`

Lowers validated AST nodes into a wire-based arithmetic circuit representation. This is the boundary
between source semantics and backend-ready circuit semantics.

Current lowering behavior also handles:

- boolean input constraints
- boolean builtins through arithmetic encodings
- `if` expressions via selector-style arithmetic combinations

### Witness Execution

`src/backend/` and `src/eval.rs`

The repository now has an explicit backend boundary. The current backend is an interpreter that
executes the lowered circuit given explicit public and private inputs, checks all constraints, and
returns named public outputs.

### Analysis and Serialization

`src/analysis.rs` and `src/serialize.rs`

These modules sit alongside backend execution and provide non-execution views of the lowered
circuit:

- human-readable circuit metrics
- JSON circuit artifacts
- command-line reporting hooks for repository tooling and future integrations

### Witness Tracing

`src/trace.rs`

This module builds a fully resolved interpreter trace:

- named public and private input assignments
- per-operation operand values and output wires
- evaluated constraints with pass/fail status
- final exposed outputs
- full wire table snapshots suitable for JSON artifacts

### Verification and Optimization

`src/verify.rs` and `src/optimize.rs`

These modules operate on lowered arithmetic circuits:

- `verify.rs` checks structural invariants such as wire ordering and definition-before-use
- `optimize.rs` applies simplification, dead code elimination, and wire compaction

### CLI

`src/main.rs`

Provides user-facing commands:

- `list-builtins`
- `list-stdlib`
- `check`
- `deps`
- `resolve`
- `verify-ir`
- `compile`
- `compile-json`
- `analyze`
- `optimize`
- `trace`
- `witness-json`
- `run`

## Security Posture

The current repository is a compiler prototype, not a production proof system. It has the following
important constraints:

- no prover backend
- no verifier backend
- no audited soundness proof
- no compatibility guarantees for IR or CLI output

## Evolution Path

The intended architecture for the next major version is:

```text
source
  -> lexer
  -> parser
  -> AST
  -> typed HIR
  -> constraint IR
  -> backend IR
  -> keygen / prove / verify
```

That future architecture is expected to separate:

- source-level semantics
- type and effect validation
- circuit-shaping transforms
- backend-specific proving logic
