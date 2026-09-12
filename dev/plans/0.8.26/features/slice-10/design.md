---
title: FathomDB 0.8.26 Slice 10 — frozen evidence repair design
status: DRAFT
---

# Slice 10 design — frozen evidence repair

## Design

Frozen search must traverse the same response-completion routine that assigns
and validates ordinary-search correlation identity before binding-specific
models are constructed. The change belongs after result assembly and before
return, not in scoring, eligibility, or evidence-handle generation.

No new public type or persisted format is expected. The regression suite
compares explanation-off and explanation-on results after removing only the
fields whose presence is requested, proving that hit order, scores, IDs,
evidence handles, and frozen boundary are unchanged.

## Public guidance shape

| Need | API path |
| --- | --- |
| frozen ranked results without evidence exposure | `freeze_read_context` then `search_frozen` |
| frozen ranked results that may support an answer | `freeze_read_context` then `search_with_evidence`, then `resolve_evidence` for selected hits |
| exact graph artifact evidence | Slice 20 point-resolution path, not body re-search |

The guide explains handle authority, lifecycle/eligibility refusal, snapshot
expiry, pagination, correlation, and why non-frozen fallback is invalid.

## Installed-artifact witness

The witness installs freshly built packages in a clean environment, opens a
real database, exercises the one-source canonical/derived/dependency path,
restarts, resolves frozen evidence with explanation, pages and looks up the
dependency, checks projection readiness, and closes cleanly. It is a FathomDB
contract witness, not a copy of Memex policy.
