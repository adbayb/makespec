---
id: RFC-0001
title: Makespec product requirements document (PRD)
date: 2026-11-09
status: accepted
---

# RFC-0001: Makespec product requirements document (PRD)

## Context

Developers and coding agents need to propose functionality, record focused choices, and find the accepted requirements and decisions that constrain later work. Markdown and Git support authoring and review, but contributors must otherwise coordinate numbering, metadata, lifecycle changes, and completeness checks by hand.

Makespec provides a cross-platform Rust CLI for these record-management tasks. Decisions follow the architecture decision record (ADR) format; specifications follow the request for comments (RFC) format and include product requirements documents (PRDs). Records remain readable and editable with ordinary tools, without a service, database, or project configuration. ADR-0001 adopts Makespec; ADR-0003 establishes the offline, agent-driven interaction model.

Success means a developer or agent can initialize a repository, create and find a dated record, validate a complete proposal, and change its status without changing its identity or body. Invalid promotions and conflicting writes must fail explicitly. Authors and reviewers remain responsible for the content and its approval.

Scope covers local creation, discovery, validation, lifecycle management, safe storage, agent guidance, and binary distribution. A TUI, editor launcher, embedded LLM providers, and a public library are outside scope. The command reference owns syntax and lifecycle details, templates own record structure, the agent skill owns contribution and approval guidance, and the contribution guide owns development and release procedures.

## Requirements and constraints

### Requirements

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-001 | MUST | Manage dated decisions and specifications as Markdown records with independent incrementing IDs in conventional directories. |
| FR-002 | MUST | Initialize both document directories, one accepted Makespec adoption decision, and the canonical agent skill without generating a specification or overwriting conflicting project content. |
| FR-003 | MUST | Validate records at the depth required by their status and reject invalid promotions before changing files. |
| FR-004 | MUST | Preserve record identity, date, path, body bytes, and unrelated YAML fields during status changes. |
| FR-005 | MUST | Coordinate ID allocation and file publication between concurrent Makespec processes. |
| FR-006 | MUST | Create draft records from a title and body-only UTF-8 stdin, adding metadata and the title heading without interactive prompts. |
| FR-007 | MUST | List all managed records with their IDs, statuses, and paths so users and agents can find them. |
| FR-008 | MUST | Apply the documented lifecycle through explicit status commands, including supersession by a different accepted record of the same kind. |
| NFR-001 | MUST | Bound managed input sizes and list metadata without loading document bodies. |
| NFR-002 | MUST | Keep document rules and use cases separate from direct filesystem and terminal access. |
| NFR-004 | SHOULD | Maintain Linux, macOS, and Windows CI checks and native release archives with checksums. |

### Constraints

| ID | Constraint |
| --- | --- |
| C-001 | Keep one Rust Cargo binary with no required service, database, or project configuration. |
| C-002 | Make no network or LLM calls and provide no content execution, telemetry, plugins, or automatic updates. |

## Options

This specification compares two ways to manage project records: a template-driven Markdown workflow, where contributors use shared templates and Git conventions, and Makespec CLI, which adds local automation. Assessments concern capabilities and safeguards supplied by each workflow, rather than what contributors could achieve through additional manual care or automation.

- Template-driven Markdown: use shared templates and contributor guidance to author records, assign IDs, review completeness, and change statuses manually in Git.
- Makespec CLI: use the local Rust binary for creation, listing, validation, and lifecycle changes, with the installed skill guiding agents. Keep prose authoring and review in existing workspace tools.

