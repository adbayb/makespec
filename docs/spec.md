# makespec implementation plan

## Problem and current state

Build a cross-platform Rust CLI and TUI for small, focused architecture decisions (ADRs) and functionality specifications (RFCs). Keep documents readable in Git and usable by humans and LLM agents without a service or database.

At the start of planning, the repository contained only `.git`; there was no application, dependency manifest, CI, documentation, or implementation convention to preserve. This remains a greenfield implementation.

This document is the repository source of truth. Implementation has not started; the implementation todos remain pending. The next step is scaffolding the Rust application.

## Confirmed scope

- Use Rust with Ratatui and Crossterm. Rust fits the memory, distribution, and safety constraints; no language change is warranted.
- Provide a browsing/creation/status TUI; use an external editor for Markdown editing.
- Make the CLI agent-driven: an LLM follows the installed skill, prepares document content, and invokes the CLI to validate and write it. The CLI makes no LLM or network calls.
- Use separate ADR/RFC numbering, YAML front matter, stable IDs and filenames, and creation dates.
- Bootstrap an accepted format RFC and accepted adoption ADR. Installing skills is the default; offer an opt-out.
- Support proposed, accepted, rejected, deprecated, and superseded statuses. Provide no delete command. The CLI creates and validates proposals; the TUI edits proposed documents through the external editor.
- Compare RFC options using weighted 0-5 scores, MUST pass/fail gates, and a do-nothing option.
- Default invocation opens the TUI on a terminal; non-terminal invocation shows help.
- The skill helps agents choose between an RFC and ADR, read accepted documents, and create proposals through the CLI. Agents do not execute implementation work as part of this feature.

## Product contract

### Minimal command surface

| Command | Responsibility |
| --- | --- |
| `makespec` | Open the TUI when stdin/stdout are terminals; otherwise show help without reading stdin or hanging. |
| `makespec init [PATH] [--no-skills]` | Create `docs/adrs`, `docs/rfcs`, accepted seed documents, and the agent skill. PATH defaults to the current directory. |
| `makespec new <adr\|rfc> --title TITLE` | Read a completed Markdown body from piped stdin, validate it, add CLI-owned metadata, allocate an ID, and write a proposed document. Fail immediately with guidance if stdin is a terminal. |
| `makespec list [adr\|rfc] [--status STATUS]` | List document IDs, titles, dates, and statuses; agents use workspace file-reading tools for full text. |
| `makespec status <ID> <STATUS> [--by ID]` | Apply a valid lifecycle change; `--by` identifies the replacement for superseded documents. |
| `makespec check` | Validate document metadata and the required ADR/RFC structure, along with IDs, filenames, status consistency, supersession links, and RFC comparison tables; suitable for CI and agent review. |

With no command, `makespec` opens the TUI on a terminal. The TUI browses, creates, reads, edits through the configured external editor, and changes document status using the same application use cases. Keep command-line operations non-interactive; they never prompt or open an editor. `init` supports the single `--no-skills` opt-out. Use stable, concise text output so agents can read command results without a separate output mode. Run commands from the project root instead of adding a project-discovery flag. Keep `--help` usable before initialization.

The agent creates the Markdown body and passes it to `new` on stdin:

```text
cat rfc-body.md | makespec new rfc --title "Offline search"
makespec list rfc --status accepted
makespec status RFC-0002 accepted
makespec check
```

`new` reads the completed Markdown sections from piped stdin, without YAML front matter or an ID/title heading. If stdin is a terminal, fail without waiting and direct the user to the TUI or to pipe a file. It requires valid content and never invokes an LLM. The CLI assigns ID, title, date, and proposed status, then validates and writes the canonical document. `new` and `check` use the same structure validator. The skill tells agents to read accepted ADRs/RFCs from the repository, draft the appropriate format, ask the user to resolve ambiguity, and invoke `new` only when the user requests document creation. An agent in plan mode proposes the document plan without writing files; after approval it can create the document through this same CLI command.

### Project layout and source of truth

```text
.agents/
  skills/
    makespec/
      SKILL.md
      references/
        adr-template.md
        cli.md
        planning.md
        rfc-template.md
docs/
  adrs/0001-use-adrs-and-rfcs.md
  rfcs/0001-specify-the-rfc-format.md
```

Markdown under `docs/adrs` and `docs/rfcs` is the source of truth. Use these fixed paths, run commands from the project root, and add no project configuration file. Keep runtime lock/staging files transient. `init` works in an existing directory and does not create a Git repository or commit files. Reserve `0001` for each seed on a fresh initialization. Re-running init validates matching managed files and leaves user content unchanged; conflicting or malformed files cause an explicit error.

