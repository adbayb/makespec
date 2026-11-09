# makespec CLI

Run commands from the project root.

Store decisions in `docs/decisions/` with `ADR-` IDs and specifications in `docs/specifications/` with `RFC-` IDs.

- `makespec`, `makespec --help`, or `makespec -h`: print help.
- `makespec init`: initialize document folders, accepted seed records, and the skill in the current directory.
- `makespec new decision "Title"` or `makespec new specification "Title"`: read a Markdown body from stdin, validate its structure, and create a draft record. The body must omit front matter and the ID/title heading. If stdin is a terminal, the command fails instead of waiting.
- `makespec list`: list all record metadata without loading document bodies.
- `makespec status ID STATUS [REPLACEMENT_ID]`: change a record's status using the lifecycle below. To supersede a record, first accept its replacement, then run `makespec status OLD_ID superseded NEW_ID`. The replacement ID is required for supersession and invalid for other statuses.
- `makespec check`: validate all project records. Drafts and rejected records receive structural checks; proposed, accepted, deprecated, and superseded records must be complete.

| Current status | Allowed next status |
| --- | --- |
| draft | proposed, rejected |
| proposed | draft, accepted, rejected |
| accepted | deprecated, superseded |
| deprecated | superseded |
| rejected, superseded | No further transitions |

Same-status requests are idempotent. Supersession requires a different, accepted replacement of the same kind; only the old record stores the link.

Promotion to `proposed` requires complete content. Specifications need all five sections, a complete comparison, and an eligible chosen option. Drafts may leave assessment cells, comparison rows, and the chosen option unfinished and omit Design and Adoption. Non-functional requirements (NFR) and constraints are optional; at least one functional requirement is required.

Both formats may end with one optional `## References` section. If present, it must contain content and follow the other template sections, including Open questions for specifications.

Exit codes are 0 for success, 1 for document or I/O errors, and 2 for invalid command-line arguments. Diagnostics go to stderr.
