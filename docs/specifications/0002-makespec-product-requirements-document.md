---
id: RFC-0002
title: Makespec product requirements document (PRD)
date: 2026-11-09
status: accepted
---

# RFC-0002: Makespec product requirements document (PRD)

## Context

Makespec is a cross-platform Rust CLI for focused architecture decisions (ADRs) and functionality proposals (RFCs). Developers and coding agents keep records in Git as Markdown without a service, database, or project configuration.

The implementation already exists. Keep its product and implementation contracts in a managed RFC, following ADR-0001 and RFC-0001, so reviewers and planning agents can cite a dated, numbered specification. This proposal changes documentation organization, not runtime behavior.

The command reference, templates, agent skill, and contribution guide define their respective contracts. This RFC covers the remaining storage, validation, safety, and implementation invariants. A TUI, editor launcher, embedded LLM providers, and a public library are outside scope.

## Requirements and constraints

### Requirements

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-001 | MUST | Manage dated ADRs and RFCs as Markdown records with independent incrementing IDs in conventional directories. |
| FR-002 | MUST | Initialize accepted adoption and format records plus the canonical agent skill without overwriting conflicting project content. |
| FR-003 | MUST | Validate records at the depth required by their status and reject invalid promotions before changing files. |
| FR-004 | MUST | Preserve record identity, date, path, body bytes, and unrelated YAML fields during status changes. |
| FR-005 | MUST | Coordinate ID allocation and file publication between concurrent Makespec processes. |
| NFR-001 | MUST | Bound managed input sizes and list metadata without loading document bodies. |
| NFR-002 | MUST | Keep document rules and use cases separate from direct filesystem and terminal access. |
| NFR-003 | MUST | Give the product contract a unique RFC ID, creation date, lifecycle status, and canonical RFC structure. |
| NFR-004 | SHOULD | Maintain Linux, macOS, and Windows CI checks and native release archives with checksums. |

### Constraints

| ID | Constraint |
| --- | --- |
| C-001 | Keep one Rust Cargo binary with no required service, database, or project configuration. |
| C-002 | Make no network or LLM calls and provide no content execution, telemetry, plugins, or automatic updates. |
| C-003 | Preserve the existing command and document contracts during this documentation change. |

## Options

- No change: retain the standalone docs/spec.md product specification and the existing implementation.
- Managed product RFC: replace that file with a record under docs/specifications/, preserving its contracts and linking the related guides and design decisions.

Both options retain the current runtime behavior. The comparison concerns whether maintainers track the product contract as an RFC.

| Criterion ID | No change | Managed product RFC |
| --- | --- | --- |
| FR-001 | Fully meets - Existing record creation and directory rules remain unchanged. | Fully meets - Design retains the existing identity and directory rules. |
| FR-002 | Fully meets - Existing initialization behavior remains unchanged. | Fully meets - Design retains seed creation, skill installation, and conflict handling. |
| FR-003 | Fully meets - Existing validators and promotion checks remain unchanged. | Fully meets - Design retains status-based validation and failed-promotion guarantees. |
| FR-004 | Fully meets - Existing metadata-update behavior remains unchanged. | Fully meets - Design retains the same immutable fields and byte-preservation rules. |
| FR-005 | Fully meets - Existing filesystem locking and publication remain unchanged. | Fully meets - Design retains locked allocation and staged publication. |
| NFR-001 | Fully meets - Existing read limits and metadata-only listing remain unchanged. | Fully meets - Design retains the exact limits and catalog behavior. |
| NFR-002 | Fully meets - Existing core, adapter, and startup boundaries remain unchanged. | Fully meets - Design retains those boundaries and their test seams. |
| NFR-003 | Does not meet - The standalone file has no managed RFC identity, date, status, or comparison. | Fully meets - The CLI supplies metadata and validates the canonical sections and comparison. |
| NFR-004 | Fully meets - Existing CI and release workflows remain unchanged. | Fully meets - Design retains CI coverage, archives, and checksum requirements. |
| C-001 | PASS - The current Rust binary needs no service, database, or project configuration. | PASS - The change adds documentation records without runtime dependencies. |
| C-002 | PASS - The current offline runtime remains unchanged. | PASS - The change adds no network, execution, or extension features. |
| C-003 | PASS - No command or document contract changes. | PASS - Only the product specification's format, location, and inbound links change. |

