---
id: ADR-0004
title: Use Markdown records as the source of truth
date: 2026-11-09
status: accepted
---

# ADR-0004: Use Markdown records as the source of truth

## Context

Developers need to read, edit, review, and version records with ordinary Git and Markdown tools. A database or generated catalog must not become a second source of document content.

## Decision

Use Markdown files with YAML front matter as the source of truth for record content and metadata. Store id, title, date, and status in front matter, with optional superseded_by and unrelated user fields. Derive kind from the ID prefix.

Keep status and date only in front matter. Do not duplicate kind, slug, or schema_version fields. Add a matching ID/title heading, and preserve unrelated YAML fields and body bytes during CLI status updates.

Parse metadata and Markdown structure rather than interpreting document contents as executable instructions. Use separate cached state only for ID allocation, not as a replacement for records.

## Consequences

Humans and agents can inspect the complete records without a running service or database. Git retains their review history.

Authors can make invalid external edits, so check must report structural errors. Canonical specification tables constrain Markdown authoring, and structural validation cannot establish whether an assessment is true.

## References

- [Document rules](../../src/core/document.rs) define metadata, parsing, and rendering.
- [Specification template](../../.agents/skills/makespec/references/specification-template.md) defines canonical specification structure and comparison labels.
- [ADR-0001: Use Makespec to manage decisions and specifications](0001-use-makespec-to-manage-decisions-and-specifications.md) adopts Makespec and the two record types.