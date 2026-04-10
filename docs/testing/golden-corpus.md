# Golden Corpus

The repository includes a golden corpus under `tests/golden/`.

## Purpose

The golden corpus serves two roles:

- regression coverage for accepted programs
- regression coverage for expected compiler errors

## Layout

- `tests/golden/ok/`: programs that should compile, and optionally run
- `tests/golden/err/`: programs that should fail with a matching error substring

## Directive Format

Supported directives are written as comments at the top of a `.zk` file:

- `# RUN: ...`
- `# EXPECT-OUTPUT: name=value`
- `# EXPECT-ERROR: substring`

## Example

```text
# RUN: --public x=5 --private y=7
# EXPECT-OUTPUT: product=35
# EXPECT-OUTPUT: shifted_value=38
```

The test harness compiles the file, optionally executes it with the supplied witness values, and
checks that the expected outputs or error message are observed.
