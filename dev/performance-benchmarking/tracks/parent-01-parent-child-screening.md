# PARENT-01 — Parent-child retrieval screening

**Status:** active; `parent_child_turn_session_v1` is HITL-approved in the
[dated grid amendment](../2026-08-17-parent-01-v1-grid-amendment.md).
Implementation and execution remain uncommissioned.

## Decision

Does child retrieval followed by parent/session and bounded-neighbor context
improve evidence recovery and answer context over parent-only retrieval?

## Preparation and contract

1. Consume the accepted TRACE-01 source/lifecycle contract for the selected
   child and parent projections.
2. Implement the approved frozen treatment: individual-turn child, exact
   enclosing-session parent, hybrid top-10 child ranking, five deduplicated
   session bundles, rank-preserving parent selection, and one neighbor on each
   side within the session.
3. Extend LOCOMO provenance and metrics to report child evidence recall, parent
   recall, duplicate rate, context expansion, and per-class latency.
4. Write tests for parent mapping, duplicate removal, missing-parent handling,
   neighbor bounds, and receipt isolation from public API changes.

## Exit evidence

The treatment is eligible only if it clears LOCOMO-01’s retrieval and latency
rules without a material class regression. It becomes an ANSWER-01 candidate,
not a product default, only after that evidence exists.
