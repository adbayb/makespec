---
id: ADR-0001
title: Use decisions and specifications
date: 2026-11-09
status: accepted
---

# ADR-0001: Use decisions and specifications

## Context

Project changes need two levels of design records: focused decisions and broader functionality specifications. A single format makes small choices hard to find and forces feature specifications into an unsuitable structure.

## Decision

Use decisions for small, focused choices and specifications for functionality or other substantial changes. Decisions follow the architecture decision record (ADR) format; specifications follow the request for comments (RFC) format and include product requirements documents (PRDs). Keep both as Markdown files in `docs/decisions` and `docs/specifications`, respectively. Give each kind its own incrementing ID, creation date, and front-matter status. Use a specification when a change needs requirements, constraints, alternatives, and an adoption plan; record a contained choice as a decision.

## Consequences

Reviewers can trace specifications and decisions in Git. Authors must keep status and structure valid, and specification authors must compare every option against the same criteria. The repository gains a small amount of documentation overhead, but does not need a service or database.