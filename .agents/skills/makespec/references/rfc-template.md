## Context

Replace with a short proposal summary, the current problem, and the intended outcome. Define scope and relevant non-goals, citing related ADRs or RFCs.

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

Describe each candidate, including `No change` (keep the current behavior without the proposed feature or modification). Name options in the column headers and compare them against the same criteria. Include one row for each requirement and constraint ID; remove rows for criteria you omit above.

Leave unassessed cells blank while the RFC is a draft. Before promoting it to proposed, fill each cell with `LABEL - explanation` and supporting evidence:

- Requirements: `Does not meet`, `Partially meets`, or `Fully meets`.
- Constraints: `PASS` or `FAIL`.

| Criterion ID | No change | Proposed approach |
| --- | --- | --- |
| FR-001 | | |
| NFR-001 | | |
| C-001 | | |

**Chosen option:**

Leave the choice blank while it is under discussion. Before promotion to proposed, name an option from the table that fully meets every MUST requirement and passes every constraint. Explain the choice and its tradeoffs using criterion IDs. Do not calculate an aggregate score.

## Design

Replace with the chosen option's behavior and interfaces, including failure cases, in enough detail to implement it. Include examples beside the behavior they illustrate. Drafts may omit this section until an option is chosen.

### Risks and mitigations

Describe material risks and how to address them. Remove this optional subsection when no significant risks need explanation.

## Adoption

Replace with rollout, compatibility, and verification steps. Include migration or rollback when relevant to the functionality. Drafts may omit this section.

## Open questions

List remaining questions and how to resolve them. Resolve questions that prevent a supported choice or implementable design before promotion to proposed. Remove this optional section when no questions remain.

## References

Link related ADRs and RFCs by ID using relative Markdown paths, and include external sources when relevant. Add a short note explaining each relationship. Remove this optional section when there are no references.
