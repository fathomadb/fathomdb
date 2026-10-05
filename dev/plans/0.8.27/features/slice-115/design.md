---
title: FathomDB 0.8.27 Slice 115 — engine performance characterization design
status: REVIEW_READY
target_release: 0.8.27
---

# Slice 115 measurement design

## Authority and boundary

The release plan and AC27-115A–D in [the execution plan](plan.md) control this
slice. Measure the current Rust engine with the unchanged Slice 90 and 114
defaults. No production tuning, new public setting, release threshold, or
cross-release speedup is authorized. Slice 135 owns the fixed 0.8.26
comparison. The HITL's 2026-10-04 exception allows 115 before the open Slice
110 Tegra row; this CPU engine evidence cannot close it.

## Protocol and cells

Write `protocol.json` and hash it before executing the candidate. Bind the
candidate Git SHA, runner source and compiled binary SHA, feature set/profile,
host/kernel/CPU/memory/storage/Rust versions, model files and corpus digest.
Use a release build on local storage, one fixed seeded corpus and default
configuration. Each path gets one warm-up followed by at least seven timed
samples; keep each raw sample, correctness count, and any failed attempt.
Separate setup and teardown from timed operations. Read-only cells reuse one
process and a warm cache; record cold open separately. Alternate query order
within each repetition to reduce drift. For mutating write, projection and
erasure cells, construct a fresh database from the same seed for **each**
sample outside the timed region. Record and verify the pre-state row counts,
source IDs, projection count and digest, then time exactly one mutation or
write-to-ready operation, verify its post-state, and close outside the timed
region. Reopening or reseeding is never included in that operation's latency.
Reject a sample when its pre-state does not match the frozen seed or its
post-state fails the semantic count; a second erasure of an already removed
source is never a valid sample. The path list is:

1. Fresh open and close, including a reopened populated database.
2. Canonical node write and ingest/project-to-ready with a deterministic
   384-dimension provider; record write latency and projection lag separately.
3. A small, targeted CPU-library repeatability/compatibility probe with the
   pinned default BGE-small-en-v1.5 provider, using cached and hash-verified
   model files. Bound it to a few fixed texts and one projection, not a corpus
   sweep; record model load/open separately from inference and projection.
   Any substantial real-model corpus embedding runs with the CUDA feature on
   an available RTX 3090, with device engagement verified, and has a separate
   label. No network fetch during a run.
4. Text-only, vector-stage and hybrid search on the same projected corpus.
   A vector-stage test seam is identified explicitly; it is not labeled public
   `Engine::search`.
5. Bounded graph expansion and evidence resolution with a known present edge
   and evidence handle. Include a null-result control; a null-only sample
   cannot satisfy evidence coverage. Investigate the `_material` construction
   in `evidence.rs` with a profile or bounded allocation comparison, without
   changing product code.
6. Source erasure of a known populated source, checked by post-erase absence.

Each cell records p50, p90, p99 and maximum latency from raw nanoseconds and
the fraction of successful semantic checks. Throughput is reported only for
workload cells with a sustained measured interval, with operations and elapsed
time retained. Tail percentiles use a stated nearest-rank rule, not an
extrapolated latency guarantee from seven samples.

The existing Slice 90 D27 v2 six-repetition mixed-workload receipt is a
historical qualification and is cited as such. Rerun it on the current
candidate only for a concrete same-protocol drift question; its frozen v2
runner, queue/backlog, projection lag, provider concurrency, close and
resource observations remain authoritative for that historical candidate.
Do not apply new cell invalidators to its reviewed protocol: its swap policy
is report-only. If a conditional current rerun is invalid, retain the attempt
and state the current drift question is inconclusive. Existing
AC-011/017/018/029/072/073/076/081 gates keep their own thresholds and
candidate requirements; they are not redefined by this characterization.

## Attribution and invalidation

Every path records harness wall time plus at least one separately timed,
nontrivial stage or a sampled profile of that same operation. The stage must
be an actual provider, projection/queue, SQLite, or engine suboperation, not
fixture setup renamed as attribution. Use existing engine observations,
provider-call timing/count from a transparent test provider, and SQLite
timing/counters where existing seams allow them. State stage boundaries and
unattributed residual explicitly; a row with only a total time is incomplete.
Record one unprofiled control before profiling. Select the two paths with the
highest p90 wall time among
valid non-model cells for deeper sampled CPU stacks; ties use the order above.
Profile graph-evidence resolution as an additional selected path when it is
not among those two, to answer Slice 114's handoff. Prefer `perf` when it can
sample this exact binary. This host has `perf_event_paranoid=4`; use repeated
interrupt-and-backtrace sampling under `gdb` as the predeclared fallback,
separate from latency measurement, if unconfined ptrace is available. Record
tool version, sample count/frequency, collection errors, and control-vs-profile
overhead. If neither method yields two usable path profiles, AC27-115C stays
open and Slice 115 cannot be marked complete.

Reject a cell for candidate, protocol, model/corpus hash or environment
mismatch; semantic failures; missing samples; competing heavy jobs; CPU
governor change; swap activity; storage change; or profiler-induced timing
substitution. Preserve invalid samples but exclude them from summaries. The
runner computes summaries and validates all bindings from raw data. It never
manufactures a PASS by filtering slow valid samples.

## Implementation and proof

Use a measurement-only Rust integration fixture or an external runner compiled
against the exact engine source. Reuse existing fixture builders and D27 code;
add the narrowest adapter needed for cells not already instrumented. First
write RED checks for schema, hash/candidate mismatch, missing or invalid
samples, percentile arithmetic, and semantic counts. Then implement GREEN and
capture a real-database smoke before timed runs. Keep tests fixed while
repairing code. A real default-model cell must verify model availability and
identity before measurement; a test provider is never its substitute.

The receipt includes the frozen protocol, raw JSON, environment and command
log, binary/runner/corpus/model digests, recalculated summary, per-cell
validity, profile artifacts, and decision table. Decisions are limited to:
accepted gate met, comparable breach needing focused remediation,
exploratory hotspot with named owner, characterized without actionable
finding, or inconclusive with a named missing control. A comparable breach
blocks closure until focused RED/GREEN repair and a new candidate run. New
code or executable scripts receive the full agent gate; documentation/data
edits receive scoped validators. Independent design, code and verification
reviews bind the final files and candidate SHA.
