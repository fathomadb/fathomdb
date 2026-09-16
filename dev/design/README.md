---
status: ACTIVE
---

# Design documents

This directory holds current subsystem designs, bounded references and
experiments, proposals, and historical design records. The exact lifecycle
classification and current owner for every Markdown document is
[`document-lifecycle.json`](document-lifecycle.json). The catalog is checked for
complete coverage, valid ownership, and unique maintained topic/role pairs by
`scripts/check-design-lifecycle.py` in both the local Markdown gate and docs CI.

## Current owners

The maintained design owners include the data-plane architecture and the active
engine, retrieval, vector, projection, scheduler, recovery, migration, binding,
error, actuation, release, performance, and operating-method designs. Discover
the exact set by filtering the catalog for `"class": "maintained"`; do not copy
that list into another hand-maintained index.

The active architecture profile is
[`fathomdb-data-plane-architecture-v2.md`](fathomdb-data-plane-architecture-v2.md).
Public contracts remain owned by `dev/interfaces/` and accepted decisions by
`dev/adr/`; those authorities override explanatory design prose.

## Lifecycle rules

- `maintained` is a current topic/role owner and must stay consistent with code,
  interfaces, and accepted ADRs.
- `reference` and `experiment` remain useful inputs but do not own current
  product behavior.
- `proposal` and `deferred` describe unshipped work; they must not be read as
  current behavior.
- `historical` records a closed slice or superseded release context.
- `superseded` must name its current successor, which is also its owner.

Adding, removing, moving, or reclassifying a design document requires updating
the lifecycle catalog in the same change. Historical records are preserved; a
new current owner supersedes them without rewriting their original decisions.

## Placement

Keep internal subsystem and cross-cutting design here. Put public operator or
SDK guidance under `docs/`, accepted decisions under `dev/adr/`, and disposable
ideation outside the maintained design surface.
