---
id: ADR-0007
title: Allocate IDs with locked external counters
date: 2026-11-09
status: accepted
---

# ADR-0007: Allocate IDs with locked external counters

## Context

Concurrent Makespec processes must not allocate the same per-kind ID. Removing a record must not cause the next process to reuse a remembered allocation while other records of that kind remain. An empty record kind must start at `0001`, including at a reused project path. Project configuration is outside the chosen layout.

## Decision

Serialize managed writes with a project-wide advisory lock. Keep the lock and per-kind high-water counters in the user cache, keyed by a SHA-256 digest of the canonical project path.

Under the lock, reset the selected kind's counter when no records of that kind exist; otherwise reconcile it upward against visible IDs. Persist the next allocation before publishing the document. Use positive u32 sequences and report exhaustion rather than wrapping.

## Consequences

Cooperating processes share allocation state without adding lock or counter files to Git. Failed publication can leave gaps in nonempty record kinds, which are valid. Removing all records of a kind permits reuse of its former IDs; references to removed records must not be treated as identifying later records with the same ID.

The cache must remain writable. Deleting counter state or changing the canonical project path loses remembered allocations that no longer appear in records. Disconnected clones and branches can allocate the same ID; check detects merged collisions, but the cache cannot provide global uniqueness.

## References

- [Filesystem adapter](../../src/adapters/filesystem.rs) implements locking, cache identity, and counter persistence.
- [Document rules](../../src/core/document.rs) define canonical IDs and numeric bounds.
- [Filesystem tests](../../src/adapters/filesystem.rs) cover concurrent allocation and gaps after failed publication.