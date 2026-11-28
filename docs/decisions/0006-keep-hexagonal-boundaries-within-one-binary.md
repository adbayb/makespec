---
id: ADR-0006
title: Keep hexagonal boundaries within one binary
date: 2026-11-09
status: accepted
---

# ADR-0006: Keep hexagonal boundaries within one binary

## Context

Document rules and lifecycle policies need tests that do not depend on terminal input or a particular filesystem implementation. CLI parsing and safe file publication have different reasons to change.

## Decision

Keep lightweight hexagonal boundaries within the single Cargo binary:

- src/hexagon/document.rs owns document construction, parsing, and validation.
- src/hexagon/service.rs owns use cases.
- src/hexagon/ports.rs owns the ProjectStore port.
- src/adapters/ implements filesystem access and supplies embedded skill assets.
- src/startup/cli.rs composes the adapter and handles terminal input/output.

Read the clock in src/startup/cli.rs and pass the operation date into use cases. Keep direct filesystem and terminal access in adapters. Add no public library, provider interface, generic repository framework, or speculative abstraction.

## Consequences

Document rules can run in tests without filesystem or terminal setup. Storage tests can exercise the same use cases with temporary directories.

The storage port adds an internal boundary that contributors must preserve. Keep it limited to current workflows rather than expanding it for hypothetical adapters.

## References

- [Contributor instructions](../../AGENTS.md) define repository coding and check requirements.
- [Contribution conventions](../../CONTRIBUTING.md) require the existing hexagonal boundaries.
- [Storage port](../../src/hexagon/ports.rs) defines the dependency boundary used by the [workflows](../../src/hexagon/service.rs).