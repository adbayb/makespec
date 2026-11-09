# Contributing

Contribute bug reports, fixes, documentation, or features to `makespec`.

## 🥇 Your first contribution

Install [stable Rust](https://rustup.rs/) and Git, then clone the repository. You can run the CLI from your checkout without installing the binary:

```sh
cargo run -- --help
```

Read [RFC-0002: Makespec product requirements document (PRD)](docs/specifications/0002-makespec-product-requirements-document.md) and the [contributor instructions](AGENTS.md) before making changes.

- For bug reports, include reproduction steps, the Makespec version, your operating system, and the expected and actual behavior.
- For features, describe the use case and discuss changes to the document format or command surface before implementing them.
- For pull requests, keep changes focused, add tests for behavior changes, and explain what changed and why. Run the checks below before opening a pull request.
- Use [Conventional Commits](https://www.conventionalcommits.org/) for commits merged into `main`: `fix:` for fixes, `feat:` for features, and `!` or a `BREAKING CHANGE:` footer for breaking changes. Release Please uses them to choose versions and write release notes.

## 🗂️ Conventions

- Follow the Rust formatting and spacing rules in [AGENTS.md](AGENTS.md).
- Preserve the hexagonal boundaries: keep document rules and workflows in `src/core/`, filesystem operations in `src/adapters/`, and CLI interaction in `src/startup/`.
- Preserve bounded reads, path validation, and protection against concurrent edits.
- Keep documentation and bundled agent instructions consistent with changes to commands or document formats.

## 👨‍🍳 Recipes

### How to run the checks?

```sh
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
```

CI runs those checks on Linux, macOS, and Windows. Pushes to `main` run the same CI workflow through the release workflow before any release automation.

### How to measure performance?

Install [Hyperfine 2 or later](https://github.com/sharkdp/hyperfine#installation) on your `PATH`, then measure catalog listing/check costs on Linux or macOS:

```sh
cargo bench --bench catalog -- 500
```

Cargo builds the optimized CLI and a Rust catalog generator. Hyperfine measures wall time and peak resident set size (RSS) for `list` and `check`, using three warmup runs followed by repeated measurements. The catalog defaults to 500 synthetic ADRs plus the two seed records.

Provisional macOS ARM64 budgets for this catalog are 100 ms per command and 32 MiB peak resident set size. Recalibrate them on release hardware. Linux measurements can include Hyperfine's inherited memory floor for small processes; see its [metric definitions](https://github.com/sharkdp/hyperfine#choosing-metrics-and-units).

### How to publish a release?

After CI passes on `main`, [Release Please](https://github.com/googleapis/release-please) opens or updates a release pull request containing version changes in `Cargo.toml` and `Cargo.lock`, plus a generated `CHANGELOG.md`. Review and merge that PR to release.

The workflow uses the built-in `GITHUB_TOKEN`, with no additional secrets.

After the release PR merges and CI passes, Release Please creates a `v<version>` tag and GitHub release. The same workflow builds that exact tag for Linux and macOS x86_64/ARM64 and Windows x86_64, then uploads the archives and SHA-256 checksums to the existing release. It does not publish to crates.io. If a build or upload fails, rerun the failed jobs to finish attaching assets.