**Chosen option:** Managed product RFC

The managed RFC satisfies NFR-003 while preserving FR-001 through FR-005 and C-001 through C-003. The existing document rules, filesystem adapter, CLI tests, and CI workflows provide the implementation evidence for the retained behavior.

Maintainers must keep the requirement table, comparison, and design aligned when the product contract changes. Structural checks help detect malformed records but do not establish that the prose matches the implementation.

## Design

### Records and identity

Store ADRs in `docs/decisions/` and RFCs in `docs/specifications/`. Each kind has its own positive `u32` sequence, starting at `0001` and expanding beyond four digits without extra leading zeros. Use IDs such as `ADR-0001` and filenames such as `0001-use-local-storage.md`.

The CLI adds YAML front matter containing `id`, `title`, `date` (`YYYY-MM-DD`), and `status`, plus a matching `# <ID>: <Title>` heading. Derive kind from the ID prefix. Keep status and date only in front matter; do not duplicate `kind`, `slug`, or `schema_version`.

Generate ASCII kebab-case slugs with `deunicode` transliteration. Creation titles must contain 1 to 200 bytes after trimming, contain no control characters or path separators, and produce a nonempty slug. Later title edits do not rename the file. CLI status updates preserve path, ID, date, body bytes, and unrelated YAML fields.

Allocate IDs under a project-wide advisory lock. Reconcile cached per-kind high-water marks against existing IDs and persist the next number before publishing a record. Failed writes may leave gaps. Store locks and counter state in the user cache, keyed by the canonical project path. Disconnected branches can allocate the same ID; `check` reports merged collisions.

### Initialization

`init` operates in the current directory, creating the two document directories and accepted seeds using the initialization date:

- `ADR-0001` records adoption of ADRs for decisions and RFCs for functionality.
- `RFC-0001` specifies the RFC format with complete requirements, constraints, and alternative comparisons.

Embed the canonical skill files at build time and install them under `.agents/skills/makespec/`. Install no scripts or agent configuration. Skill discovery depends on agent support, and users must put `makespec` on the agent's `PATH`.

Preflight managed destinations, stage files beside their destinations, and publish without overwriting existing files. Reinitialization preserves matching files and valid accepted seeds; conflicts fail with a path. If publication fails, remove only newly published files whose contents still match, reporting rollback failures. Initialization is not a multi-file transaction; rerunning it can complete a partial initialization. It does not initialize Git or commit files.

### Validation and updates

Parse front matter with `serde_yaml_ng` and headings/tables with `pulldown-cmark`. Validation checks metadata, matching title headings, ordered unique sections, canonical tables, criterion references, assessments, and option eligibility. It cannot judge prose quality or verify evidence.

Choose validation depth from the document status. Before writing a promotion to proposed or accepted, validate the prospective document with its new status. Failed readiness checks and invalid transitions leave the original file unchanged.

Supersession records `superseded_by` only on the old record. Require a different, accepted replacement of the same kind when applying the transition. Historical targets may later become deprecated or superseded; `check` rejects missing or invalid targets and cycles.

Use normal workspace tools for body edits. Before replacing a record, compare its current bytes with the loaded version under the lock, stage the replacement, and check again before publication. Advisory locks coordinate Makespec processes, not external editors. `check` cannot establish historical immutability without Git history.

### Implementation boundaries

Keep one Cargo binary and the existing hexagonal boundaries:

- `src/core/document.rs` owns document construction, parsing, and validation.
- `src/core/service.rs` owns use cases and the `ProjectStore` port.
- `src/adapters/` implements filesystem access and supplies embedded skill assets.
- `src/startup/cli.rs` composes adapters and handles terminal input/output.

Pass the operation date into use cases. Keep filesystem and terminal effects outside document rules; test pure rules without I/O and storage workflows with temporary directories. Add no public library, provider interface, generic repository framework, or speculative abstraction.

### Safety and distribution

