---
id: ADR-0002
title: Use Rust for the standalone CLI
date: 2026-11-09
status: accepted
---

# ADR-0002: Use Rust for the standalone CLI

## Context

Makespec needs a cross-platform executable that works locally without a service. Users should be able to install a prebuilt binary without installing a language runtime.

## Decision

Implement Makespec as one Rust Cargo binary. Keep dependencies locked in Cargo.lock and declare the minimum supported Rust version in Cargo.toml. Build native release archives for the supported operating systems and CPU targets.

## Consequences

Users can run the distributed executable without a Rust toolchain. Contributors need Rust and must maintain compatibility with the declared minimum version.

Rust provides typed document and lifecycle models, but the language alone does not establish memory or latency budgets. Measure those properties with the catalog benchmark. Build coverage and runtime test coverage remain separate responsibilities.

## References

- [Cargo manifest](../../Cargo.toml) defines the binary, dependencies, and minimum Rust version.
- [Contribution guide](../../CONTRIBUTING.md) defines build checks, benchmarks, and release procedures.
- [Release workflow](../../.github/workflows/release.yml) defines native archive targets.