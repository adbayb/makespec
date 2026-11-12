# makespec CLI

Run commands from the project root.

Store decisions in `docs/decisions/` with `ADR-` IDs and specifications in `docs/specifications/` with `RFC-` IDs.

- `makespec`, `makespec --help`, or `makespec -h`: print help.
- `makespec init`: create both document folders, one accepted Makespec adoption decision, and the skill with its templates and command reference in the current directory. When no specifications exist, add an empty `docs/specifications/.gitkeep` for Git to track the directory. It creates no specification and preserves existing records. In a fresh project, the first specification uses `RFC-0001`.
- `makespec new decision "Title"` or `makespec new specification "Title"`: read a Markdown body from stdin, validate its initial structure, and create a `proposed` record. The body must omit front matter and the ID/title heading. If stdin is a terminal, the command fails instead of waiting. Creation can preserve incomplete content; run `makespec check` to find readiness errors.
- `makespec list`: list all record metadata without loading document bodies.
- `makespec status ID STATUS [REPLACEMENT_ID]`: change a record's status using the lifecycle below. To supersede a record, first accept its replacement, then run `makespec status OLD_ID superseded NEW_ID`. The replacement ID is required for supersession and invalid for other statuses.
- `makespec check`: validate all project records. Rejected records receive structural checks; proposed, accepted, deprecated, and superseded records must be complete. An incomplete proposal fails checking and cannot be accepted.

| Current status | Allowed next status |
| --- | --- |
| proposed | accepted, rejected |
| accepted | deprecated, superseded |
| deprecated | superseded |
| rejected, superseded | No further transitions |

Same-status requests are idempotent and do not validate readiness; use `makespec check` for that. Supersession requires a different, accepted replacement of the same kind; only the old record stores the link.

Creation requires Context, Decision, and Consequences for decisions. Specifications initially require Context, Requirements and constraints, and Options, including at least one functional requirement and a comparison table. Checking a proposal and accepting it require complete content: all five specification sections, assessments for each criterion and option, an eligible chosen option, and no template placeholders. Non-functional requirements (NFR) and constraints are optional.

ADR and RFC sequences are independent. Under the project lock, an empty record kind restarts at `0001`, even at a reused project path with cached allocations. For a nonempty kind, the allocator retains cached high-water marks, including allocations for removed records or failed writes. Do not renumber existing records to close gaps.

Both formats may end with one optional `## References` section. If present, it must contain content and follow the other template sections, including Open questions for specifications.

Exit codes are 0 for success, 1 for document or I/O errors, and 2 for invalid command-line arguments. Diagnostics go to stderr.
