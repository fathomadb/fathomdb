---
title: FathomDB 0.8.26 Slice 15 — graph-evidence impact design
status: APPROVED
reviewed_on: 2026-09-13
review_result: independent PASS after one focused correction cycle
---

# Slice 15 design — graph-evidence impact spike

## Existing substrate and net-new prototype

| Concern | Exists today | Test-only Slice 15 delta |
| --- | --- | --- |
| graph selection | deterministic bounded traversal; target and winning edge cursors exist during selection | retain the winning edge cursor in an internal candidate until final truncation |
| revision registry | node/edge revisions; indexed `(artifact_class, write_cursor)` lookup | two bounded post-selection joined evidence-preflight statements |
| evidence authority | authenticated, database/context/artifact-bound search references | graph-disclosure-bound prototype reference |
| materialization | primary-connection transaction, frozen checks, provenance/lifecycle/hash/dependency validation | intrinsic point result without rank/contribution claims |
| erasure proof | before-resolve-return hook and fail-closed WAL checkpoint behavior | reuse the hook for graph-point resolution race proof |
| public graph API | current V1 request/result, target, codecs, and bindings | no retained public change in Slice 15 |

No schema, migration, evidence table, cache, reader-pool route, or persistent
sidecar is required.

## One treatment, two policy alternatives

The prototype exposes one internal treatment toggle around post-selection
hydration and reference minting. It does not implement competing public APIs.

- **A, optional sidecar:** ordinary calls execute the control; evidence calls
  execute the treatment. Only opted-in responses pay hydration and carrier
  cost.
- **B, required target fields:** every call executes the same treatment and
  every target response grows. Binding and fixture blast radius is counted
  from the existing maintained surfaces rather than implemented three times.

Transient canonical encoders measure A's sidecar carrier and B's inline target
carrier separately. A checked-in manifest counts affected maintained Rust,
Python, and TypeScript types, codecs, fixtures, and documents. Workload models
report `A = control + p × incremental treatment` and
`B = control + incremental treatment` for evidence-request fractions 0, 0.01,
0.1, 0.5, and 1.0. Because graph evidence requires frozen authority, B would
either reject today's current-context graph calls or create mixed semantics in
which required identity fields are not always resolvable. That consequence is
part of the HITL comparison.

This isolates the runtime difference relevant to D26-01. It also permits a
literal baseline-byte oracle: the disabled control must reproduce the exact
pre-spike encoded response, not merely equal another newly encoded control.

## Post-selection evidence preflight

The internal candidate is `{ target, terminal_edge_cursor }`. Candidate
replacement carries the edge cursor associated with the deterministically
winning origin. Sorting and truncation remain unchanged. Only the selected
targets are hydrated:

- node query: `artifact_class='node'` and at most 50 write cursors;
- edge query: `artifact_class='edge'` and at most 50 non-null terminal-edge
  write cursors.

Each class-specific statement starts from the existing unique revision-
registry index and joins the source link, source revision, canonical source
record, lifecycle, locator, and hash facts needed to decide whether a usable
reference may be minted. It accepts at most 50 cursors. Canonical source bytes
are hashed and locators validated once per distinct source revision in memory,
then shared across selected artifacts backed by that source.

Missing, duplicate, wrong-class, incomplete, hash-invalid, locator-invalid, or
source-ineligible rows refuse the evidence treatment as a whole. Preflight is
`O(result_limit)` in selected artifacts and bounded distinct sources, never
`O(work_units)`. `EXPLAIN QUERY PLAN`, input counts, recorded returned rows and
bytes, and a 10,000-work-unit fixture prove the claim without adding production
SQL counters. These complete preflight costs—not revision lookup alone—enter
the A/B measurements.

## Identity, authority, and class-aware eligibility

Each prototype sidecar entry carries stable target and terminal-edge revision
identity for joining plus opaque authenticated references for resolution. The
ID is not authority. Evidence-bearing expansion requires
`GraphReadContextV1::Frozen`; a current-context request refuses before
hydration.

The prototype reference binds the database, frozen-context commitment,
artifact class and immutable revision, disclosure role (target or terminal
edge), returned target position/revision, terminal-edge revision,
predecessor/target endpoints, traversal direction, semantic edge kind, and a
canonical commitment over graph schema, seed, direction, normalized
edge/target kinds, depth, result/work limits, and frozen context. Resolution accepts that
reference and an equivalent frozen context, never a bare or guessed revision
ID.