Preflight all destinations before init writes. Stage generated content before publication; retain a small recovery manifest if an interrupted multi-file initialization needs completion. Do not claim multi-file atomicity. Recovery MUST preserve concurrent/user changes and report conflicts.

### Identity and metadata

Use filenames `<number-at-least-four-digits>-<kebab-title>.md`; expand beyond 9999 without wrapping. Every ADR and RFC MUST store `id`, `kind`, `title`, `slug`, `date: YYYY-MM-DD`, `status`, and `schema_version: 1` in YAML front matter. Front matter is the sole source of status for both document types; ADR and RFC bodies have no `## Status` section. The CLI owns these fields; agents provide only the body and title. `slug` records the creation-time filename suffix; title edits do not rename files. Optional metadata includes references and a `superseded_by` link on superseded documents.

Derive the next per-kind number by scanning existing files under a cross-platform advisory lock stored outside the repository; no high-water mark or project configuration is needed. Use exclusive creation and a conflict check; failures may leave gaps but MUST NOT reuse IDs. Reject duplicate or malformed identifiers rather than guessing. Independent Git branches can allocate the same ID; `check` detects merged collisions and documentation explains explicit renumbering before merge. Guarantee uniqueness within a project, not across repositories or disconnected clones.

Generate ASCII kebab-case with a documented transliteration policy; reject titles with an empty resulting slug, path separators, control characters, or invalid length. Validate typed dates and metadata at input boundaries. Creation dates are immutable through CLI operations; capture one local calendar date per operation, with a supplied clock/date in tests.

ADRs follow Nygard's title, Context, Decision, and Consequences structure, with status moved to front matter and no body Status heading. This intentional template adaptation avoids two status values that could diverge. RFCs retain a visible title and the Vue-derived sections described below; their status lives in front matter, and their visible Start Date mirrors `date`. Status changes update front matter for both ADRs and RFCs.

### Lifecycle and revision rules

| Current status | Allowed next status |
| --- | --- |
| proposed | accepted, rejected |
| accepted | deprecated, superseded |
| rejected | terminal; create a new proposal to reconsider |
| deprecated | superseded |
| superseded | terminal |

Same-status requests are idempotent. To supersede a record, first accept its replacement, then run `makespec status <old-ID> superseded --by <new-ID>`. The command verifies kind and accepted status, then records `superseded_by` only on the old document. This keeps the lifecycle change to one file; the replacement does not need a reciprocal link. `check` reports malformed chains or cycles. Before acceptance, validate complete required sections and metadata; RFCs also need a complete options comparison. Placeholder drafts fail acceptance.

The CLI creates complete proposed documents from agent-supplied Markdown. Agents can revise proposed documents with normal workspace editing tools; the TUI launches the configured editor for human edits. Preserve IDs, dates, slug, and status during edits; status changes use the status command. Validate edited content and check that the original has not changed before publishing. On invalid edits, retain the draft in an explicit recovery location and report its path. Git users can edit files outside the CLI; `check` validates structure but cannot establish historical immutability without inspecting Git history.

### Templates and initial documents

ADR template:

1. Front matter, including required `status`, and `# <ID>: <Title>`.
2. `## Context`, `## Decision`, `## Consequences`.
3. Short original prompts that keep a decision focused; no RFC-sized scoring machinery.

RFC template:

1. Front matter, including the same required `status` field as ADRs, and title; Start Date, Target Version, Reference Issues, and Implementation PR metadata.
2. Summary.
3. Basic example, optional when no API or usage example applies.
4. Motivation.
5. Requirements: Functional (`FR-001`) and Non-functional (`NFR-001`).
6. Constraints (`C-001`).
7. Detailed design.
8. Drawbacks.
9. Alternatives, including a comparison matrix, rationale, and do-nothing option.
10. Adoption strategy.
11. Unresolved questions, permitted in proposals but not as placeholders for acceptance criteria.

Keep the criterion list focused on requirements and constraints that distinguish the alternatives. Each criterion has a stable ID, description, priority (MUST/SHOULD/MAY), positive integer weight reflecting its relative importance, and verification/evidence note. Each option scores every criterion from 0 (does not satisfy) to 5 (fully satisfies); justify scores with evidence rather than treating the numbers as objective measurements. For MUST criteria, record a separate pass/fail assessment: failing any MUST makes the option ineligible regardless of score. Compute the normalized weighted score as `100 * sum(weight * score) / (5 * sum(weight))`; annotate ineligible options and do not recommend an option by aggregate score alone. Unknown assessments remain explicit and block acceptance. Include a rationale for the chosen eligible option.

