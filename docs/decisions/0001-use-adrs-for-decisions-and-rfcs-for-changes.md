---
id: ADR-0001
title: Use ADRs for decisions and RFCs for changes
date: 2026-11-09
status: accepted
---

# ADR-0001: Use ADRs for decisions and RFCs for changes

## Context

Project changes need two levels of design records: focused decisions and broader functionality proposals. A single format makes small choices hard to find and forces feature proposals into an unsuitable structure.

## Decision

Use ADRs for small, focused decisions and RFCs to specify functionality or other substantial changes. Keep both as Markdown files in `docs/decisions` and `docs/specifications`, respectively. Give each kind its own incrementing ID, creation date, and front-matter status. Use an RFC when a change needs requirements, constraints, alternatives, and an adoption plan; record a contained choice as an ADR.

## Consequences

Reviewers can trace proposals and decisions in Git. Authors must keep status and structure valid, and RFC authors must compare every option against the same criteria. The repository gains a small amount of documentation overhead, but does not need a service or database.