- Run commands from the project root. With no command, print help without reading stdin. Commands do not prompt or open an editor; `new` requires piped UTF-8 input.
- Limit stdin and complete documents to 2 MiB, catalog front matter to 64 KiB, and cached counter state to 1 KiB. Listing reads metadata without loading document bodies.
- Reject traversal, symlinked managed paths, and unexpected files in document directories; ignore only regular `.makespec-*.tmp` staging files. Use bounded reads, exclusive publication, and same-directory staged replacements. Escape control characters in terminal output.
- Treat record contents as text. Make no network or LLM calls and provide no content execution, telemetry, plugin loading, or automatic updates.
- Keep dependencies locked in `Cargo.lock`. CI checks Linux, macOS, and Windows; tagged releases publish native archives and SHA-256 checksums. Distinguish release build coverage from runtime testing.

### Risks and mitigations

The specification and related ADRs can drift from implementation. Review contract changes alongside the source and tests, and consult each record's status before treating a proposal as an accepted commitment.

The CLI cannot prove the truth of comparison evidence or recover forgotten allocations after external cache deletion. Reviewers must examine the referenced implementation and account for the local scope of ID allocation.

## Adoption

Initialize the repository with Makespec to establish the accepted adoption ADR and format RFC. Create the product RFC with the CLI, preserve the existing specification's contracts, and update README and contribution links to its assigned path before removing the standalone file.

Record the current implementation choices in focused ADRs. Keep newly authored records proposed until the user requests acceptance; documenting existing behavior does not imply a historical approval.

Run `makespec check`, verify relative links, and run the contribution checks. Existing unit and CLI tests cover record rules, initialization, concurrent allocation, status changes, and bounded discovery. No binary behavior or project-data conversion is required. Git can restore the documentation move without a runtime rollback.

## References

- [ADR-0001: Use ADRs for decisions and RFCs for changes](../decisions/0001-use-adrs-for-decisions-and-rfcs-for-changes.md) adopts the documentation workflow.
- [RFC-0001: Specify the RFC format](0001-specify-the-rfc-format.md) defines record structure, lifecycle, and comparison rules.
- [ADR-0002: Use Rust for the standalone CLI](../decisions/0002-use-rust-for-the-standalone-cli.md) records the language and distribution choice.
- [ADR-0003: Keep the CLI offline and agent-driven](../decisions/0003-keep-the-cli-offline-and-agent-driven.md) records interaction and provider boundaries.
- [ADR-0004: Use Markdown records as the source of truth](../decisions/0004-use-markdown-records-as-the-source-of-truth.md) records document storage and metadata ownership.
- [ADR-0005: Use fixed content-oriented document directories](../decisions/0005-use-fixed-content-oriented-document-directories.md) records the layout and configuration boundary.
- [ADR-0006: Keep hexagonal boundaries within one binary](../decisions/0006-keep-hexagonal-boundaries-within-one-binary.md) records module responsibilities and dependency direction.
- [ADR-0007: Allocate IDs with locked external counters](../decisions/0007-allocate-ids-with-locked-external-counters.md) records allocation state and its limits.
- [ADR-0008: Stage managed writes and detect conflicts](../decisions/0008-stage-managed-writes-and-detect-conflicts.md) records publication and concurrent-edit safeguards.
- [README](../../README.md) defines installation and quick start.
- [Command reference](../../.agents/skills/makespec/references/cli.md) defines syntax, lifecycle transitions, and completion rules.
- [ADR template](../../.agents/skills/makespec/references/adr-template.md) and [RFC template](../../.agents/skills/makespec/references/rfc-template.md) define body structure and comparison grammar.
- [Makespec skill](../../.agents/skills/makespec/SKILL.md) defines authoring, planning, and approval boundaries.
- [Contribution guide](../../CONTRIBUTING.md) defines development checks, benchmarks, and releases.
- [Document rules](../../src/core/document.rs), [core workflows](../../src/core/service.rs), and [filesystem adapter](../../src/adapters/filesystem.rs) implement the retained contracts.
- [CLI tests](../../tests/cli.rs) and [CI workflow](../../.github/workflows/ci.yml) provide executable checks.