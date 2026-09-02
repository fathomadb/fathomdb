---
title: 0.8.25 Slice 25 — atomic semantic actuation design
status: REVIEWED_BLOCKED_ON_SLICE_7
design_version: 1
review_fix: 2
depends_on: 20
---

# Slice 25 design

## Authority and disposition

Implements R25/AC25-25, N25-01, Memex needs 3/17/18, and A25-05. Typed-write,
`PreparedWrite`, and `Engine.write` are reused internally. Provider-based
consolidation is historical reference; no provider runs here. Existing compact
receipts remain; this adds model-free actuation. READY stays blocked.

## Dependency-stage public contract

```text
ExpectedWriteBoundaryV1 { schema_version: 1, database_id,
                          canonical_write_boundary }
ResultingWriteBoundaryV1 { schema_version: 1, database_id,
                           canonical_write_boundary }
ProjectionWorkIntentV1 { schema_version: 1, projection_name,
  mutation_boundary, projection_cursor, terminal_state, work_kind }
SemanticBatchV1 { schema_version: 1, operation_id, policy,
  expected_write_boundary?, operations }
SemanticBatchReceiptV1 { schema_version: 1, operation_id, request_sha256,
  outcome, policy, admitted_boundary, resulting_write_boundary?, created_ids,
  affected_ids, state_changes, dependency_changes, projection_work_intents,
  closure_operation_ids, refusals }
Outcome = committed | committed_closure_pending | refused
```

Operations are canonical/derived/fact/edge writes, dependency put/retire,
transition, and caller-decided coexist/supersede/invalidate/merge. Slice 25 has
no frozen snapshot/generation type; Slices 35/40 extend additively. Merge writes
exact caller content/dependencies; no semantic provider runs.

## Crash-stable journal and erasure index

The operation journal stores ID, canonical digest, prepared request while
nonterminal, admission boundary, phase, and terminal receipt. A normalized
`semantic_operation_references` index records every source ID, source revision,
artifact revision, logical identity, and dependency-set revision referenced by
the request, keyed by operation ID. Journal and reference index updates share
the admission/terminal writer transactions.

First transaction reserves ID/digest and validates at the stored boundary.
Invalid requests store terminal refusal in that transaction; valid requests
store an admitted deterministic prepared plan. The next transaction applies
all domain rows and terminal receipt atomically. Engine open recovers admitted
plans before later writes. Same ID/digest replays terminal or returns
`operation_in_progress`; another digest conflicts.

On either committed or refused terminal transition, prepared request bytes and
any content-bearing diagnostics are zeroed/deleted in the same transaction.
Terminal journal rows retain only operation/digest, policy version, non-content
opaque IDs, phase, boundary, reason codes, and compact receipt. Reference-index
rows retain opaque non-content IDs for idempotency, audit, and erasure lookup.
Terminal rows are outside cap sweeps until explicit governed operation-record
erasure/database destruction. Nonterminal payloads remain inside Slice 30's
source/revision erasure boundary and cannot be removed before recovery resolves.

## Lifecycle staging and invariants

Every lifecycle batch creates a baseline closure intent and root/source barrier
regardless of size. Intent ID, roots, action, admission boundary, liveness
instant, and journal references commit atomically with domain mutation and
receipt. Direct roots fail closed; receipt is `committed_closure_pending` and
names closure IDs. Slice 30 processes intents.

Validation considers the prospective whole batch and allows earlier creates.
Any domain failure rolls back all domain state. Partial refusal is diagnostic
only. Failures include invalid operation/reference/transition, cycle,
write-boundary mismatch, idempotency conflict/in-progress, projection contract,
bounds, barrier conflict, and recovery required. Cap at 1,000 operations,
10,000 references, and existing payload limits.

## Tests and verification

Fault injection covers reservation, validation, admission, commit, reopen,
same-ID concurrency, terminal replay/retention, and reference-index atomicity.
Fixtures prove terminal payload stripping, non-content receipts, admitted
source lookup, and no deletion before recovery. Also cover forward references,
verdicts, cycles, boundaries/intents/barriers, codecs, SDK/Windows/installed
parity. Run fast, heavy, all/all-feature and registry; operator for recovery;
CUDA/model N/A.
