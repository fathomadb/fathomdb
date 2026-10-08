---
title: Slice 135 installed TypeScript S02 contention feasibility
status: AUDITED_FUNCTIONAL_ONLY
target_release: 0.8.27
---

# TypeScript S02 contention feasibility — 2026-10-07

The installed TypeScript API returns promises backed by Rust blocking tasks
for writes and searches. A new runner used one shared engine handle and
started an eight-write task alongside an eight-cycle reader task. Each reader
cycle called `read.get`, text search, vector search and graph neighbors. It
then drained projection work, checked evidence, erased the graph and writer
sources, closed and reopened the database, and checked product and persisted
state. The runner bound the exact installed packages from the
[vector-repaired TypeScript S02 subset](../../s02-ts-vector-repaired-comparison-protocol.json):
0.8.26 source `f99e002f0d2e4002f3694c9f8d4986b56089edaa` and candidate
source `3f29d649d0213e595c0dab251a449d92fd625792`.

Both single-process functional runs passed an independent audit of source,
npm archive and installed module hashes, actual call intervals, semantic
observations and direct reopened SQLite state. Writer/reader intervals
overlapped in eight reader cycles on the baseline and seven on the candidate.
Projection readiness was `ready` before and after reopen. Evidence resolved
to the expected source bytes; erasure excised three graph nodes, one edge and
eight writer nodes. After reopen, 32 corpus nodes remained, erased rows were
absent and `PRAGMA integrity_check` returned `ok`.

An initial functional pair included direct SQLite verification inside its
whole timer. Its raw outputs are retained as `*-v1` but excluded from latency
interpretation. The corrected `v2` runner moved direct SQLite verification
after the product timer and retained SDK state checks before and after
reopen. Its single observed elapsed times were 4,964.444 ms on the baseline
and 5,245.419 ms on the candidate. **These one-off times are not a qualified
paired latency comparison**: no baseline noise pilot, frozen contention
protocol or sampled blocks existed for this cell. The values must not be used
as a regression estimate.

The independent auditor rejected falsified overlap and a changed canonical
SQLite row while structural integrity remained `ok`. The focused Node tests
passed 3/3, Python auditor tests passed 2/2, and Ruff passed. No full agent
gate was run for this feasibility increment.

Local raw evidence is `/tmp/slice135-ts-s02-contention-feasibility`. Its
verified `SHA256SUMS` covers 30 files, including both initial attempts, both
corrected receipts, SQLite databases, runner and auditor snapshots, prior
installed-artifact reference manifests and negative-control output. The
manifest SHA-256 is
`ea810cda8f123302bf92e89ddb94033f69cb6e3690e31ee0bc461958c816efd9`.
The next TypeScript step is a baseline-only noise pilot, then a frozen
alternating-pair campaign with fresh processes, resource/environment records
and independent raw recomputation. Rust SDK contention and the broader Phase
1 matrices remain open. Final raw retention is deferred to the end of Phase
1.