Define a canonical Markdown table grammar in the seed RFC so `check` can validate criterion references, unique options, score ranges, and gates without interpreting arbitrary prose. Use a Markdown parser rather than regular expressions across full documents. Treat totals as derived and calculate them during validation; never present a high aggregate score as a substitute for evidence or judgment. A comparison table may use one row per option/criterion with summary rows, keeping evidence readable as criteria grow.

Bootstrap:

- `ADR-0001` records the decision to use ADRs for focused decisions and RFCs for functionality/change specifications, with workflow consequences.
- `RFC-0001` specifies the RFC format, metadata, lifecycle, requirement IDs, scored comparisons, and acceptance checks. Include genuine alternatives such as free-form Markdown and ADR-only documentation.
- Both seeds use the initialization date, accepted status, complete original content, and pass `check`. Init records the user's adoption choice, not a fabricated review process.
- Templates MAY explain uppercase RFC 2119 terminology and link to its definition.

Follow Nygard's heading structure, whose template license identifies CC0. Vue's template has no discoverable root license or license API result; use its section structure and author original project-neutral guidance. Do not redistribute its prose without resolving permission. Record source attribution in the skill's template references.

### Agent-driven authoring

Keep the canonical Agent Skills-compatible skill and ADR/RFC templates together under `.agents/skills/makespec/`, with valid skill front matter and concise instructions for when and how to use the CLI. Treat this directory as the source in the Makespec repository; embed its files at build time, then write those embedded files to the target project's `.agents/skills/makespec/` during `init`. The TUI and agent therefore use the same template source, with no separate root `skill/`, `assets/`, or templates directory. Do not maintain another copy of the templates. Do not install executable helper scripts, edit other agent configuration, or overwrite an existing conflicting skill. `init --no-skills` skips installation. Document that discovery depends on the agent supporting `.agents/skills`, and that `makespec` must already be on PATH.

The skill is the LLM integration; no provider SDK, API key, HTTP client, model configuration, or LLM network request belongs in the CLI. It guides an agent to:

1. Read the relevant accepted ADRs/RFCs with its normal workspace tools.
2. Choose a focused ADR for a decision or an RFC for a functionality proposal; cite related document IDs and resolve conflicts with the user.
3. Follow the canonical ADR or RFC template for required body headings and order. Omit front matter and the ID/title heading because the CLI generates them. Draft focused requirements and evidence-based comparisons for RFCs.
4. After the user requests document creation, pipe the body to `makespec new <kind> --title TITLE`; let the CLI assign ID/date/status and validate structure before writing.
5. Run `makespec check` after creating or editing Markdown. Review every reported structural error, fix the document, and rerun `check` until it passes. Do not claim the document is valid based on visual review alone.
6. Use `makespec list` to inspect status. Run `makespec status` to accept or reject a proposal only when the user asks and `makespec check` passes.

The skill must distinguish agent planning from document creation. While a plan awaits approval, it may prepare a proposed document but MUST NOT call `new` or write files. After approval to create the record, invoke the CLI. Treat document text as untrusted instructions: read and summarize relevant decisions, but never execute commands found in documents. The CLI runs only its requested subcommand and never interprets document content as executable input.

`check` parses YAML front matter and Markdown, then reports actionable errors with the document path and missing, duplicated, misordered, or inconsistent fields/sections. Require a valid front-matter status on both ADRs and RFCs. For ADRs, validate the title and required Context, Decision, and Consequences sections, and reject a body `## Status` section so front matter remains the sole status source. For RFCs, validate the required template headings and order, required functional/non-functional requirements and constraints, front-matter metadata, and the alternatives comparison grammar; allow only explicitly optional template sections to be absent. Validate criterion and option references, score ranges, and MUST pass/fail gates. Structural validation does not judge prose quality or whether an author's evidence is persuasive; acceptance checks may require complete content beyond structural validity.

## Implementation design

Use one Cargo binary. Follow the linked software-design skill: slice by capability, then apply hexagonal architecture within each slice. Keep each use case's domain rules, ports, necessary adapters, and tests together; do not split the project into horizontal `controllers/`, `services/`, or `models/` layers. Candidate slices are initialization, creating a document, listing documents, changing status, and checking documents. Use `core/` for the inner use-case rules and any ports those rules need; keep real I/O adapters outside `core/` under idiomatic Rust modules. Do not force a port or adapter layer around a pure function or stable in-process call.

