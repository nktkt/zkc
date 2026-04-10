# Standard Library

`zkc` now ships with an experimental source-level standard library under [`stdlib/`](../stdlib).
These modules are plain include fragments and are loaded with the same mechanism as user-authored
includes.

## Import Style

Use the `@std/...` prefix inside a circuit:

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

## Catalog

The current `stdlib/` tree is grouped into a few categories:

- `arith/`: constants, scaling helpers, accumulators, and polynomial helpers
- `bool/`: boolean helpers and higher-order boolean combinators
- `select/`: selector-oriented field and boolean routing helpers
- `patterns/`: higher-level circuit-shaped helpers built from lower-level modules

Use the CLI to inspect the installed modules:

```bash
cargo run -- list-stdlib
```

JSON output is available too:

```bash
cargo run -- list-stdlib --json
```

## Design Notes

- stdlib modules are source fragments, not compiled binary artifacts
- stdlib modules can include other stdlib modules
- repeated includes are deduplicated by resolved file path
- cycles are rejected during include resolution
- there is no namespace system yet, so function names still live in one circuit-level scope

## Current Modules

Representative modules include:

- `arith/constants.zk`
- `arith/scaling.zk`
- `arith/accumulators.zk`
- `arith/polynomials.zk`
- `bool/core.zk`
- `bool/advanced.zk`
- `select/field.zk`
- `select/bool.zk`
- `patterns/gated.zk`
- `patterns/blends.zk`
- `patterns/pipeline.zk`

## Current Limitations

- this is not a package manager
- there is no semantic versioning or artifact stability for stdlib modules
- stdlib modules are still source-level and experimental
- because there is no namespacing yet, user code must avoid colliding with imported function names
