# Contributor instructions

## Rust readability

- At module scope, keep related single-line imports, constants, or type aliases of the same kind together. Separate adjacent declarations with one blank line if either spans multiple lines.
- Preserve separate standard-library, dependency, and project import groups.
- Keep blank lines between functions, structs, enums, traits, and impl blocks regardless of length. Visibility (`pub`) does not affect spacing.
- Do not add spacing between struct fields, enum variants, or trait method signatures solely because they span multiple lines.
- Separate logical phases with one blank line: setup, validation, processing,
  side effects, and the final result.
- Keep closely related variable declarations together.
- Separate declarations from subsequent control flow with a blank line.
- Separate a final return expression from preceding work.
- Do not add blank lines immediately inside braces or split related expressions
  merely to add spacing.
- Preserve string literals, templates, and fixture contents during readability
  passes. Formatting-only changes must not rename or restructure code.
- Run `cargo fmt` after editing. It maintains formatting but does not enforce
  logical grouping; check that during review.

## Checks

Run the [contribution checks](CONTRIBUTING.md#how-to-run-the-checks).
