---
title: 0.8.25 Slice 30 — lifecycle and erasure closure design
status: REVIEWED_BLOCKED_ON_SLICE_7
design_version: 1
review_fix: 2
depends_on: 25
---

# Slice 30 design

## Authority and predecessor disposition

Implements R25/AC25-30, N25-01/N25-02, Memex needs 5/6, and A25-04/A25-05.
Existing lifecycle/erasure/projection-registry paths are reused. Historical
lifecycle and erasure records remain unchanged; this is their dependency-
closure successor. READY remains blocked on Slice 7/review.

## Dependency-stage contract

```text
ClosurePhase = planned | barriered | propagating | projections_pending |
               proving | complete | incomplete
ClosureStatusV1 { schema_version: 1, operation_id, root_action,
  admitted_boundary, liveness_view, phase, processed_count, pending_count,
  barriers, state_changes, dependency_consequences, projection_work_intents,
  erasure_scope?, blockers, proof? }
ClosureProofV1 { schema_version: 1, operation_id, proof_write_boundary,
  active_orphan_count: 0, searchable_orphan_count: 0,
  projection_orphan_count: 0, post_plan_dependency_count: 0,
  journal_payload_match_count: 0, wal_complete, checks }
```

This slice uses Slice 25 boundaries/projection intents only. Reactivation is the
existing transition to active, requires live dependencies, and cannot restore
erased bytes.

## Liveness versus physical erasure

Ordinary lifecycle closure uses Slice 20 strict liveness. Physical erasure is
stricter: any caller-authored artifact possibly containing erased-source
material is barriered and inactivated/erased regardless of `any_surviving`.
Only a registered Engine-owned source-separable projection may remove one source
component and remain. A caller artifact can return only as a new external
revision with clean bytes/dependencies. Removed members remain non-content
tombstoned audit links; the same body is never silently detached.

## Barrier admission and conservative read guard

Slice 25 atomically persists root/source barriers with closure intent. Every
write, dependency registration, actuation, projection, and lifecycle path
rejects new reference/derivation from a barriered root/source.

From barrier commit until proof retires it, every governed canonical, list,
search, FTS, vector, graph, projection, and evidence read applies an Engine
`closure_visibility_guard` **before** candidate/seed/frontier truncation. Using
indexed Slice 20 reverse dependencies, an artifact is ineligible when its
active dependency ancestry reaches a barriered artifact/source, whether or not
a work row has been materialized. The guard is cycle-safe, bound to the read's
strict effective instant, and has no permissive limit: storage error, missing
index, traversal-resource exhaustion, or inability to prove absence fails the
entire read `closure_visibility_unavailable`. SDK/post-filter emulation is
forbidden. The guard remains until zero proof/atomic barrier retirement.

Propagation uses durable frontier rows keyed by `(operation, artifact, action)`
and processes at most 1,000 per transaction to a fixed point. Total impact is
not capped. Resource exhaustion records resumable incomplete and retains
barriers/guard; operator resume cannot lift them. Each discovered artifact is
also directly fenced.

## Semantic-operation journal erasure

Before source/revision erasure propagation, use Slice 25's reference index to
find every nonterminal operation that may contain matching bytes. Under the
single writer, deterministically recover it to committed/refused terminal state
before the erasure advances. Terminalization strips request bytes atomically.
If recovery cannot finish, erasure becomes `incomplete`, retains its barrier and
visibility guard, and reports `journal_recovery_required`; it never deletes
recovery state or claims success.

Terminal receipts/reference rows contain only the approved non-content
idempotency/audit minimum. Erasure removes/redacts prohibited references under
the existing audit policy. Completion scans journal payload/blob columns and
reference indexes, and proves no matching source/revision bytes or prohibited
content remains. Raw byte canaries cover admitted, recovery-required, committed,
and refused operations plus WAL. The journal participates in the same proof
boundary as canonical/dependency/projection/telemetry stores.

## Proof, invariants, and failures

After propagation, drain projection intents and run raw canonical, dependency,
FTS/vector/graph, evidence/telemetry, semantic-journal, and WAL checks at a later
write boundary. Proof checks no post-admission dependency exists. Only zero
proof atomically marks complete/retires barrier; failure retains it. Empty work
queue alone is not completion.

No impacted artifact is visible after barrier admission, including last-page
undiscovered descendants. Failures include closure/boundary/barrier conflict,
visibility unavailable, journal recovery required, dependency rule, inactive
reactivation, erasure/projection/proof incomplete, and resource exhausted.

## Tests and verification

Tests cover all transitions/rules, physical-erasure cases, source-separable
projection, clean replacement, audit tombstones, >1-page closure with immediate
query of last undiscovered descendant, guard failure-closed paths, write races,
resource resume/restart, journal admitted/recovery/terminal payload stripping,
raw receipt/reference/WAL canaries, and later-boundary proof. Run fast, heavy,
all/all-feature/operator, Windows SDK/native, and installed; CUDA/model N/A.
