# Roadmap

## Phase 0: Scope and Specification

Status: in progress

- define the initial threat model
- define the initial target use case
- freeze the source-language specification for v0
- document current non-goals

## Phase 1: Compiler Hardening

Status: partially complete

- introduce a typed intermediate representation
- improve diagnostics and invariant checking
- extend tests beyond unit coverage
- add linting and CI gates

Implemented so far:

- basic parser and validator
- typed HIR construction
- arithmetic circuit IR lowering
- backend abstraction plus interpreter backend
- unit tests and CLI integration tests
- formatting and lint checks in CI

## Phase 2: Proof MVP

Status: not started

- choose one proving backend
- implement `keygen`
- implement `prove`
- implement `verify`
- define proof and public-input artifact formats

## Phase 3: Language Expansion

Status: partially complete

- richer types
- functions
- modules and imports
- collections and structured values
- standard library

Implemented so far:

- pure user-defined functions
- `bool` as a second scalar type
- boolean builtins
- expression-level conditionals
- file-based include composition
- experimental `@std` source library

## Phase 4: Quality and Tooling

Status: partially complete

- contributor documentation
- security policy
- release process
- CI and repository templates
- changelog discipline

Implemented so far:

- `README`
- `CONTRIBUTING`
- `SECURITY`
- `RELEASE`
- GitHub issue and PR templates
- GitHub Actions CI

## Phase 5: Productionization

Status: not started

Definition of done:

- real proving backend integrated
- artifact stability policy defined
- compatibility tests in place
- external security review completed
- release signing and provenance in place
- operational ownership for vulnerabilities and releases established

This repository is not yet at Phase 5. The current work prepares the project to move toward it
without falsely claiming that the proof system itself is production-ready.
