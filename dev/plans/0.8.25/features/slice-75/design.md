---
title: 0.8.25 Slice 75 — integrated closure design
status: DRAFT_SPLIT_RECONCILIATION_REQUIRED
design_version: 3
target_release: 0.8.25
depends_on: 73
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 75 design

## Authority and boundary

This design owns final integrated release closure after Slice 73. Slices 71,
72, and 73 own respectively the two performance investigations, preflight plus
installed CE profiles, and focused Windows Node/N-API coverage. Their strict,
exact-commit receipts are prerequisites. Slice 75 neither recreates those
studies nor treats historical handoff text as duplicate authority.

## Manifest and receipt

`IntegratedClosureManifestV1` binds the unchanged candidate, version, all
Slice 10–73 status/verification receipts and digests, package hashes, fixtures,
platform/device requirements, commands, workload cells, repetitions,
thresholds, timeouts, and allowed N/A routes. Unknown or missing material
fields reject before execution.

`IntegratedClosureReceiptV1` records every command and cell, installed package
and actual Engine method, read/projection identities, raw-output digest,
errors/timeouts, latency/resource summaries, and `passed`, `failed`,
`insufficient_samples`, `missing_prerequisite`, or `environment_invalid`.
Only `passed` supports closure. Partial evidence remains visible and cannot be
pooled into completeness.

## Installed cross-SDK and wire conformance

Build locally packaged, registry-equivalent Rust crate/CLI, Python, npm/native,
and applicable CUDA artifacts from one commit. In isolated consumers, prove
resolved paths and execute retained Slice 15–60 success, error, and unknown-
version fixtures. Require semantic and canonical-wire equality after the
documented casing conversion. Linux and Windows x64 CPU/native coverage is
mandatory; Slice 73's focused receipt is checked for identity and included,
but final exact-head hosted CI remains required.

## Representative concurrency and consistency

Run fresh-database 10,000-record one-reader/one-writer and 50,000-record
12-reader/one-writer cells with at least three cold and three steady
repetitions and 500 steady operations per reader. A fixed cycle covers search,
bounded canonical/state walks, evidence resolution, dependency trace, compact
integrity, constrained expansion, record/dependency/lifecycle mutation,
erasure/recreate, and projection readiness.

The manifest pins corpus/trace digests, seed, reset/warm-up, timeouts, no-retry
policy, runtime, affinity, and resource sampling. Report latency distributions,
throughput, writer wait, typed outcomes, errors/timeouts, RSS, CPU, storage,
and open/close time. Mixed visibility, duplicate/omitted pages, stale evidence,
searchable erased dependents, untyped busy/timeout, or false-ready projection
is a correctness failure.

## Lifecycle, overhead, and packaged native witness

At 10,000 records, run paired fresh-database baseline/new-operation cells for
eligibility, optional frozen context, canonical/state pages, evidence,
dependency, bounded actuation, projection status, integrity, and constrained
expansion. Run deterministic end-to-end mutation-to-ready, supersession,
erase-to-fence/no-orphan, restart/resume, and rebuild-generation cells.

Run frozen GLOBAL-01 input through the locally packaged candidate's named
native A0 `Engine.search` path. Record identities, source coverage, duplicates,
arm contributions, latency/resources, and gold sufficiency with
`measurement_layer: data_plane` and `engine_search_executed: true`. This proves
execution and descriptive retrieval behavior, not answer quality.

## Failure policy and final verification

Validate cleanliness, exact candidate, prior-slice closure, manifest, packages,
fixtures, and platforms before execution. Write raw results atomically after
each cell. Drift, source-installed artifacts, digest conflicts, or SDK
contradiction are environment failures; missing evidence is never zero/pass.

RED/GREEN harness tests reject bad schemas/digests, absent cells, pooled
repetitions, source fallback, version skew, wire mismatch, mixed-layer claims,
bypassed native search, stale prior receipts, and relaxed thresholds. Then run
the complete local release matrix and an unmerged PR's hosted checks at the
same SHA. Release-ready closure does not authorize tags, publication, registry
mutation, release creation, or post-publish smoke.