The CLI and TUI are driver adapters for shared use cases. File access, locking, clock, and editor process launch are driven effects; isolate them behind narrow core-owned ports where substitution or independent testing needs that boundary. Use cases own the invariants and depend only on core contracts; adapters translate filesystem and terminal details and do not depend on one another. `startup/cli` and `startup/tui` compose real adapters; tests drive use cases with deterministic fakes. Keep TUI rendering and state transitions free of I/O, and route user actions through the same use cases as CLI commands. Do not add a generic repository framework, event bus, database, service, or abstraction without a demonstrated consumer.

Apply the skill's other constraints: the smallest API that meets the document workflow, explicit types and errors at boundaries, one owner for each invariant, no infrastructure types in use-case contracts, inward dependency direction, and no pattern without current pain. Keep clocks/ID allocation controllable in tests and resource cleanup explicit. Define failure and recovery behavior at each file-write boundary; avoid cross-file updates when one-sided metadata preserves the required invariant. Before implementation, follow the linked skill workflow and consult its Rust language conventions, architecture styles, API-surface, SOLID/GRASP, cohesion/coupling, patterns, clean-code, testing/refactoring, security, and code-smell references.

Candidate dependencies to evaluate during implementation:

- `clap` for parsing/help.
- `ratatui` and `crossterm` with minimum required features.
- `serde` and a maintained YAML parser for typed document metadata.
- `pulldown-cmark` or an equivalent maintained parser for document/table validation.
- A focused date library with only necessary features.
- A cross-platform file-lock API/crate, temporary-file support, and focused error types.
- Test-only CLI/temporary-directory helpers where they reduce test plumbing.

Choose maintained releases and an MSRV based on dependency requirements; commit Cargo.lock. Review feature flags and licenses. Avoid `unsafe` in application code, unnecessary clones, preloading all document bodies, persistent indexes, and background file watchers.

TUI behavior: metadata list with kind/status filters, detail view, creation wizard, status action, external edit, and help. Load one selected body on demand; bound caches and input sizes. Provide keyboard-only navigation, clear focus, scrolling, resizing, empty-state guidance, and small-terminal fallback. Suspend and restore the terminal around editors. Restore terminal state on errors, panic, and interruption. Strip/escape control sequences from document content before terminal rendering.

Treat external editor invocation as explicit user-authorized code execution. Respect user editor configuration using a tested cross-platform argument parser and direct process spawning, never a shell command string. Never take editor commands from project documents. Report missing editors and exit failures.

## Security, performance, and distribution

- Validate canonical path containment within the selected project; account for symlinks and check/use races rather than relying on string-prefix checks. Reject traversal; use exclusive creation, locks, same-directory staging, and race-resistant file operations where supported. Document that advisory locks cannot control arbitrary external editors.
- Check expected source content before replacing a file, and preserve unknown files/YAML fields/body text during targeted metadata changes where schema permits. Reject unsupported schema versions and malformed files with paths and explanations.
- Treat Markdown as text; no HTML execution, plugin loading, telemetry, auto-update, credential storage, or automatic network access.
- Set explicit input/output limits before allocating buffers. Load catalog metadata and selected bodies on demand; avoid whole-project body retention.
- Add a reproducible synthetic-catalog benchmark and document measured peak RSS, idle CPU, and navigation latency. Establish release budgets from the first prototype; memory efficiency is a measurement requirement, not an unverified claim.
- CI covers stable Rust on Linux, macOS, and Windows, formatting, Clippy, unit/integration tests, and release builds. Cover x86_64 and ARM64 release targets through native runners or supported cross-build jobs; distinguish build coverage from runtime coverage.
- Publish a single binary through Cargo and GitHub release archives with checksums. Document installation, PATH, editor setup, Windows behavior, and agent discovery. Do not add installers that download/execute shell scripts.

## Ordered implementation todos

These items remain pending. Dependencies appear below so contributors can track execution without access to the planning session.

1. **Scaffolding the Rust application** (`scaffold`): Cargo binary, CLI/TUI composition roots, idiomatic module boundaries, minimal dependencies, typed errors, and platform CI.
2. **Defining formats and skill references** (`formats`): typed metadata, ADR/RFC templates, scoring grammar, source attribution, accepted seed documents, and canonical skill files.
3. **Implementing document use cases** (`workflows`): initialization, safe ID allocation and file writes, create/list/status/check, lifecycle rules, and validation ports/adapters.
4. **Adding agent skill integration** (`agents`): install the skill by default with init, document CLI workflows, and verify an LLM can create documents by invoking CLI through stdin.
5. **Implementing the TUI** (`tui`): browser, creation/status actions, editor handoff, terminal cleanup, and shared use cases.
6. **Hardening and releasing** (`release`): cross-platform behavior, dependency/security review, benchmark budgets, installation docs, release artifacts and checksums.

