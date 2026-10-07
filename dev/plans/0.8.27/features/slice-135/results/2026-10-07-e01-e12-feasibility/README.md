---
title: Slice 135 paired twelve-path engine feasibility
status: FEASIBILITY_ONLY_NOT_QUALIFIED_LATENCY
target_release: 0.8.27
---

# Paired E01–E12 engine feasibility — 2026-10-07

The exact 0.8.26 baseline source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa` and Slice 135 candidate
`b981b021bdfbeb080b2b4f803e99c118c436b13c` each completed the twelve
inherited [Slice 115 engine paths](slice115_workload.rs): fresh and populated
open, close, canonical write, write-to-ready projection, CPU default-model
probe, text, vector stage, hybrid, graph expansion, graph evidence and erasure.
Each path emitted seven valid observations. Both runs used real temporary
SQLite databases and the same 32-document plus graph corpus digest,
`c4bbde04fc7415ea321d24a29338a5d56beabca2d04677797de6b14503d3cde0`.

The retained [raw generator source](feasibility.py.raw), [baseline](baseline/raw.json) and
[candidate](candidate/raw.json) raw observations, exact Cargo manifests and
locks, build/run commands and logs, and compressed executables bind the
source, runner and binary bytes. The run succeeded for all 12 cells on each
version. The [independent audit](audit.py) recomputes per-cell counts and
min/median/max from the retained raw files, checks mutation state and stage
fields, hashes the decompressed executables, and rejects a deliberately
corrupted erasure post-state. Its [result](audit.json) lists each measured
cell and limitation. Re-running the audit from this directory produced the
same JSON bytes.

**No latency regression verdict follows from these runs.** Seven samples per
cell are below the planned count, there were no per-run start/end environment
snapshots, and the inherited query checks establish result counts rather than
exact eligible IDs and order. The legacy raw `profile_ref` values have no
sampled profiles in this feasibility run. Its `protocol_sha256` field binds
the inherited workload source, not a frozen Slice 135 protocol. The default
model files matched the Slice 115 hashes before the run, but the generator did
not capture their hashes at the timing boundary. These deficiencies must be
fixed in the executable Slice 135 adapter before baseline noise pilots and
the final E01–E12 comparison.

The [SHA-256 manifest](SHA256SUMS) seals all retained receipt files except
this README and the manifest itself. Source-bound feasibility does not replace
the installed SDK workloads or the four-area Phase 1 checkpoint.
