---
id: RFC-0001
title: Specify the RFC format
date: 2026-11-09
status: accepted
---

# RFC-0001: Specify the RFC format

## Context

Define a consistent RFC format for functionality proposals. Use this format for product requirements documents (PRDs) as well as technical change proposals. Authors need a reviewable record of the problem, evaluation criteria, options, chosen design, and adoption approach without a hosted service.

## Requirements and constraints

### Requirements

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-001 | MUST | An RFC specifies functionality requirements, constraints, and qualitative alternatives. |
| NFR-001 | SHOULD | An RFC remains readable and reviewable as plain Markdown in Git. |

### Constraints

| ID | Constraint |
| --- | --- |
| C-001 | The format must work with repository files and require no service or database. |

## Options

- No change: leave proposal formats to individual authors.
- Free-form Markdown: record proposals without required sections or comparison rules.
- ADR-only records: reuse the focused decision format for functionality proposals.
- Makespec format: use shared criteria, an options comparison, and a design for the chosen option.

| Criterion ID | No change | Free-form Markdown | ADR-only records | Makespec format |
| --- | --- | --- | --- | --- |
| FR-001 | Does not meet - Proposals need not capture the required areas. | Does not meet - No structure requires all required areas. | Does not meet - ADRs do not specify functionality proposals. | Fully meets - The format requires functionality criteria, constraints, and comparisons. |
| NFR-001 | Partially meets - Markdown stays readable, but authors lack a shared structure. | Fully meets - Free-form Markdown remains easy to read and diff. | Fully meets - ADRs remain plain Markdown in Git. | Fully meets - The format uses reviewable Markdown tables. |
| C-001 | PASS - It requires no service or database. | PASS - It uses repository files only. | PASS - It uses repository files only. | PASS - The CLI reads and writes local Markdown files. |

**Chosen option:** Makespec format

The chosen format fully meets FR-001 and satisfies the hard constraint. It keeps records portable while making comparison criteria explicit.

## Design

Every RFC has YAML front matter containing `id`, `title`, `date`, and `status`. The CLI supplies this metadata and the title heading. The ID prefix determines the document kind, and the filename retains its creation-time slug if the title later changes.

RFC bodies use Context, Requirements and constraints, Options, Design, and Adoption in that order. Context starts with a short proposal summary and explains the problem and scope. Put functional requirements in a `### Requirements` table, adding non-functional requirements only when relevant. Add a separate `### Constraints` table only for genuine hard boundaries. An optional Open questions section may follow Adoption.

An optional References section comes last. Link related ADRs and RFCs by ID using relative Markdown paths, explain each relationship, and include external sources when relevant. An RFC may link to broader or focused RFCs instead of duplicating requirements.

Functional requirements use unique `FR-001`-style IDs, and non-functional requirements use `NFR-001`-style IDs. Both use priority (MUST, SHOULD, or MAY). Constraints use unique `C-001` IDs. Options includes candidate descriptions and one comparison table with criteria in rows and options in columns, including No change. Requirement cells use `Does not meet`, `Partially meets`, or `Fully meets`, followed by ` - ` and evidence. Constraint cells use `PASS` or `FAIL`, followed by ` - ` and evidence. The chosen option must fully meet each MUST requirement and pass every constraint. Explain the choice using criterion IDs rather than an aggregate score.

New records start as drafts. Draft RFCs require Context, Requirements and constraints, and Options, including at least one functional requirement and a comparison table. Draft comparison cells and the chosen option may remain blank, and Design and Adoption may be omitted. Before promotion to proposed, complete the five sections, assess every criterion for every option, select an eligible option, and replace template placeholders. Proposed records may return to draft for rework or proceed to accepted or rejected. Drafts may also be rejected. Accepted records may become deprecated or superseded.

Design describes only the chosen option, including behavior, examples, and failure cases. Add a Risks and mitigations subsection when material risks need explanation. Adoption covers rollout, compatibility, and verification, with migration or rollback when relevant.

### Risks and mitigations

Comparison labels require supporting evidence; the CLI checks structure and eligibility but cannot judge whether an assessment is true. Reviewers must examine the evidence before accepting a proposal.

## Adoption

Initialize a repository with the CLI, which creates the accepted format RFC and adoption ADR. Create drafts from the bundled template and run `makespec check` after edits. Complete a draft before promoting it to proposed, then request acceptance after review. Verify adoption by checking the seed records and exercising the draft-to-proposed-to-accepted workflow.