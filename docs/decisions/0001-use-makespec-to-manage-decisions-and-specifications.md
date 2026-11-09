---
id: ADR-0001
title: Use Makespec to manage decisions and specifications
date: 2026-11-09
status: accepted
---

# ADR-0001: Use Makespec to manage decisions and specifications

## Context

Contributors need a shared way to document proposed functionality and preserve the reasoning behind focused choices. Humans and coding agents need to find and review these records in the repository.

## Decision

Use Makespec to create, validate, and manage the lifecycle of decisions and specifications stored as Markdown. Makespec relies on the architecture decision record (ADR) format for focused decisions and the request for comments (RFC) format for functionality specifications, including product requirements documents (PRDs).

Keep decisions in `docs/decisions/` and specifications in `docs/specifications/`. Contributors may write and edit content with ordinary editors or agent tools.

## Consequences

Contributors gain stable references and a shared lifecycle. They must keep records current and validate changes with `makespec check`. Makespec becomes a contributor tool, but reading the records does not require the binary. Contributors can review and version records in Git.

Follow the installed skill, templates, and command reference for authoring and review rather than maintaining a separate workflow specification.

## References

- [Contribution workflow](../../.agents/skills/makespec/SKILL.md#workflow) defines authoring, planning, and approval boundaries.
- [Decision template](../../.agents/skills/makespec/references/decision-template.md) and [specification template](../../.agents/skills/makespec/references/specification-template.md) define record structure.
- [Command reference](../../.agents/skills/makespec/references/cli.md) defines syntax and lifecycle transitions.