# Contributing

Contribute bug reports, fixes, documentation, or features to `makespec`.

## 🥇 Your first contribution

Install [stable Rust](https://rustup.rs/) and Git, then clone the repository. You can run the CLI from your checkout without installing the binary:

```sh
cargo run -- --help
```

Read the [product specification](docs/spec.md) and [contributor instructions](AGENTS.md) before making changes.

- For bug reports, include reproduction steps, the Makespec version, your operating system, and the expected and actual behavior.
- For features, describe the use case and discuss changes to the document format or command surface before implementing them.
- For pull requests, keep changes focused, add tests for behavior changes, and explain what changed and why. Run the checks below before opening a pull request.

## 🗂️ Conventions

- Follow the Rust formatting and spacing rules in [AGENTS.md](AGENTS.md).
- Preserve the hexagonal boundaries: keep document rules and workflows in `src/core/`, filesystem operations in `src/adapters/`, and CLI interaction in `src/startup/`.
- Preserve bounded reads, path validation, and protection against concurrent edits.
- Keep documentation and bundled agent instructions consistent with changes to commands or document formats.

## 👨‍🍳 Recipes

### How to run the checks?

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

CI runs those checks on Linux, macOS, and Windows.

### How to measure performance?

To measure catalog listing/check costs on Linux or macOS:

```sh
cargo bench --bench catalog -- 500
```

Cargo builds the optimized CLI and Rust benchmark. The benchmark creates a temporary project and reports wall time and peak resident memory for `list` and `check`, measuring each command in an isolated worker. The count defaults to 500. Results depend on the machine and filesystem; use them to set release budgets rather than treating one run as a portability guarantee.

One reference run on macOS ARM64 with 500 synthetic ADRs plus the two seed records measured `list` at 74 ms / 7.5 MiB peak RSS and `check` at 49 ms / 8.2 MiB. Provisional same-host budgets are 100 ms for `list` and `check` and 32 MiB peak RSS. Re-run the benchmark on release hardware before treating those budgets as portable.

### How to publish a release?

For maintainers: pushing a `v*` tag builds Linux and macOS release archives for x86_64 and ARM64, plus Windows x86_64, and publishes SHA-256 checksums.
