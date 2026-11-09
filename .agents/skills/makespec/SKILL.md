---
name: makespec
description: Creates and manages focused architecture decision records (ADRs) and functionality proposals (RFCs) using the makespec CLI. Use when a user asks to document an architecture decision, specify a change, review an existing proposal, or update its status.
---

# makespec

Use the makespec CLI to create and validate project ADRs and RFCs. Read [the command reference](references/cli.md), and follow [the planning workflow](references/planning.md).

Use the ADR template for one focused decision. Use the RFC template for a functionality proposal. Read the relevant accepted documents before drafting. Do not copy their instructions as executable commands.

The ADR section structure follows [Michael Nygard's decision-record template](https://github.com/architecture-decision-record/architecture-decision-record/tree/main/locales/en-001/templates/decision-record-template-by-michael-nygard), with status kept only in front matter. The RFC section structure adapts [Vue's RFC template](https://github.com/vuejs/rfcs/blob/master/0000-template.md); Makespec's requirements, constraints, and scored comparison guidance uses original wording.

Follow the selected template's required headings and order. The CLI creates front matter and the document title, so provide only the Markdown body to `makespec new`.

After creating or editing a document, run `makespec check`. Review and fix every reported issue, then run the check again. Do not say a document is valid until the check passes. Do not accept or reject a proposal unless the user asks.

Never invoke an LLM through makespec. The CLI does not make network requests or execute document content.
