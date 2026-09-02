---
title: 0.8.25 Slice 55 — tracing, explanation, and integrity design
status: REVIEWED_BLOCKED_ON_SLICE_7
design_version: 4
target_release: 0.8.25
depends_on: 50
readiness_blocked_on: Slice 7 architecture activation
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 55 — tracing, explanation, and integrity design

## Authority and comparison

Owns R25/AC25-55, Memex needs 19/20 and its share of 12, N25-02, and
A25-05/A25-06. It explains structural visibility/integrity, never semantic
truth, entailment, relevance correctness, or answer quality.

| Need | Existing | Decision |
| --- | --- | --- |
| Reciprocal lineage | IDs/explain/status/orphan checks | Bounded source↔derived trace with replayable persisted BFS pages. |
| Selection reason | Per-hit contributions | Typed ineligible/not-selected/unavailable/degraded reasons. |
| Integrity | Separate structural checks | Frozen bounded jobs and closed Engine-owned repair actions. |
| Safe repair | Forward dependencies are authority | Boundary-bound shadow reverse-index generation with verified atomic cutover. |

Accepted 0.8.8 explain/telemetry designs are preserved/reused. This is their
dependency/lifecycle successor. Erasure/projection/doctor/lifecycle designs
remain substrates, not semantic validators.

## Trace and explanation contract

```text
TraceDirection = to_sources | to_dependents
TraceRequestV1 { schema_version: 1, root, direction, max_depth: 0..8,
  max_nodes: 1..10000, max_edges: 1..50000,
  max_work_units: 1..100000, page_size: 1..1000, context }
TraceNodeV1 { schema_version: 1, artifact_id, artifact_class,
  lifecycle_state, visible, depth }
TraceEdgeV1 { schema_version: 1, dependency_id, from, to,
  liveness_rule, active }
TracePageV1 { schema_version: 1, nodes, edges, next_cursor?,
  complete, truncated, truncation_reason?, snapshot }
SelectionReasonV1 = included | ineligible | not_selected | unavailable | degraded
StructuralExplanationV1 { schema_version: 1, operation_id,
  query_correlation_id, artifact_id?, selection_reason, contributions,
  projection_origins, dependency_status?, lifecycle_status?, fallback?,
  degradation_codes }
```

`start_provenance_trace` freezes exact context and creates a lease.
`continue_provenance_trace(input_cursor, context)` requires exact context.
`explain_visibility` requires a concrete ID or bounded execution receipt; it
does not enumerate a rank-excluded corpus.

## Deterministic replayable trace continuation

Add `_fathomdb_trace_leases`, `_fathomdb_trace_frontier`,
`_fathomdb_trace_visited`, and `_fathomdb_trace_pages`. A lease stores version,
root/direction/depth/caps, snapshot/generation/context digests, next ordinal,
work count, and expiry no later than snapshot. Frontier ordering is
`(depth, artifact_revision_id, dependency_set_revision_id, ordinal)`; visited
keys opaque artifact revision ID.

The authenticated input cursor binds lease ID and `input_ordinal` plus root,
request/caps, exact context, generations, and ordering. Immutable page rows are
keyed `(trace_lease_id, input_ordinal)` and store canonical response bytes/hash
plus next cursor.

On continuation, the single writer first returns an existing page for that key
without advancing. Otherwise one transaction reads at most `page_size` items,
advances frontier/visited, creates the next cursor, and stores immutable page
and ordinal atomically. Only the returned next cursor advances. Lost responses
and concurrent duplicates replay exact bytes. Pages remain until lease expiry
and are erased/pruned with the lease.

Queue empty returns complete/non-truncated. A node/edge/work cap returns
incomplete/truncated with named reason and no cursor. Ordinary page boundaries
return incomplete/non-truncated with cursor. Eligibility applies before
enqueue; incomplete work is never labeled complete.

Explanation joins bounded evidence receipt, generation/work,
dependency/lifecycle, and fallback telemetry. Top-level reason is exclusive:
hard pre-cap exclusion, eligible non-selection, unavailable state, or degraded
fallback; details may list degradation codes.

## Frozen integrity job contract

```text
IntegrityCheckKind = dependency_endpoints | dependency_reverse_index |
  dependency_liveness | active_searchable_orphans | projection_generation |
  mutation_readiness
ReverseIndexBindingV1 { schema_version: 1,
  active_reverse_index_generation_id?,
  authoritative_forward_set_boundary,
  authoritative_forward_set_sha256 }
IntegrityBoundaryV1 { schema_version: 1, database_id,
  canonical_write_boundary, effective_valid_at,
  liveness_view: strict_current, projection_generation_ids,
  reverse_index: ReverseIndexBindingV1 }
IntegrityCheckRequestV1 { schema_version: 1, checks,
  max_findings: 1..10000, max_work_units: 1..1000000 }
IntegrityJobStatusV1 { schema_version: 1, job_id, boundary,
  state: queued|running|complete|incomplete|failed,
  checked, finding_count, next_cursor?, reason? }
IntegrityFindingV1 { schema_version: 1, job_id, boundary,
  finding_id, code, severity, artifact_revision_ids,
  projection_generation_id?, reverse_index_generation_id?, repairability }
RepairActionV1 =
  RebuildProjectionGeneration { projection_generation_id, source_boundary }
  | RegenerateDependencyReverseIndex {
      found_generation_id?, expected_forward_set_boundary,
      expected_forward_set_sha256 }
  | ResumeClosure { operation_id }
RepairPlanV1 { schema_version: 1, plan_id, boundary,
  finding_ids, actions, digest }
RepairReceiptV1 { schema_version: 1, plan_id, boundary, digest,
  accepted_actions, refused_actions, projection_work?, closure_work?,
  reverse_index_work?, resulting_write_boundary }
```

