# Refused embedder characterization and recommendation

Owner-requested investigation, 2026-10-31. No D50-1 or D50-2 implementation,
no engine changes, no release landing. Engine base: `ac83688a1` (phase-1 docs
on `544bf74f8`). Tool commits: `5fb45c24a`, fixture correction `caecfb831`. See [README](README.md) for method.

## Measured baseline results

Retained: **108 observations and 108 accepted-runtime recoveries**: nine
operations, four states, three repeats. The first run passed eight operation
tests; its purge fixture mistakenly used an active target. That failed run
is preserved. Only purge was corrected and rerun (12/12 observations and
recoveries, test passed). Neither initial purge observation is pooled.

| Operation | Refused + pending median (s) | Refused + empty median (ms) | Absent + pending result | Accepted + ready result |
| --- | --- | --- | --- | --- |
| `actuate` | 30.000662 (`Scheduler`) | 1.586 (`Ok`) | `Ok` | `Ok` |
| `configure_projections` | 30.000397 (`Scheduler`) | 0.639 (`Ok`) | `Ok` | `Ok` |
| `drain` | 3.000485 (`Scheduler`) | 0.226 (`Ok`) | `EmbedderRequired` | `Ok` |
| `erase_source` | 30.000625 (`Scheduler`) | 18.851 (`Ok`) | `Ok` | `Ok` |
| `excise_source` | 30.000621 (`Scheduler`) | 18.700 (`Ok`) | `Ok` | `Ok` |
| `purge` | 30.000618 (`Scheduler`) | 7.756 (`Ok`) | `Ok` | `Ok` |
| `rebuild_projections` | 30.000397 (`Scheduler`) | 9.352 (`Ok`) | `EmbedderRequired` | `Ok` |
| `rebuild_vec0` | 30.000461 (`Scheduler`) | 7.563 (`Ok`) | `EmbedderRequired` | `Ok` |
| `transition` | 30.000547 (`Scheduler`) | 0.808 (`Ok`) | `Ok` | `Ok` |

Every retained failed call preserved all table snapshots and its pending count.
Refused/pending calls made zero embedding calls during the measured operation.
Every retained case reopened with the original accepted provider and reached
zero pending work. Full min/median/max and pending counts are in
[summary.json](summary.json); raw observations and source hashes are in
[results/results.json](results/results.json) and
[purge-corrected/results.json](purge-corrected/results.json).

Validation: focused clippy and cargo check passed; 3 readiness, 13 cache, and
24 vector-equivalence tests passed. Markdown gate passed. Full agent verify
stopped at missing checkout-owned `.venv` preflight, before its later gates.
See the committed logs. No full-workspace green claim.

## Diagnosis and scope

The equivalence probe refuses the supplied backend, while the engine retains
it as an attached runtime. Open passes no embedder to the projection scheduler.
`drain` handles only the absent-runtime case specially, then asks for both
idle workers and no durable pending work. The refused session has idle workers
but cannot remove the pending work, so waiting cannot succeed. The metadata
and erasure callers inherit this store-wide barrier.

This matrix deliberately gives lifecycle/actuation/purge a non-vector `note`
target and puts the pending vector work on a separate body edge. Their timeout
therefore establishes that unrelated pending work blocks those operations.
Configuration re-applies the existing declarations unchanged: even a valid
no-op request waits. This is not a measurement of a vector-configuration change.
The accepted control drains its seed before calling the operation, so it does
not characterize rebuild admission races or contending workers.

Failed calls are checked against logical snapshots of every application table,
including canonical, projection, provenance and audit tables. Recovery checks
accepted reopen and zero pending work, not complete vector contents, lifecycle
postconditions, physical erasure or file/WAL byte identity. Neither these
checks nor the existing suites establish that D50-2 is safe to implement.

`probe_calls_at_open` is an imperfect historical field name in the first tool
version: it counts all calls through the controlled provider up to open's
return, potentially including background work in the accepted case. It is
attributable to probes in the refused cases, whose scheduler has no provider.
No conclusion depends on an exact accepted-case probe count.

## Additional rebuild finding

Both `rebuild_projections` and `rebuild_vec0`, when refused but initially
without pending work, return `Ok` and change pending count from zero to one.
The existing projected anchor is invalidated and readiness changes to
`Deferred`; the refused scheduler cannot rebuild it in that session. This
occurred in all three repeats for each operation. Reopening with the original
accepted backend drains the work. This does not establish corruption or
violate an assumed synchronous rebuild contract; it does show that successful
degraded-open admits operations creating new work it cannot process. The
earlier characterization did not cover refused-empty rebuilds: the result
depends on whether backlog exists at admission.

## Original evidence recovered

The original raw command and tool result survive in Claude session
`98f1be70-64c9-48f0-bb77-4907798b9fe6`, subagent
`agent-a0a193428c0394156.jsonl`, lines 355-356, tool id
`toolu_01EvtmHMTujx41utcEj9drdC`. Extracted without rewriting into
[original-probe-record.json](original-probe-record.json), with source hash.
It reports `Deferred`, pending 1, no usable runtime; `drain(3000)` returns
`Scheduler` in 3.000420905 s; `transition(n1, Deleted)` returns `Scheduler`
in 30.000630307 s. This corrects the earlier investigation's inability to
locate raw evidence; it is not merely a number repeated in the phase-1 docs.

