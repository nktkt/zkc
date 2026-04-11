# Constraint IR

`zkc` now exposes a symbolic equation layer through `src/constraint.rs`.

## Purpose

The current prover backend story is still incomplete, but future proving systems need something
more stable than a raw wire-operation listing. The constraint IR provides that intermediate view.

Today it is derived from the arithmetic circuit IR and models the circuit as:

- public input variables
- private input variables
- witness variables
- definition equations
- assertion equations
- range assertions
- named outputs

## CLI

Print the symbolic view:

```bash
cargo run -- constraints examples/product.zk
```

Emit JSON:

```bash
cargo run -- constraints examples/product.zk --json
```

## Shape

A simple circuit like:

```zk
circuit product_check {
    public x: field;
    private y: field;

    let product = x * y;
    let shifted = product + 3;
    constrain shifted == 38;
    expose product;
}
```

becomes equations in the form:

```text
[def] v2 == v0 * v1
[def] v3 == v2 + 3
[assert] v3 == 38
```

## Current Limits

- this is still derived from the lowered arithmetic circuit IR, not yet the primary backend target
- it does not yet encode lookup arguments, range checks, or prover-specific gates
- boolean values are already algebraized before they appear here
- no proving backend consumes it yet