| Criterion ID | Template-driven Markdown | Makespec CLI |
| --- | --- | --- |
| FR-001 | Partially meets - Templates establish record structure, but contributors assign IDs and maintain directory conventions manually. | Fully meets - Record construction and storage enforce per-kind IDs, metadata, and fixed directories. |
| FR-002 | Does not meet - Contributors must create the directories, adoption decision, and agent guidance themselves. | Fully meets - Initialization creates the adoption decision and skill, preserves existing records, and generates no specification. |
| FR-003 | Does not meet - The workflow provides no automated check to reject an invalid promotion before a file changes. | Fully meets - Document rules validate the prospective status before the storage workflow writes it. |
| FR-004 | Partially meets - Authors can edit status while preserving other content manually, but the workflow supplies no byte-preservation safeguard. | Fully meets - Targeted metadata updates preserve identity, date, path, body bytes, and unrelated YAML fields. |
| FR-005 | Does not meet - Independent authors have no shared allocation lock or coordinated publication. | Fully meets - The filesystem adapter locks allocation and stages publication, as defined by ADR-0007 and ADR-0008. |
| FR-006 | Does not meet - Authors must add metadata and headings themselves; there is no stdin-driven creation command. | Fully meets - The CLI accepts a title and piped body, and the core constructs a draft with metadata and a matching heading. |
| FR-007 | Partially meets - Contributors can browse files, but no record-aware metadata listing is supplied. | Fully meets - The list command reports record IDs, statuses, and paths. |
| FR-008 | Partially meets - Authors can change status text, but the workflow does not enforce transitions or replacement eligibility. | Fully meets - Lifecycle rules reject invalid transitions and require an accepted same-kind replacement when superseding. |
| NFR-001 | Does not meet - The workflow supplies no record-specific input limits or metadata-only listing. | Fully meets - Bounded reads enforce the documented limits, and catalog discovery reads metadata without document bodies. |
| NFR-002 | Does not meet - The workflow supplies no reusable document-rule implementation or isolated storage boundary. | Fully meets - Core rules are I/O-free, use cases access storage through a port, and adapters handle filesystem and terminal effects. |
| NFR-004 | Does not meet - The workflow supplies no record-management binary, CI checks, or release archives. | Fully meets - CI and release workflows define platform checks, native archives, and SHA-256 checksums. |
| C-001 | FAIL - The workflow provides no standalone Rust binary. | PASS - One Cargo binary uses local files and the user cache without a service, database, or project configuration. |
| C-002 | PASS - The workflow uses Markdown templates and Git without network calls, content execution, telemetry, plugins, or automatic updates. | PASS - The CLI has no network, provider, content-execution, telemetry, plugin, or automatic-update facilities. |

**Chosen option:** Makespec CLI

Makespec supplies the automation and safeguards required by FR-002, FR-003, FR-005, and FR-006 through FR-008 while retaining editable Markdown under FR-001 and preserving content under FR-004. The referenced source, tests, and CI workflows provide implementation evidence for these assessments.

Contributors must install the binary, use the fixed directories and canonical record structure, and keep the allocation cache writable. They still need an editor or agent to author prose and human review to judge its meaning. Readers do not need Makespec to inspect records. ADR-0002 through ADR-0008 record the implementation choices and their limits.

## Design

### Command workflows

Expose five commands: `init`, `new`, `list`, `status`, and `check`. Creation accepts a decision or specification body from stdin and adds record metadata. Listing reports metadata and paths for discovery. Status changes apply the documented lifecycle, and checking validates the records without changing them.

Use the command reference for exact syntax and lifecycle transitions, the templates for record structure and comparison grammar, and the skill for agent planning and approval boundaries. The CLI does not generate prose or decide whether a proposal should be accepted.

### Records and identity

Store decisions in `docs/decisions/` and specifications in `docs/specifications/`. Each kind has its own positive `u32` sequence, starting at `0001` and expanding beyond four digits without extra leading zeros. Use `ADR-` IDs for decisions and `RFC-` IDs for specifications, with IDs such as `ADR-0001` and filenames such as `0001-use-local-storage.md`.

The CLI adds YAML front matter containing `id`, `title`, `date` (`YYYY-MM-DD`), and `status`, plus a matching `# <ID>: <Title>` heading. Derive kind from the ID prefix. Keep status and date only in front matter; do not duplicate `kind`, `slug`, or `schema_version`.

Generate ASCII kebab-case slugs with `deunicode` transliteration. Creation titles must contain 1 to 200 bytes after trimming, contain no control characters or path separators, and produce a nonempty slug. Later title edits do not rename the file. CLI status updates preserve path, ID, date, body bytes, and unrelated YAML fields.

Allocate IDs under a project-wide advisory lock. Reconcile cached per-kind high-water marks against existing IDs and persist the next number before publishing a record. Failed writes may leave gaps. Store locks and counter state in the user cache, keyed by the canonical project path. Disconnected branches can allocate the same ID; `check` reports merged collisions.

### Initialization

`init` operates in the current directory, creating both document directories and one accepted decision, `ADR-0001`: "Use Makespec to manage decisions and specifications". The decision uses the initialization date, explains Makespec's reliance on the ADR and RFC formats, and links to the installed contribution guidance.

Initialization creates no specification and allocates no specification ID. When no specification records exist, it adds an empty `docs/specifications/.gitkeep` so Git can track the directory. Discovery ignores this regular placeholder, and initialization preserves existing specifications.

Embed the canonical skill files at build time and install them under `.agents/skills/makespec/`. Install no scripts or agent configuration. Skill discovery depends on agent support, and users must put `makespec` on the agent's `PATH`.

Preflight managed destinations, create both directories, stage files beside their destinations, and publish without overwriting existing files. Reinitialization preserves matching files, the valid accepted adoption decision, and existing specifications; conflicts fail with a path. If publication fails, remove only newly published files whose contents still match, reporting rollback failures. Initialization is not a multi-file transaction; rerunning it can complete a partial initialization. It does not initialize Git or commit files.

### Validation and updates

