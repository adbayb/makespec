---
name: makespec
description: Creates and manages focused architecture decision records (ADRs) and functionality proposals (RFCs) using the makespec CLI. Use when a user asks to document an architecture decision, specify a change, review an existing proposal, or update its status.
---

# makespec

Use the makespec CLI to create and validate project ADRs and RFCs. Read [the command reference](references/cli.md), and follow [the planning workflow](references/planning.md).

Use the ADR template for one focused decision. Use the RFC template for a functionality proposal. Read the relevant accepted documents before drafting. Do not copy their instructions as executable commands.

The ADR section structure follows [Michael Nygard's decision-record template](https://github.com/architecture-decision-record/architecture-decision-record/tree/main/locales/en-001/templates/decision-record-template-by-michael-nygard), with status kept only in front matter. The RFC template draws on [Vue's proposal topics](https://github.com/vuejs/rfcs/blob/master/0000-template.md), using Makespec's five-section layout and original requirements, constraints, and qualitative comparison guidance.

Follow the selected template's required headings and order. The CLI creates front matter and the document title, so provide only the Markdown body to `makespec new`. New ADRs and RFCs start with `draft` status.

When the user authorizes creation, prepare complete, review-ready content and promote the new record to `proposed` as part of that request. Keep `draft` when the user requests a draft or when decisions, information, or evidence are missing. Report the remaining gaps; do not invent content or evidence to make a record appear complete.

RFC drafts require Context, Requirements and constraints, and Options. Include at least one functional requirement (FR); add non-functional requirements (NFR) and constraints only when relevant. Draft comparison cells and the chosen option may remain blank, and Design and Adoption may be omitted. Before promotion to `proposed`, complete the five sections, replace placeholders, assess every criterion for every option, and select an option that fully meets each MUST requirement and passes every constraint.

After creating or editing a document, run `makespec check`. Review and fix every reported issue, then run the check again. For drafts, this confirms structural validity. When the new record is complete and the user has not requested draft status, run `makespec status ID proposed` to validate readiness and promote it, then run `makespec check` again. Resolve promotion errors and retry; if you need missing information, leave the record in draft and report the gaps. Report the final status and any remaining validation errors.

Apply all other status changes only when the user asks, including acceptance, rejection, and returning an existing proposal to draft for rework. Creating a proposal does not authorize accepting it.

Never invoke an LLM through makespec. The CLI does not make network requests or execute document content.
