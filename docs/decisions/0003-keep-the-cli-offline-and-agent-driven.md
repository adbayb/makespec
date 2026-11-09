---
id: ADR-0003
title: Keep the CLI offline and agent-driven
date: 2026-11-09
status: accepted
---

# ADR-0003: Keep the CLI offline and agent\-driven

## Context

Developers and coding agents need the same record creation and validation rules. Embedding model providers, credentials, editors, or a terminal UI would add separate configuration and interaction paths to the binary.

## Decision

Expose a non-interactive CLI with init, new, list, status, and check. Let agents prepare Markdown with their workspace tools and pipe document bodies to new. Install a skill that explains formats, planning, and approval boundaries.

Keep model-provider calls, editor launching, and terminal UI behavior outside the binary. Bare invocation prints help without reading stdin. New requires piped UTF-8 input and fails when stdin is a terminal.

Agents use accepted specifications and decisions to inform plans. Reading or accepting a record does not authorize implementation, and creating a proposal does not authorize acceptance.

## Consequences

The binary needs no model credentials or network permissions. Humans and agents share validation and lifecycle behavior.

Users need an editor or agent to author prose. Agent discovery depends on support for the installed skill directory and an available Makespec executable. The CLI cannot judge evidence or prose quality.

## References

- [CLI adapter](../../src/startup/cli.rs) defines commands and terminal-input behavior.
- [Makespec skill](../../.agents/skills/makespec/SKILL.md) defines agent workflows and approval boundaries.
- [Command reference](../../.agents/skills/makespec/references/cli.md) defines syntax and lifecycle transitions.