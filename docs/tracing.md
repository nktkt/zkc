# Witness Tracing

`zkc` can execute lowered circuits with the interpreter backend and emit a resolved witness trace.
This is not a proof artifact. It is a development-time debugging artifact that helps explain how a
particular witness assignment flowed through the circuit.

## Human-readable Trace

Use the `trace` command to print a formatted execution trace:

```bash
cargo run -- trace examples/product.zk --public x=5 --private y=7
```

Example output:

```text
execution trace for product_check via interpreter over field modulus 21888242871839275222246405745257275088548364400416034343698204186575808495617
public inputs:
  x = 5
private inputs:
  y = 7
operations:
  w2 = mul w0 (5) , w1 (7) => 35
  w3 = add w2 (35) , 3 (3) => 38
constraints:
  #0: w3 (38) == 38 (38) [ok]
outputs:
  product = w2 (35)
  shifted_value = w3 (38)
wires:
  w0 = 5
  w1 = 7
  w2 = 35
  w3 = 38
```

## JSON Artifact

Use `witness-json` when you want a structured artifact that can be stored, diffed, or consumed by
other tools:

```bash
cargo run -- witness-json examples/product.zk --public x=5 --private y=7
```

The emitted JSON contains:

- circuit metadata
- backend name
- public input assignments
- private input assignments
- final wire table
- resolved operations
- resolved constraints
- resolved outputs

The `trace` command can also emit JSON with `--json`:

```bash
cargo run -- trace examples/product.zk --json --public x=5 --private y=7
```

## Intended Use

Witness tracing is useful for:

- debugging arithmetic mistakes in a circuit
- validating optimizer output against concrete inputs
- generating regression artifacts for example circuits
- understanding wire allocation and output naming

Witness tracing is not a substitute for:

- proof generation
- proof verification
- soundness arguments
- audited artifact formats

## Notes

- Constraint failures stop execution and return a runtime error.
- JSON output is currently designed for repository tooling, not long-term stability.
- The only available backend today is the interpreter backend.
