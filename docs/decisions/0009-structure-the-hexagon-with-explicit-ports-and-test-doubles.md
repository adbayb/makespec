---
id: ADR-0009
title: Structure the hexagon with explicit ports and test doubles
date: 2026-11-28
status: proposed
---

# ADR-0009: Structure the hexagon with explicit ports and test doubles

## Context

ADR-0006 keeps lightweight hexagonal boundaries: one `ProjectStore` trait shared by all use cases, a service module holding every workflow, and a CLI module that both parses arguments and wires the filesystem adapter. Hexagon tests reach the use cases only through the real filesystem adapter, so a workflow rule cannot be tested without temporary directories. The CLI module mixes terminal input and output with composition, and the embedded skill files reach initialization as an untyped slice of path and content pairs.

Contributors follow the `software-design` skill, whose hexagonal guidance names ports by purpose, splits them into driver and driven sides, groups adapters by technology with a test double per port, and gives each driver technology its own startup entry.

## Decision

Structure `src/` as follows, still within one Cargo binary:

- `src/hexagon/` holds document rules (`document.rs`), errors, one file per use case (`initialize_project.rs`, `create_document.rs`, `get_summaries.rs`, `change_document_status.rs`, `check_documents.rs`), and `makespec.rs`, which implements the driver port on top of constructor-injected driven ports.
- `src/hexagon/ports/driver/for_managing_documents.rs` defines the `ForManagingDocuments` driver port and its input types.
- `src/hexagon/ports/driven/for_storing_documents.rs` defines the `ForStoringDocuments` port, and `for_obtaining_skill_files.rs` defines the `ForObtainingSkillFiles` port.
- `src/adapters/<tech>/<port>.rs` holds one adapter per port and role: `cli` drives the hexagon from the terminal, `filesystem` stores documents, `embedded` supplies the compiled skill, and test-only `fake` adapters keep documents in memory and supply a stub skill.
- `src/startup/cli.rs` instantiates the driven adapters, the hexagon, and the CLI driver adapter, then runs it. The test-only `src/startup/test.rs` wires the fakes for use case tests.

The CLI driver adapter reads the clock and passes the operation date into use cases. The hexagon imports no adapter outside test code. Add no public library, dependency injection framework, or ports for capabilities that current workflows do not use.

## Consequences

Use case tests drive the hexagon through its driver port with in-memory doubles, while filesystem tests focus on the adapter contract: locking, counters, path safety, discovery, and conflict detection. Each port and adapter has one file, so a contributor can locate a boundary by its purpose.

The layout adds module files and a second implementation of the storage port that contributors must keep consistent with the filesystem adapter's observable behavior. The in-memory fake does not model locking or cached counters, so tests of those rules stay in the filesystem adapter. The CLI driver adapter now opens the project store before parsing arguments, so `--help` also requires a readable working directory.

## References

- [ADR-0006: Keep hexagonal boundaries within one binary](0006-keep-hexagonal-boundaries-within-one-binary.md) is the decision this record would supersede once accepted.
- [Hexagon wiring](../../src/hexagon/makespec.rs) and [production startup](../../src/startup/cli.rs) implement the layout.
- [Hexagonal architecture explained](https://jmgarridopaz.github.io/content/hexagonalarchitecture-ig/intro.html) describes the driver and driven port vocabulary.