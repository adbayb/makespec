## Context

Replace with a short proposal summary, the current problem, and the intended outcome. Define scope and relevant non-goals, citing related decisions or specifications.

## Requirements and constraints

### Requirements

Write one testable requirement per row, with unique `FR-001`-style IDs for functional requirements. Include `NFR-001`-style IDs for non-functional requirements, such as measurable latency or reliability targets, only when relevant. Assign `MUST` (required), `SHOULD` (recommended), or `MAY` (optional) priority.

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-001 | MUST | Replace with a functional requirement |
| NFR-001 | SHOULD | Replace with a measurable non-functional requirement |

### Constraints

Keep this subsection only for genuine hard boundaries, such as a required platform or budget limit. Use unique `C-001`-style IDs. Constraints are pass/fail gates, separate from functional and non-functional requirements.

| ID | Constraint |
| --- | --- |
| C-001 | Replace with a hard constraint |

## Options

Identify the decision themes needed for an implementable proposal before listing candidates. Separate independent choices into named `###` subsections under Options, such as user experience and technical stack. For technical proposals, address stack and infrastructure choices when material to implementation; explain when an accepted record already settles them or they do not apply. Do not force unrelated choices into a single winner or enumerate every possible combination.

Use one comparison table and one `**Chosen option:**` line per theme. For a single decision, the table and choice may sit directly under Options without a subsection.

Before building a table, make a working inventory of candidates for that theme. Beyond the first ideas, examine relevant differences in user behavior, architecture, technology, operations, and existing approaches as applicable to that decision. Include user-specified candidates and other materially distinct, plausible alternatives that address the same need, including options that may fail a criterion so the comparison shows why. Exclude duplicates and implausible options with a reason; do not present guesses as verified facts. If the inventory might be incomplete, label that uncertainty.

Derive the option columns from the inventory. Two is a minimum per theme, not a target or maximum; there is no default count. Let themes have different counts when their decision spaces differ. Do not stop at an arbitrary shortlist, truncate meaningful alternatives to match another table, or pad a small decision space with redundant or implausible candidates.

Include `No change` only when it represents a meaningful current-state alternative; omit it when it would add no useful comparison, such as for a greenfield project with no existing capability. When omitting `No change`, compare at least two distinct approaches that address the same need. Start each table with a `Criterion ID` column, followed by one column per candidate using its name as the header. Compare candidates against the same criteria within the theme. Include rows for every relevant requirement and constraint ID; the tables together must cover all IDs. Repeat shared criteria in each affected theme, with explanations scoped to that choice.

Fill each cell with `LABEL - explanation` and supporting evidence. Label assumptions and distinguish expected behavior from verified results:

- Requirements: `Does not meet`, `Partially meets`, or `Fully meets`.
- Constraints: `PASS` or `FAIL`.

**Chosen option:** Selected candidate

Replace `Selected candidate` with one option per theme that fully meets every MUST requirement and passes every constraint assessed there. Copy its name exactly from that theme's table. Explain each choice and its tradeoffs using criterion IDs. Verify that the combined choices are compatible and together satisfy all MUST requirements and constraints; explain cross-theme dependencies rather than treating local eligibility as proof of compatibility. Before finishing, compare each table against its inventory and add any omitted plausible alternatives or explain their exclusion. Do not calculate an aggregate score.

## Design

Replace with the combined chosen options' behavior, stack, infrastructure, and interfaces as applicable, including failure cases, in enough detail to implement it. Include examples beside the behavior they illustrate.

### Risks and mitigations

Describe material risks and how to address them. Remove this optional subsection when no significant risks need explanation.

## Adoption

Replace with rollout, compatibility, and verification steps. Include migration or rollback when relevant to the functionality.

## Open questions

List only remaining non-blocking questions and how to resolve them. Remove this optional section when no questions remain.

## References

Link related decisions and specifications by ID using relative Markdown paths, and include external sources when relevant. Add a short note explaining each relationship. Remove this optional section when there are no references.
