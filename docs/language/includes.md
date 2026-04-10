# Includes

`zkc` supports file-based composition through `include` directives inside circuit bodies.

## Syntax

```zk
circuit main {
    include "lib/math.zk";
    include "lib/logic.zk";

    public expected: field;
    private x: field;

    let result = helper(x);
    constrain result == expected;
    expose result;
}
```

An included file is not a full circuit. It is an item fragment containing any mix of:

- `include` directives
- input declarations
- function declarations
- statements

## Path Resolution

- include paths are written as string literals
- relative paths are resolved against the including file's directory
- `@std/...` paths resolve into the repository `stdlib/` directory
- include resolution happens before typechecking
- included items are inserted at the include site in source order
- previously expanded files are skipped when included again

## Nested Includes

Included files may include other files:

```zk
include "../shared/offsets.zk";

fn shifted_square(value: field) -> field {
    value * value + unit_offset()
}
```

## Dependency Graphs

The CLI can print the resolved dependency graph:

```bash
cargo run -- deps examples/includes/main.zk
```

JSON output is also available:

```bash
cargo run -- deps examples/includes/main.zk --json
```

## Constraints

- cycle detection is enforced during include resolution
- missing include files fail before typechecking
- `compile_source` does not resolve includes because it has no filesystem root
- include files should not contain a top-level `circuit` declaration
- includes are source-only; namespacing is provided separately through `import`