Target resolution rechecks the frozen node view and target eligibility.
Terminal-edge resolution authenticates that the edge was disclosed by the
committed graph operation, then rechecks frozen state, source access,
lifecycle, closure barriers, immutable identity, endpoint/direction/kind
coherence, and membership of the semantic kind under committed `edge_kinds`. It
does not rerun traversal or reinterpret the target-oriented
`SearchFilter.kind` as an edge-kind constraint. Missing, state-drifted,
out-of-window, erased, superseded, closure-
fenced, ineligible, foreign, mismatched, or nonexistent evidence collapses to
one nondisclosing unavailable result.

Ordinary writes whose registry metadata is `migrated_incomplete` cannot yield
evidence. An evidence-bearing graph call is all-or-nothing, matching the
existing ranked-evidence contract.

## Intrinsic resolution and erasure

The prototype result contains only intrinsic facts established by the
artifact and its canonical source: artifact class/revision, logical identity,
kind/body or edge endpoints, canonical bytes/span/locator, source
identity/version, hash, lifecycle, and direct dependency when applicable. It
does not contain ranking contribution, query score, projection generation, or
fabricated projection origin.

Resolution uses the primary connection and one transaction. It authenticates
and validates before materialization, fires the existing before-return test
rendezvous after bytes are materialized, revalidates the frozen snapshot, and
commits before returning. Primary-connection serialization prevents erasure
from reporting success while the resolver owns pre-erasure bytes. A separate
held WAL reader may still produce the existing typed
`ErasureIncomplete(stage="wal_checkpoint")`; committed erasure must remain
observable even in that outcome.

## Measurement design

Each timing comparison uses release-mode binaries, warmup outside samples,
balanced arm order, monotonic clocks, identical immutable fixtures, and raw
per-operation samples. Tiny-operation/RSS pairs run in isolated processes so
historical allocator state cannot contaminate the treatment.

The graph matrix covers 1 result, 50 results, 10,000 work units, and eight-
caller concurrent graph expansion. The point matrix covers node and edge
evidence at 1 KiB and 100 KiB, sequential and eight-caller loads, and a 50-
resolution Memex-shaped batch. Writer campaigns
compare writes alone, with repeated hydrated graph reads, and with point
resolution. At least five balanced writer campaigns support the trigger:
repeatable throughput loss over 10 percent or p99 growth over 25 percent.
Idle and resolver-held erase/excise latency receives at least 20 paired
observations in addition to the deterministic ordering oracle. Fixture setup
and WAL-holder setup occur outside timed regions.

Record p50/p95/p99, throughput, response bytes, process RSS, indexed plans,
fixture bytes copied/hashed, WAL/checkpoint state around erasure, and erasure
latency/outcome. End-to-end writer latency is the mutex-contention proxy. Do
not add direct production instrumentation for SQL counts, mutex wait, or byte
copies.

Arm order is balanced within paired campaigns. Decisions use paired ratios and
median/interquartile range across campaigns, with deterministic-bootstrap
95 percent intervals for treatment/control ratios. Percentiles use the
nearest-rank estimator. A writer trigger is repeatable
only when the median ratio crosses its threshold and at least four of five
campaigns agree in direction. Tail percentiles from fewer than 1,000 samples
are labeled descriptive rather than inferential. Raw samples remain available
so HITL can distinguish absolute sub-millisecond effects from large relative
percentages.

The existing AC-081a/b identity-bound observation and AC-081c concurrency
oracle are sentinels, not newly invented acceptance criteria. Entering the
AC-081 warning band triggers its established campaign before attribution.
Cross-platform/package matrices remain Slice 50 work.

## Retention boundary and HITL output

Prototype RED/GREEN tests live in prototype-only files and are explicitly
transient because their subject is removed. Retain durable raw measurements,
derived analysis, the literal
ordinary-byte oracle, characterization tests for baseline selection/index/
erasure invariants, and only behavior-neutral reusable measurement fixtures.
Before close, delete those transient test files unchanged together with the provisional
request/carrier, public/runtime resolver, reference encoding, codecs, binding
changes, and treatment plumbing. Run every final-tree gate after that removal.
This planned teardown is not an oracle edit. Fast-forwarding the full commit
chain onto the release branch before branch cleanup keeps the exact RED/GREEN/
measurement/removal chronology reachable;
final public-surface and schema inventories must match the Slice 15 baseline.

The status compares A and B on ordinary-path cost, opted-in cost, response
growth, implementation/binding blast radius, security authority, erasure
safety, and consumer ergonomics. It recommends A, B, a narrower replacement,
or deferral, but it does not mark D26-01 ruled or unblock Slice 20 by itself.
