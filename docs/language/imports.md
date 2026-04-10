# Imports

`zkc` supports source-level module composition through namespaced `import` directives.

## Syntax

```zk
circuit main {
    import "modules/left.zk" as left;
    import "modules/right.zk" as right;

    public expected: field;
    public gate: bool;
    private x: field;
    private y: field;

    let result = left::score(x, gate) + right::score(y, gate);
    constrain result == expected;
    expose result;
}
```

Imported modules can be called with qualified names such as `left::score(...)`.

## Module Rules

Imported files are item fragments, not full `circuit` declarations.

An imported module may contain:

- function declarations
- `include` directives
- nested `import` directives

An imported module may not contain:

- public inputs
- private inputs
- top-level `let`, `constrain`, or `expose` statements

This restriction keeps module loading side-effect free. Imported files only contribute helper
functions into a namespace.

## Namespacing

- `import "path.zk" as left;` maps exported function names under the `left::` prefix
- nested imports extend the namespace, so `import "@std/select/field.zk" as select;` inside `left`
  becomes `left::select::...`
- includes inside an imported module inherit that module namespace
- the same file may be imported more than once under different aliases; each alias gets its own
  namespace

## Resolution

- import paths use the same path resolution rules as `include`
- relative paths resolve against the importing file
- `@std/...` resolves into the repository `stdlib/` tree
- import resolution runs before typechecking
- cycles are rejected with the same path-stack checks used for includes

## Dependency Graphs

The dependency graph printed by `deps` includes both includes and imports.

```bash
cargo run -- deps examples/imports/main.zk
```

Edges are annotated with:

- edge kind: `include` or `import`
- namespace: the namespace active when the edge was expanded

JSON output preserves the same metadata:

```bash
cargo run -- deps examples/imports/main.zk --json
```

## Flattened View

`resolve` renders a flattened, resolved view of the program after includes and imports have been
expanded.

```bash
cargo run -- resolve examples/imports/main.zk
```

The rendered view is intended for inspection and debugging. It may include namespaced function
names such as `fn left::score(...)`, which are internal resolved names rather than original surface
syntax.