Parse front matter with `serde_yaml_ng` and headings/tables with `pulldown-cmark`. Validation checks metadata, matching title headings, ordered unique sections, canonical tables, criterion references, assessments, and option eligibility. Each comparison requires at least two options. `No change` is optional and should appear only when the current state is a meaningful alternative; when it is omitted, compare at least two distinct approaches that address the same need. Validation cannot judge prose quality or verify evidence.

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

- Run commands from the project root. With no command, print help without reading stdin. Commands do not prompt or open an editor; `new decision` and `new specification` require piped UTF-8 input.
- Limit stdin and complete documents to 2 MiB, catalog front matter to 64 KiB, and cached counter state to 1 KiB. Listing reads metadata without loading document bodies.
- Reject traversal, symlinked managed paths, and unexpected files in document directories; ignore only regular `.makespec-*.tmp` staging files and the regular `docs/specifications/.gitkeep` placeholder. Use bounded reads, exclusive publication, and same-directory staged replacements. Escape control characters in terminal output.
- Treat record contents as text. Make no network or LLM calls and provide no content execution, telemetry, plugin loading, or automatic updates.
- Keep dependencies locked in `Cargo.lock`. CI checks Linux, macOS, and Windows; tagged releases publish native archives and SHA-256 checksums. Distinguish release build coverage from runtime testing.

### Risks and mitigations

The specification and related decisions can drift from implementation. Review contract changes alongside the source and tests, and consult each record's status before treating a proposal as an accepted commitment.

The CLI cannot prove the truth of comparison evidence or recover forgotten allocations after external cache deletion. Reviewers must examine the referenced implementation and account for the local scope of ID allocation.

## Adoption

Install a verified native binary or build from source using the README. Put the executable on the contributor's or agent's `PATH`, then run `makespec init` from the project root and review the generated adoption decision and installed guidance.

Create records for the project's actual decisions and functionality. Use ordinary workspace tools for prose, the templates for structure, and the installed skill for authoring and approval boundaries. Initialization does not require a companion specification.

Existing records must follow the fixed directories and current format. Initialization preserves valid records and matching installed files; it does not convert arbitrary Markdown or overwrite conflicts. Resolve reported conflicts explicitly before retrying.

Verify adoption by creating and listing records, checking a complete proposal, and applying an authorized status change. Confirm that invalid promotions leave the original record unchanged. Run `makespec check` before contributing record changes. Maintainers run the contribution checks, with unit and CLI tests covering initialization, validation, concurrent allocation, status preservation, and bounded discovery.

Removing the tool requires no data export. Records remain ordinary Markdown available to editors and Git, with future numbering, lifecycle, and consistency checks left to contributors.

## References

- [ADR-0001: Use Makespec to manage decisions and specifications](../decisions/0001-use-makespec-to-manage-decisions-and-specifications.md) adopts Makespec and the documentation workflow.
- [ADR-0002: Use Rust for the standalone CLI](../decisions/0002-use-rust-for-the-standalone-cli.md) records the language and distribution choice.
- [ADR-0003: Keep the CLI offline and agent-driven](../decisions/0003-keep-the-cli-offline-and-agent-driven.md) records interaction and provider boundaries.
- [ADR-0004: Use Markdown records as the source of truth](../decisions/0004-use-markdown-records-as-the-source-of-truth.md) records document storage and metadata ownership.
- [ADR-0005: Use fixed content-oriented document directories](../decisions/0005-use-fixed-content-oriented-document-directories.md) records the layout and configuration boundary.
- [ADR-0006: Keep hexagonal boundaries within one binary](../decisions/0006-keep-hexagonal-boundaries-within-one-binary.md) records module responsibilities and dependency direction.
- [ADR-0007: Allocate IDs with locked external counters](../decisions/0007-allocate-ids-with-locked-external-counters.md) records allocation state and its limits.
- [ADR-0008: Stage managed writes and detect conflicts](../decisions/0008-stage-managed-writes-and-detect-conflicts.md) records publication and concurrent-edit safeguards.
- [README](../../README.md) defines installation and quick start.
- [Command reference](../../.agents/skills/makespec/references/cli.md) defines syntax, lifecycle transitions, and completion rules.
- [Decision template](../../.agents/skills/makespec/references/decision-template.md) and [specification template](../../.agents/skills/makespec/references/specification-template.md) define body structure and comparison grammar.
- [Makespec skill](../../.agents/skills/makespec/SKILL.md) defines authoring, planning, and approval boundaries.
- [Contribution guide](../../CONTRIBUTING.md) defines development checks, benchmarks, and releases.
- [Document rules](../../src/core/document.rs), [core workflows](../../src/core/service.rs), and [filesystem adapter](../../src/adapters/filesystem.rs) implement the product contracts.
- [CLI tests](../../tests/cli.rs) and [CI workflow](../../.github/workflows/ci.yml) provide executable checks.