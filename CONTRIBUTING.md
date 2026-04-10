# Contributing

## Development Workflow

1. Create a feature branch from `main`.
2. Make focused changes with matching tests and documentation updates.
3. Run the local verification steps before opening a pull request.
4. Open a pull request with a clear problem statement and validation notes.

## Local Verification

Run these commands before pushing:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run -- compile examples/product.zk
cargo run -- run examples/product.zk --public x=5 --private y=7
```

## Coding Guidelines

- Keep the language surface small unless a feature unlocks a clear next step.
- Prefer explicit compiler stages over implicit coupling.
- Treat diagnostics as part of the public developer experience.
- Avoid unchecked complexity in the source language until the IR model is stable.

## Documentation Expectations

Update the relevant docs when you change:

- source syntax
- lowering behavior
- runtime input behavior
- release or security process

## Pull Requests

Each pull request should explain:

- what changed
- why it changed
- how it was validated
- any compatibility or soundness risks
