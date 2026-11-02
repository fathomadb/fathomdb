---
title: FathomDB 0.8.28 Slice 60 — status
status: COMPLETE
target_release: 0.8.28
qualified_sha: e240efd4b91b8e3ba4ea53dc3edea73639f44747
---

# Slice 60 status

Complete on `dev/0.8.28`; qualified code and harness candidate
`e240efd4b91b8e3ba4ea53dc3edea73639f44747`. Final records follow that candidate.
[Requirements](requirements.md), [design](design.md) and
[ADR](../../../../adr/ADR-0.8.28-store-format-35.md) are accepted after Astra
design and code review, scoped finding repairs and independent verification.
No new branch or worktree was created for implementation. The existing
release baseline is fast-forwarded to the same final records commit.

Format 35 uses 64-row vec0 chunks, refuses version-34 stores without migration,
and keys node body FTS rowid to write cursor. A per-vector span stores truthful
head coverage atomically; unknown remains NULL. Independent embedder-api
0.8.0 adds the default-unknown truncation report; Candle BGE supplies actual
UTF-8 offsets. Span completeness, custom-trigger visibility, erasure,
rebuild and recenter preservation are covered. Overflow generation and native
eligibility remain follow-on slices. This completes allocated TC-016ff172 and
the B2-8 storage portion; populated-open work stays deferred as planned.

Behavior changes used separate observed RED/GREEN commits. Astra findings
on physical tuple completeness, cleanup-trigger authentication, registry
ownership, historical fixtures and actual batch coverage were resolved.
Additional passing coverage was recorded as coverage, not fictitious RED.
The full gate's stages completed through scoped corrections and independent
reruns: lint, typecheck, security, Rust, Python, TypeScript and NAPI checks
passed. The initial 205-suite collection had seven failed registrations;
all were fixed and rerun independently. This is a completed gate campaign,
not a claim that the original single invocation passed. Independent affected
coverage passed 44 tests plus the batch child; cached real-BGE coverage passed.

| Qualification | Result |
| --- | --- |
| AC-013, 10k vectors, 768 dimensions, 1,000 samples | PASS: p50 59 ms / p99 67 ms against 80 / 300 ms |
| Synthetic ac_013b | Report-only 0.0036, identical to Slice 42 baseline; fused-search versus vector ground truth diagnostic |
| Binding TC-5 GPU bridge | PASS: 7,667 real documents, zero synthetic, 100 queries, recall 0.958, 95% bootstrap CI [0.938, 0.974]; unchanged CI-high >= 0.90 |
| Requested legacy EU7 | Actual exit 101 retained: N=1,000 recall 0.877, CI-high 0.897; N=7,667 recall 0.775, CI-high 0.801; superseded recall diagnostic, not PASS |
| Bare-vec0 scale | All 30 cells completed (384/768 dimensions, five sizes through 2M, chunks 64/128/256); CS64 wins all ten dimension/size comparisons |

The [scale harness and protocol](../../../../experiments/slice60-vec0-scale/README.md)
are reproducible in-repo. At 2M vectors, chunk64/128/256 p50 was
436.43/586.92/653.87 ms (384 dimensions) and 499.60/627.78/709.70 ms
(768 dimensions). Peak scratch storage was 6.568 GB. At 5.1 vectors/record,
2M vectors represent about 392,157 records. There were zero top10 differences
and 97 equal-Hamming cutoff ties. The sweep used warm caches and some cells
overlapped compilation; results establish this measured comparison, not an
isolated cold-cache production SLO. The old CS64-versus-1024 crossing near
1.2M was not remeasured and is neither confirmed nor refuted. CS64 satisfies
the planned decision rule and is frozen for format 35.

TC-5 used the frozen query-source-first fixture and exact same-model f32 ground
truth, verified candidate wheel/CLI/benchmark hashes and RTX 3090 CUDA binding.
The runner is [tc5_gpu_v2.py](../../../../../experiments/tc5_gpu_v2.py);
[requirements](requirements.md#recall-qualification-reconciliation) records
the Astra-approved correction to the already authoritative oracle. No recall
threshold or baseline waiver changed. Receipt:
`/tmp/slice60-tc5-qualification/tc5-gpu-arm-result.v2.json`
(SHA-256 `a6cfccb1b3ea827f347bfac8fcc7709a0ba21e20012605d1a6596061ac89683b`); candidate config:
`/tmp/slice60-tc5-candidate.json`. Raw gate and scoped logs remain under
`/tmp/slice60-independent-*` and `/tmp/slice60-fixture-*`; generated raw data
is not committed. Final documentation and state receive scoped Markdown,
release-view and closed-dependency checks. Slice 70 is next; no Slice 60 work
remains and no publication is authorized.

Follow-up completion audit (2026-11-01) at
`50452e7eda0182b998d610fa506ef9980b47b813`: independent Astra review approved
the draft reconciliation, requirements, design, implementation and RED/GREEN
history with no actionable findings. A separate Astra verifier passed 26
focused tests, including schema spans, format/refusal, projection coverage,
the batch child, custom triggers, orphan erasure, default-unknown API and
cached real-BGE coverage. Markdown, generated release views and the Slice 60
closed-dependency preflight passed. Landing preflight caught a stale handwritten
board row and remaining-slices list; both were reconciled to the release state.
The TC-5 receipt checksum and candidate
binding match this record; changes after the qualified candidate are records
only. No additional implementation or full regression run was needed. No
branch or worktree was created during this audit; Slice 70 remains next.
