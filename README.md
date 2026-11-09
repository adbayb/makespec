# Makespec

Makespec manages focused architecture decision records (ADRs) and functionality proposals (RFCs) as Markdown in your repository. It uses Rust, Ratatui, and Crossterm. The CLI has no LLM integration, network calls, telemetry, or document-driven command execution.

## Install

Install the stable Rust toolchain, then install Makespec from a checkout:

```sh
cargo install --path . --locked
```

Make sure Cargo's binary directory is on `PATH`. Configure `VISUAL` or `EDITOR` to use external editing in the TUI, for example `export EDITOR="vim"`.
Windows users can install with the same Cargo command and use PowerShell to pipe a Markdown body, for example `Get-Content -Raw proposal.md | makespec new rfc --title "Offline search"`. Editor settings are split into executable and arguments without invoking a shell; quote paths or arguments that contain spaces.

## Start a project

Run this from the target repository:

```sh
makespec init
makespec check
makespec
```

`init` creates `docs/adrs/`, `docs/rfcs/`, accepted seed records, and `.agents/skills/makespec/`. Pass `--no-skills` to skip skill installation. The installed skill requires an agent that discovers `.agents/skills`; Makespec must also be on that agent's `PATH`.

## Commands

| Command | Purpose |
| --- | --- |
| `makespec` | Open the TUI in a terminal; print help when stdin or stdout is not a terminal. |
| `makespec init [PATH] [--no-skills]` | Bootstrap a project. PATH defaults to the current directory. |
| `makespec new adr --title "Title"` | Read a complete ADR body from stdin and create a proposed ADR. Use `rfc` for an RFC. |
| `makespec list [adr\|rfc] [--status STATUS]` | List records and their statuses. |
| `makespec status ID STATUS [--by ID]` | Accept, reject, deprecate, or supersede a record. |
| `makespec check` | Validate front matter, document structure, filenames, lifecycle links, and RFC comparison tables. |

For example, provide a Markdown body without front matter or the ID/title heading:

```sh
cat proposal.md | makespec new rfc --title "Offline search"
makespec check
makespec list rfc --status proposed
makespec status RFC-0002 accepted
```

Create proposals with the matching template in `.agents/skills/makespec/references/`. Run `makespec check` after creating or editing a document; fix reported issues and rerun it before accepting or rejecting a proposal.

The TUI supports ↑/↓ navigation, `k` to cycle document-kind filters, `s` to cycle status filters, `n` to create, `e` to edit a proposal in the configured editor, `a` to accept, `r` to reject, Page Up/Down to scroll the selected record, and `q` to quit. Status changes use the same validation rules as the CLI.

## Records and safety

ADRs capture small, focused decisions. RFCs specify functionality and include functional/non-functional requirements, constraints, and a scored alternatives matrix. Each kind has its own incrementing ID. Both use YAML front matter as the sole status source.

Makespec stores records in Git and keeps ID allocation locks outside the repository. It validates paths and metadata, limits document input to 2 MiB, uses exclusive creation, and checks for external edits before replacing a file. Advisory locks coordinate Makespec processes but cannot control arbitrary external editors. Invalid TUI edits are saved under the ignored `.makespec/recovery/` directory.

The full product and implementation specification is in [`docs/spec.md`](docs/spec.md).

## Development

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

CI runs those checks on Linux, macOS, and Windows.
Pushing a `v*` tag builds Linux, macOS, and Windows release archives for x86_64 and ARM64 targets and publishes SHA-256 checksums.

To measure catalog listing/check costs, TUI idle CPU, and navigation latency on a POSIX system:

```sh
cargo build --release
python3 benchmarks/catalog.py target/release/makespec 500
```

The script creates a temporary project and reports wall time and peak resident memory for CLI commands, plus idle CPU and five navigation samples. Results depend on the machine and terminal; use them to set release budgets rather than treating one run as a portability guarantee.

One reference run on macOS ARM64 with 500 synthetic ADRs plus the two seed records measured `list` at 74 ms / 7.5 MiB peak RSS, `check` at 49 ms / 8.2 MiB, TUI idle CPU at 3.0%, and navigation at 98–119 ms / 9.4 MiB. Provisional same-host budgets are 100 ms for `list` and `check`, 32 MiB peak RSS, 5% idle CPU, and 200 ms navigation p95. Re-run the benchmark on release hardware before treating those budgets as portable.