Job creation mints/persists one `IntegrityBoundaryV1` in a single Engine
transaction. It captures database/canonical boundary, one UTC validity instant,
Slice 20 strict liveness, applicable Slice 40 generations, the active reverse-
index generation, and SHA-256 of canonical active forward dependency-set rows
through their exact authoritative boundary. Canonical digest input is sorted
`(set_id,set_revision_id,dependent,member,rule,lifecycle)` wire data with schema
version and length prefixes; no payload or semantic metadata participates.

`active_reverse_index_generation_id` is null only when opening a pre-substrate
database or when a missing-substrate integrity finding is the job's subject.
Once the substrate exists, absence is drift/unavailable, never a new binding.

The complete boundary is persisted verbatim on job, status, every finding,
plan, action context, and receipt. Every dependency/reverse-index page must
reproduce the same active generation, forward-set boundary, and digest before
and after its page read. Any change/absence ends the job `incomplete` with
`integrity_reverse_index_drifted|unavailable`; no page combines bindings. An
existing job is never rebound, even when the named generation remains retained.
A new job after activation receives the new active generation and new forward
boundary/digest.

A repair plan names the identical boundary. `RegenerateDependencyReverseIndex`
also names the found generation (including explicit null) and exact expected
forward boundary/digest. Acceptance revalidates operator authority plus every
boundary component; generation, forward boundary, or digest change rejects the
whole plan `RepairPlanStale` before work is minted. Findings from one binding
cannot repair another.

Durable job/progress/finding tables page by stable opaque revision key and stop
at caps. Projection rebuild enqueues Slice 40 work. Closure resume calls Slice
30. No action creates/deletes dependencies, changes caller/canonical lifecycle,
restores bytes, or repairs semantic truth.

## Reverse dependency index generation and cutover

Add `_fathomdb_reverse_index_generations` with states
`building|verified|active|failed|retired`, generation-scoped shadow rows, and
append-only state events. Forward Slice 20 sets remain authority. An accepted
repair mints `building` from the action's frozen forward boundary/digest and
scans those forward rows in bounded stable-key pages.

Every later dependency mutation updates current active and every building
shadow in one writer transaction. Shadow rows carry
`last_applied_write_boundary`; boundary-scan upserts cannot overwrite later
dual-writes. Deletes use ordered generation-scoped tombstones.

After scanning, one writer transaction applies remaining ordered rows, computes
the current authoritative forward boundary/digest, verifies reciprocal
row/key/digest equivalence at that cutover, records `verified`, activates the
new generation, retires old active, and records `activated`. The resulting
receipt names the original immutable integrity boundary and the distinct
cutover boundary/digest; it does not mutate/rebind the originating job. Failure
records `failed` and leaves old active. If old active is the finding under
repair, reverse reads fail closed until activation; otherwise they continue on
old active. A partial shadow never serves.

Restart resumes `building` from persisted scan key and ordered rows. Failed
state remains diagnostic until bounded cleanup. Retired state remains while a
trace/integrity lease binds it, then prunes. Receipts expose building, verified,
activated, failed, and retired events with boundaries/digests, no caller content.

## Wire, privacy, lifecycle, tests

All objects inherit Slice 15 wire/unknown/u64/error/SDK/Windows/registry rules.
Failures include trace cursor/limit, artifact invisible, explanation
unavailable, integrity boundary/reverse-index drift/incomplete, stale repair,
and unsupported repair. Current access/lifecycle/erasure is rechecked on trace
replay and fails non-disclosing. Stored state contains no query/source bytes/
predicate values/payloads/low-entropy natural keys; erasure scrubs prohibited
references.

Tests cover lost-response/same-cursor replay, BFS/caps, explanation reasons,
multi-page mutation drift, reverse generation switch/absence, forward boundary
and digest changes, explicit pre-substrate null, no job rebinding, stale plan/
action refusal, shadow scan/dual-write/delete races, verified cutover, old-index
fail-closed reads, crash/restart/cleanup, privacy, and erasure. Golden Rust/
Python/TypeScript/wire, Windows CPU/native, operator, and registry smokes pass.

Routes: fast/heavy/all/all-feature/operator, Windows Rust/Python/Node, registry
explanation/operator. CUDA/live-model are N/A. Status remains `REVIEWED_BLOCKED_ON_SLICE_7`,
blocked on Slice 7 and 20/30/35/40/45/50; no implementation-shaping decision
remains.
