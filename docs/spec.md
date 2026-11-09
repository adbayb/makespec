# Makespec product specification

Makespec is a cross-platform Rust CLI for focused architecture decisions (ADRs) and functionality proposals (RFCs). Records live in Git as Markdown; the CLI needs no service, database, or project configuration.

## Canonical guides

- [README](../README.md): installation and quick start.
- [Command reference](../.agents/skills/makespec/references/cli.md): commands, lifecycle transitions, and completion rules.
- [ADR template](../.agents/skills/makespec/references/adr-template.md) and [RFC template](../.agents/skills/makespec/references/rfc-template.md): body structure and comparison grammar.
- [Agent Skill](../.agents/skills/makespec/SKILL.md): authoring workflow, planning, and approval boundaries.
- [Contributing](../CONTRIBUTING.md): development checks, benchmarks, and releases.

These files define their respective contracts; this specification covers the remaining storage and implementation invariants.

## Records and identity

Store ADRs in `docs/decisions/` and RFCs in `docs/specifications/`. Each kind has its own positive `u32` sequence, starting at `0001` and expanding beyond four digits without extra leading zeros. Use IDs such as `ADR-0001` and filenames such as `0001-use-local-storage.md`.

The CLI adds YAML front matter containing `id`, `title`, `date` (`YYYY-MM-DD`), and `status`, plus a matching `# <ID>: <Title>` heading. Derive kind from the ID prefix. Keep status and date only in front matter; do not duplicate `kind`, `slug`, or `schema_version`.

Generate ASCII kebab-case slugs with `deunicode` transliteration. Creation titles must contain 1 to 200 bytes after trimming, contain no control characters or path separators, and produce a nonempty slug. Later title edits do not rename the file. CLI status updates preserve path, ID, date, body bytes, and unrelated YAML fields.

Allocate IDs under a project-wide advisory lock. Reconcile cached per-kind high-water marks against existing IDs and persist the next number before publishing a record. Failed writes may leave gaps. Store locks and counter state in the user cache, keyed by the canonical project path. Disconnected branches can allocate the same ID; `check` reports merged collisions.

## Initialization

`init` operates in the current directory, creating the two document directories and accepted seeds using the initialization date:

- `ADR-0001` records adoption of ADRs for decisions and RFCs for functionality.
- `RFC-0001` specifies the RFC format with complete requirements, constraints, and alternative comparisons.

Embed the canonical skill files at build time and install them under `.agents/skills/makespec/`. Install no scripts or agent configuration. Skill discovery depends on agent support, and users must put `makespec` on the agent's `PATH`.

Preflight managed destinations, stage files beside their destinations, and publish without overwriting existing files. Reinitialization preserves matching files and valid accepted seeds; conflicts fail with a path. If publication fails, remove only newly published files whose contents still match, reporting rollback failures. Initialization is not a multi-file transaction; rerunning it can complete a partial initialization. It does not initialize Git or commit files.

## Validation and updates

Parse front matter with `serde_yaml_ng` and headings/tables with `pulldown-cmark`. Validation checks metadata, matching title headings, ordered unique sections, canonical tables, criterion references, assessments, and option eligibility. It cannot judge prose quality or verify evidence.

Choose validation depth from the document status. Before writing a promotion to proposed or accepted, validate the prospective document with its new status. Failed readiness checks and invalid transitions leave the original file unchanged.

Supersession records `superseded_by` only on the old record. Require a different, accepted replacement of the same kind when applying the transition. Historical targets may later become deprecated or superseded; `check` rejects missing or invalid targets and cycles.

Use normal workspace tools for body edits. Before replacing a record, compare its current bytes with the loaded version under the lock, stage the replacement, and check again before publication. Advisory locks coordinate Makespec processes, not external editors. `check` cannot establish historical immutability without Git history.

## Implementation boundaries

Keep one Cargo binary and the existing hexagonal boundaries:

- `src/core/document.rs` owns document construction, parsing, and validation.
- `src/core/service.rs` owns use cases and the `ProjectStore` port.
- `src/adapters/` implements filesystem access and supplies embedded skill assets.
- `src/startup/cli.rs` composes adapters and handles terminal input/output.

Pass the operation date into use cases. Keep filesystem and terminal effects outside document rules; test pure rules without I/O and storage workflows with temporary directories. Add no public library, provider interface, generic repository framework, or speculative abstraction.

## Safety and distribution

- Run commands from the project root. With no command, print help without reading stdin. Commands do not prompt or open an editor; `new` requires piped UTF-8 input.
- Limit stdin and complete documents to 2 MiB, catalog front matter to 64 KiB, and cached counter state to 1 KiB. Listing reads metadata without loading document bodies.
- Reject traversal, symlinked managed paths, and unexpected files in document directories; ignore only regular `.makespec-*.tmp` staging files. Use bounded reads, exclusive publication, and same-directory staged replacements. Escape control characters in terminal output.
- Treat record contents as text. Make no network or LLM calls and provide no content execution, telemetry, plugin loading, or automatic updates.
- Keep dependencies locked in `Cargo.lock`. CI checks Linux, macOS, and Windows; tagged releases publish native archives and SHA-256 checksums. Distinguish release build coverage from runtime testing.
