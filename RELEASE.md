# Release Process

## Release Preconditions

- `main` is green in CI
- changelog entries are up to date
- version has been updated in `Cargo.toml`
- release notes summarize behavior changes and known limitations

## Verification Checklist

Run locally before tagging:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run -- compile examples/product.zk
cargo run -- run examples/product.zk --public x=5 --private y=7
```

## Tagging

Use annotated tags:

```bash
git tag -a v0.1.0 -m "zkc v0.1.0"
git push origin v0.1.0
```

## Release Notes

Each release should document:

- language changes
- IR changes
- CLI changes
- validation changes
- known limitations
- migration notes if behavior changed

## Production Reminder

Do not market a release as production-ready until:

- a real proving backend exists
- compatibility guarantees are documented
- a security review has been completed