A requested Sonnet recovery agent could not run (weekly usage limit). The
available read-only helper located the transcript and scratch source; the
parent independently extracted the raw tool records. No transcript was resumed.

The main transcript's later owner challenge (line 21378) and agent response
(line 21458) proposed investigating the global drain barrier before changing
metadata operations, with a possible causal fix in Slice 70. That proposal
was not an owner ruling. Format 35 itself does not modify these drain paths.

## Why refusal exists

This was a deliberate portability policy, rather than accidental leftover
model-choice behavior:

- [Original GPU/portability design](../../design/0.8.1-embedder-gpu-and-portability.md)
  section 3 names the same model/revision/dimension producing different vectors
  across machines or backends. Identity alone cannot test numeric compatibility.
- [Accepted 0.8.18 ADR](../../adr/ADR-0.8.18-vector-equivalence-self-check.md),
  signed 2026-07-09, R-VEQ-4/6 explicitly chooses successful degraded-open so
  explicit FTS remains available. ADR authored at `beee25a4f`, implementation
  `f2f843ed6`, fail-safe correction `5a35442db`, rulings aligned `589a23a5d`.
  The accepted header freezes the tolerances; its lower "D4 pending" prose is
  stale and must not be treated as a still-open decision.
- [Accepted 0.8.27 runtime ADR](../../adr/ADR-0.8.27-engine-owned-runtime-topology.md)
  explicitly retains degraded-open on vector-equivalence failure, including
  probe timeout. The runtime redesign consciously preserved this choice.

Current refusal includes both numeric mismatch and inability to verify:
provider error/panic/timeout/wrong dimension, incomplete or unreadable baseline,
and missing/unreadable required mean. Those failures remain possible with one
fixed model. They should not all be described as measured numeric divergence.

## Has this happened with supported production backends?

No documented supported-production numeric refusal was found in the searched
release records and memory. That is a search finding, not proof it never occurs.
The new refused cases use intentionally different vectors with a forced stale
cache. They do not measure real CPU/CUDA false-refusal frequency or retrieval harm.

[Historical U3 calibration](../../plans/runs/0.8.18-slice-0-cross-backend-calibration.md)
measured the pinned BGE weights:

| Pair | Centered flips | Maximum L2 | Interpretation |
| --- | --- | --- | --- |
| Candle CPU / CUDA | 0 / 17,280 | 1.402e-6 | Passed the current 0-flip / 1e-5 limits |
| Candle CPU / ONNX CPU | 0 / 17,280 | 8.781e-7 | Numerically passed; ONNX still has a distinct identity |

[Historical eval-only ONNX CUDA](../../plans/runs/0.8.18-slice-0-onnx-gpu-ep-calibration.md)
measured 2 raw flips, 1 proxy-mean flip and maximum L2 5.381e-4, reproducibly
across three runs. It would exceed the thresholds, but ONNX CUDA is currently
unsupported and its distinct identity already blocks that substitution. It
is evidence that backend numeric differences can exceed the limits, not of a
supported production refusal or harmful retrieval regression.

The August CPU/CUDA float-byte difference recorded in memory is benign ULP
noise. Different float bytes do not establish a failed equivalence check.
No current production-runtime calibration was performed for this investigation.

## Cache caveat

[Accepted TC-68 trade](../../design/0.8.20-tc68-equivalence-probe-fingerprint-cache.md)
section 6 states that the cache excludes backend/device/library/driver changes.
A same-identity changed backend can hit the cache, skip all probe embeds and
never be detected. The existing executable residual test pins this behavior;
its current rerun is captured in [controls.log](controls.log). This is distinct
from the intentionally invalidated cache used to produce refused fixtures.
An open failure policy alone would not correct this coverage limitation.

## Recommendation

Retain the ability to reject unverified or incompatible vector output. A fixed
model does not prevent a changed implementation, provider failure, malformed
output, or damaged verification state. The measurements do not justify using
vectors after verification fails, nor do they justify relaxing the numeric
thresholds. They also do not prove retrieval harm at the current threshold.

Prefer revisiting **successful degraded-open with a refused attached runtime**:
normal vector-enabled open should fail with a specific reason if verification
cannot accept its backend. Deliberate no-embedder/text-only/recovery access can
remain explicit; the absent-runtime controls show these baseline metadata and
erasure calls already have a separate path. This is a proposed successor to
the accepted ADRs, not a decision made or implemented by this investigation.
Provider-timeout shutdown and cleanup obligations still apply on failed open.

Distinguish actual mismatch from inability to verify in that design. Review
cache invalidation separately if backend-change protection remains the stated
contract. Do not silently convert a failed check into dense readiness or
embedding success.

Keep D50-1/D50-2 unimplemented pending that policy choice. The measurements
prove an impossible-progress wait; they do not validate replacing store-wide
barriers with worker-idle checks. If degraded-open is retained, its operation
semantics need an explicit contract and race/reopen tests. Independently,
removing unrelated store-wide waits on a healthy but busy engine requires the
worker-commit and dependency-closure investigation discussed in the transcript.
