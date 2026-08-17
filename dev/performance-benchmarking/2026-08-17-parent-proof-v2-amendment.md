# PARENT-01 canonical parent-proof v2 amendment

**Tracks:** `PARENT-01`, `LOCOMO-01`  
**Date:** 2026-08-17  
**Authority:** direct HITL instruction to complete authorized work.

## Problem and replacement

The v1 parent proof treated raw LOCOMO turn identifiers as globally unique.
They are not unique across their enclosing conversation/session scopes, so v1
correctly blocked qualification.

The replacement `locomo-parent-relation-proof.v2` derives a content-free child
identity from the ordered tuple of conversation identifier, session identifier,
and raw turn identifier. The derived identifier is a SHA-256-based stable
identifier; it does not embed the raw turn identifier. The adapter uses the
same identity for turn ingestion, evidence binding, PARENT ranking, and
bounded-neighbor attribution.

## Binding and qualification

This is a representation amendment, not a corpus or treatment change. It
retains the frozen corpus, turn provenance, session provenance, dry-run subset,
metric, and CPU/GPU pins. Its factual qualifier report is qualified with no
blockers and SHA-256
`de7e968f78f4c46dc7e86c4e40e9687ae1f3b64b7d8d650cb1935c76fcc685d2`.

The externally derived v2 proof has 5,882 entries and SHA-256
`cfa38b5ad09c7e74bbcc1103a51635fcaede4028e91301c1f901c4cdc77ebc30`.
The associated TRACE projection SHA-256 is
`8797b79975b71fa21894377837d15151d21bc49e3f1604ff69649d857abaf6fa`.

## Required gate

This amendment does not authorize a run by itself. The qualified implementation
and the exact external adapter copy require independent review, then a
coordinator-issued release that binds the new proof and its reviewed hashes
before the five-cell dry run.
