# makespec CLI

Run commands from the project root.

- `makespec`: open the TUI in a terminal.
- `makespec init [PATH] [--no-skills]`: initialize document folders, accepted seed records, and the skill.
- `makespec new adr --title "Title"` or `makespec new rfc --title "Title"`: read the completed Markdown body from stdin and create a proposed record. The body must omit front matter and the ID/title heading. If stdin is a terminal, the command fails instead of waiting.
- `makespec list [adr|rfc] [--status STATUS]`: list matching records.
- `makespec status ID STATUS [--by ID]`: change a proposal's status. To supersede a record, first accept its replacement, then set the old record to `superseded --by NEW_ID`.
- `makespec check`: validate all project records, including required document structure and RFC comparison tables.

After creating or editing a record, run `makespec check` and fix reported issues before changing status.
