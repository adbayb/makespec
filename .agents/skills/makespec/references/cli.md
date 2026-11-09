# makespec CLI

Run commands from the project root.

- `makespec`, `makespec --help`, or `makespec -h`: print help.
- `makespec init [PATH] [--no-skills]`: initialize document folders, accepted seed records, and the skill.
- `makespec new adr --title "Title"` or `makespec new rfc --title "Title"`: read a Markdown body from stdin, validate its structure, and create a draft record. The body must omit front matter and the ID/title heading. If stdin is a terminal, the command fails instead of waiting.
- `makespec list [adr|rfc] [--status STATUS]`: list matching records.
- `makespec status ID STATUS [--by ID]`: change a record's status using the lifecycle below. To supersede a record, first accept its replacement, then set the old record to `superseded --by NEW_ID`.
- `makespec check`: validate all project records. Drafts and rejected records receive structural checks; proposed, accepted, deprecated, and superseded records must be complete.

| Current status | Allowed next status |
| --- | --- |
| draft | proposed, rejected |
| proposed | draft, accepted, rejected |
| accepted | deprecated, superseded |
| deprecated | superseded |
| rejected, superseded | No further transitions |

Promotion to `proposed` requires complete content. RFCs need all five sections, a complete comparison, and an eligible chosen option. Drafts may leave assessment cells, comparison rows, and the chosen option unfinished and omit Design and Adoption. Non-functional requirements (NFR) and constraints are optional; at least one functional requirement is required.

After creating or editing a record, run `makespec check` and fix reported issues before changing status.