Dependencies: formats depends on scaffold; workflows depends on formats; agents depends on workflows; tui depends on workflows; release depends on agents and tui. Canonical templates and skill references belong to formats; installation and usage examples belong to agents. Extract shared document types only when multiple slices need the same invariant; avoid a global models dumping ground.

## Verification and acceptance

Add tests alongside each slice and executable integration tests for externally observable behavior:

- Fresh init creates exactly the required accepted seed documents and default skill. Opt-out, repeat init, interrupted init, conflicts, and existing user files behave as specified.
- ADR headings follow Nygard; RFC sections cover Vue's structure plus requirements, constraints, and scored alternatives. CLI, TUI, and installed skill guidance use the same canonical templates. Both seeds have valid dates and metadata.
- `check` requires front-matter status on both document types, rejects an ADR body `## Status` heading, and reports missing, duplicate, misordered, or inconsistent ADR/RFC sections with actionable file locations. Valid documents may omit only explicitly optional sections. `new` rejects invalid structure without writing a file.
- Independent sequences, 9999 overflow, rejected/empty slugs, concurrent creation, failed-write gaps, and merged duplicate IDs cannot silently produce conflicting records.
- Piped `new` accepts only valid Markdown body input, returns the CLI-assigned ID/path, and fails immediately with useful guidance when stdin is a terminal.
- Lifecycle transitions, immutable fields, proposed-only editing, invalid editor output, concurrent edits, one-sided supersession links/cycles, write failures, and acceptance validation follow the contract.
- Scoring rejects missing/duplicate criterion references, out-of-range scores, invalid weights, unassessed MUST gates, and ineligible chosen options. Verify the weighted formula with behavior-based cases.
- The installed skill follows the bundled template, runs `makespec check` after creating or editing a document, fixes reported errors, and does not accept a proposal until validation passes.
- Document text cannot trigger command execution, filesystem writes, or network calls; only an explicit CLI invocation performs its documented action.
- TUI state tests and Ratatui test-backend snapshots cover navigation, filtering, errors, resizing, and actions. Add platform smoke checks for terminal restoration and editor spawning.
- Path traversal, symlinked destinations, malformed YAML/Markdown, terminal escape sequences, and unexpected files produce safe explicit failures.
- Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` in CI; run focused tests while implementing. Confirm release builds and installation smoke tests on supported platforms.

Finish with a dogfooding workflow: initialize this project using the compiled CLI, inspect the two accepted seeds, ask an agent to read them and draft an RFC with scored alternatives, create it through `makespec new`, accept it through the status workflow, supersede a decision, and validate the repository. The workflow needs no network access.

## Design note

- Slice: local specification/decision management with agent-driven authoring, exposed through CLI/TUI.
- API surface: one binary, five commands plus the default TUI entry; no public library, LLM provider interface, or MCP protocol.
- Inner style: hexagonal architecture by capability, with `core/` holding use cases and inner-owned ports; CLI/TUI and file/editor adapters sit outside it.
- Keep filesystem and terminal concerns outside document rules; no network or provider layer.
- Share use cases between CLI and TUI; the CLI alone assigns document identity and lifecycle metadata.
- Add ports only for concrete I/O boundaries; no generic framework or speculative plugin system.
- Pure rules accept supplied dates/IDs; filesystem tests use temporary directories and deterministic fakes.

## Sources

- Nygard template: https://github.com/architecture-decision-record/architecture-decision-record/tree/main/locales/en-001/templates/decision-record-template-by-michael-nygard
- Nygard license: https://raw.githubusercontent.com/architecture-decision-record/architecture-decision-record/main/locales/en-001/templates/decision-record-template-by-michael-nygard/LICENSE.md
- Vue RFC template: https://github.com/vuejs/rfcs/blob/master/0000-template.md
- Requirement language: https://www.rfc-editor.org/rfc/rfc2119.txt
- Ratatui framework: https://ratatui.rs/
- Agent Skills format: https://agentskills.io/specification
- Software design skill: https://github.com/adbayb/stack/tree/main/stack/skills/software-design
