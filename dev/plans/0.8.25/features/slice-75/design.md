---
title: 0.8.25 Slice 75 — trimmed integrated closure design
status: DRAFT_SCOPE_RECONCILED_BLOCKED_ON_SLICE_7
design_version: 2
target_release: 0.8.25
depends_on: 60
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 75 design

## Authority and boundary

Implements the retained subset of R25/AC25-75, the measurement portion of
Memex need 21, needs 22/24, and the integrated audit portion of need 23. It
audits evidence already produced by Slices 10–60; it does not backfill missing
feature tests, repair a failed implementation, or tune a benchmark treatment.

The matrix is deliberately representative. Exhaustive scale-by-feature-by-
CUDA coverage, live-model routes, future profile/temporal treatments, and
full integrity orchestration are preserved after 0.8.25.

## Workload 0 — release-state-aware preflight

The preflight gate discovers the single `dev/plans/release-state-*.json` writer
for the active release and validates its release identity, board/plan paths,
ladder, completion/ref state, and requested prerequisite before evaluating a
feature worktree. Worktree freshness is relative to the release-state-declared
base and release tip, not a hard-coded historical release or current
`origin/main`. Dependency closure comes from the exact ladder entry and its
Git-verifiable SHA, not a prose grep.

The existing primary-checkout landing refusal, disk, mid-operation, board,
ledger, governed-surface, and transcript checks remain unchanged. Missing,
multiple, malformed, wrong-release, unverifiable-ref, non-descendant, or open-
prerequisite state fails closed. Canonical fixtures preserve both a completed
0.8.23 release and the active 0.8.25 release without embedding either version
in product logic.

TDD begins with fixtures reproducing the Slice 20 false rejection, then covers
every accepted and refused state above. Implementation review must verify that
the correction does not let a stale worktree or incomplete prerequisite pass;
independent verification runs the fixture suite and the real Slice 75
commissioning preflight.

## Manifest and receipt

`IntegratedClosureManifestV1` is a strict checked-in evaluation configuration.
It binds candidate commit/version, design and verification-record digests,
package hashes, fixture/config digests, platform/device requirements, exact
commands, workload cells, repetitions, policy/advisory thresholds, and allowed
N/A routes. Unknown or missing material fields reject before execution.

`IntegratedClosureReceiptV1` records each command/cell, actual package and
Engine method, read/projection identities, raw-output digest, errors/timeouts,
latency/resource summaries, and result state:
`passed | failed | insufficient_samples | missing_prerequisite |
environment_invalid`. Only `passed` supports a release claim. Raw partial data
is retained and never pooled into a complete result.

Every metric records `measurement_layer`, `engine_search_executed`, contributing
components, and shared/differing comparison components under Slice 10. The
receipt is evaluation data, not a public product API.

## Workload 1 — installed cross-SDK and wire conformance

Build locally packaged, registry-equivalent artifacts once per selected target
from the same candidate commit. In isolated consumers with no source-tree or
editable fallback:

- unpack/test the Rust crate and packaged CLI;
- install the hashed wheel in a fresh venv and prove Python/native paths;
- install packed npm plus matching native package and prove resolved paths;
- execute the retained Slice 15–60 success/error/unknown-version fixtures; and
- require semantic and canonical-wire equality across Rust, Python, and
  TypeScript after documented casing conversion.

Linux and Windows x64 CPU/native Rust/Python/Node evidence is mandatory.
Focused Linux CUDA runs only when a retained dense/rerank contract changed;
they capture device/runtime/model identity and allocation proof. Windows CUDA
is N/A. Actual PyPI/npm/crates.io installs are a separate post-publication gate
requiring explicit publish authorization.

## Workload 2 — representative concurrency and consistency

Run two preregistered cells against fresh databases:

1. 10,000 canonical records, one reader and one bounded writer;
2. 50,000 canonical records, 12 readers and one bounded writer.

Each has three process-cold and three steady repetitions. Steady repetitions
retain at least 500 operations per reader. A fixed cycle covers A0 search,
three-page canonical/state walks, evidence resolution, direct dependency
trace, compact integrity check, and constrained graph expansion. The writer
uses a checked-in deterministic trace of record, dependency, lifecycle,
erasure/recreate, and projection-readiness operations.

The manifest pins corpus/trace digests, seed, process/database reset, warm-up,
timeouts, no-runner-retry policy, shipped SQLite/runtime configuration, allowed
CPU affinity, and resource sampler. Report p50/p95/p99, throughput, writer
wait, typed outcomes, errors/timeouts, RSS, CPU, database/WAL size, open/close
time, and GPU only when applicable.

Every multi-call read either remains stable under its compact bound context or
returns its specified typed stale/unavailable outcome. Mixed visibility,
duplicate/omitted pages, stale evidence bytes, searchable erased dependents,
untyped busy/timeout, or a false-ready projection is a correctness failure.

## Workload 3 — focused lifecycle and overhead regressions

At 10,000 records, run paired fresh-database baseline/new-operation cells for
eligibility, optional frozen context, canonical/state pages, evidence create/
resolve, dependency write/trace, bounded actuation, projection status, basic
integrity, and constrained expansion. Report absolute and paired latency,
resource, and storage overhead. Retrieval-bearing pairs must also prove exact
identity/order semantics; speed alone cannot pass.

Run compact deterministic end-to-end cells for:

- canonical plus derived plus dependency actuation;
- mutation-to-projection-ready;
- source supersession/invalidation closure;
- erase-to-fence and erase-to-no-orphan across FTS/vector/graph/evidence;
- restart/resume at each Slice 30 phase; and
- projection rebuild with generation replacement.

Correctness, SDK/wire parity, lifecycle closure, and receipt integrity are
zero-tolerance gates. Performance thresholds must cite an accepted policy and
be sealed before execution; otherwise the cell is advisory and cannot support
or block a release claim.

## Workload 4 — packaged native retrieval witness

Run the frozen GLOBAL-01 held-out input through a locally packaged candidate's
named native A0 `Engine.search` call in a fresh database. Record returned
identities, source coverage, duplicates, arm contributions, latency/resources,
and gold sufficiency. No answerer, judge, map-reduce, or semantic controller is
present. The receipt must record `measurement_layer: data_plane` and
`engine_search_executed: true`.

This proves execution and descriptive retrieval behavior only. It does not
reclassify earlier GLOBAL-01 answer results or claim global synthesis quality.

## Failure policy and verification

Validate cleanliness, exact commit, owner-slice closure, manifest, package
identity, fixtures, and platform/device prerequisites before running. Write raw
results atomically after every cell. Malformed exhausted cells fail but later
independent cells continue where safe. Missing evidence is
`missing_prerequisite`, never zero or pass; candidate drift, source-installed
artifacts, digest conflict, or contradictory SDK results are
`environment_invalid`.

RED/GREEN harness tests reject missing/extra cells, bad digests, pooled
repetitions, false complete state, source-tree imports, package/version skew,
wire mismatch, mixed-layer claims, bypassed `Engine.search`, and relaxed
thresholds. Integrated tests exercise the two concurrency cells, focused
overhead/lifecycle cells, package consumers, Windows jobs, and conditional
CUDA path.

Run repository fast, heavy, all, applicable all-feature/operator, locally
packed Rust/Python/npm/native/CLI, Windows CPU/native, strict ptrace stress,
and focused Linux CUDA only where selected. Live-model, Windows CUDA,
pre-publication registry-installed, and exhaustive matrices are N/A. A formal
independent READY review remains required after Slice 7 and every Slice 10–60
verification record is complete.
