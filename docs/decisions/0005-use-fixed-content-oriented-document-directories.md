---
id: ADR-0005
title: Use fixed content-oriented document directories
date: 2026-11-09
status: accepted
---

# ADR-0005: Use fixed content\-oriented document directories

## Context

Agents and developers need predictable locations for decisions and specifications. Project configuration and custom path flags would introduce additional layouts that tooling must discover and preserve.

## Decision

Store decisions in docs/decisions/ and specifications in docs/specifications/. Use the full content-oriented names in directories, CLI kinds, and documentation so readers do not need to know the ADR and RFC acronyms to use Makespec.

Run commands from the project root and install the skill under .agents/skills/makespec/. Do not require a project configuration file or expose custom document-directory flags. Use decision/specification as the only CLI kinds, without aliases. Keep ADR-/RFC- ID prefixes to identify the underlying formats.

## Consequences

Humans and agents share one layout, and initialization needs no configuration choices.

Repositories cannot customize managed locations. A future layout change requires an explicit compatibility decision.

## References

- [Directory mapping](../../src/core/document.rs) defines each kind's managed directory.
- [CLI adapter](../../src/startup/cli.rs) uses the current directory as the project root.
- [Embedded skill assets](../../src/adapters/embedded.rs) define the installed skill files.