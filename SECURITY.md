# Security Policy

## Supported Scope

This repository is an experimental compiler prototype.

Security-sensitive reports are still valuable, especially for:

- incorrect constraint lowering
- witness validation bugs
- parser crashes or panics on malformed input
- unsound behavior that could lead to incorrect acceptance or rejection

## Reporting a Vulnerability

Please do not open a public issue for undisclosed security problems.

Preferred path:

1. Use GitHub Security Advisories for a private report if available.
2. If private reporting is unavailable, contact the repository owner directly before public
   disclosure.

Include:

- affected version or commit
- minimal reproducer
- impact assessment
- whether the issue can lead to unsound constraint acceptance

## Security Notes

- `zkc` is not production-ready.
- No external audit has been completed.
- No proof-generation backend is currently implemented.
