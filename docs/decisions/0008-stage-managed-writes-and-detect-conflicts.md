---
id: ADR-0008
title: Stage managed writes and detect conflicts
date: 2026-11-09
status: accepted
---

# ADR-0008: Stage managed writes and detect conflicts

## Context

Creation must not overwrite existing records. Status updates must detect external edits, and interrupted initialization must not remove user changes during cleanup.

## Decision

Stage managed content in a synchronized temporary file beside its destination. Publish new records without clobbering an existing destination.

For replacements, hold the project lock and compare the current bytes with the loaded version before staging and again before publication. Preserve path, ID, date, unrelated YAML fields, and document body during status updates.

For initialization, preflight destinations, stage content, and publish files without overwriting conflicts. Roll back only newly published files whose bytes still match the generated content, and report rollback failures. Ignore only regular .makespec-*.tmp staging files during discovery.

## Consequences

Makespec can reject creation conflicts and detected external edits without replacing the existing record. Rerunning initialization can complete a partial initialization.

Initialization is not a multi-file transaction. Advisory locks coordinate Makespec processes, not external editors; byte comparisons cannot eliminate all races with tools that ignore the lock. A failed operation must report remaining conflicts or cleanup errors.

## References

- [Filesystem adapter](../../src/adapters/filesystem.rs) implements staging, exclusive creation, replacement checks, and rollback.
- [Core workflows](../../src/core/service.rs) validate initialization and proposed status changes before publication.