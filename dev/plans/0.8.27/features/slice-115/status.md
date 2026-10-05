---
title: FathomDB 0.8.27 Slice 115 — engine characterization status
status: IMPLEMENTED_PENDING_INDEPENDENT_REVIEW
target_release: 0.8.27
engine_source_sha: 4ca09d443d7bcc24072e39fb3c683905a9134f27
---

# Slice 115 status

The measurement-only workload ran against engine source
`4ca09d443d7bcc24072e39fb3c683905a9134f27`. No engine or public API
behavior was edited. The [final receipt](evidence/final) validates its exact
source, dependency lock, protocol, runner, corpus, model and binary bindings. The raw run has
twelve paths, one warm-up and seven valid semantic samples per path. Independent
code review and Terra verification remain pending
before this slice can be marked complete on the release branch.

## Acceptance assessment

| Acceptance | Result |
| --- | --- |
| AC27-115A | The frozen [protocol](protocol.json) records source SHA, Cargo lock digest, release profile, `test-hooks,default-embedder`, accepted `2/5/30,000/1,000,000/100` settings, 32-row seeded corpus, one warm-up/seven samples, path order and alternating search order, cache and mutation-state controls, metric rule, invalidators, selection rule and GPU policy. The receipt records binary, runner, protocol, corpus and model digests; host/kernel/CPU/Rust/storage/governor/swap observations and exact command. |
| AC27-115B | All twelve planned paths have seven real-database samples and a correctness count. A cached, pinned BGE-small-en-v1.5 CPU-library cell returned repeatable 384-dimension vectors and wrote/drained one real vector projection per sample. The vector-stage seam is explicitly test-only; text and hybrid use engine search. No substantial embedding corpus ran on CPU. |
| AC27-115C | Projection, model CPU, vector-stage and hybrid have actual provider/inference stage timings. The other eight paths have GDB interrupt-and-stack profiles tied to the same binary and operation, with at least three operation frames. The final non-model p90 rule selects fresh open and populated reopen for deeper profiles, plus graph evidence. Each has a matched unprofiled control. GDB pauses only profile loops, not the seven latency samples. Caveats on short paths and profile overhead appear below. |
| AC27-115D | [Raw samples](evidence/final/raw.json), [recomputed summary](evidence/final/summary.json), environment/commands, model hashes, GDB stacks and [invalid-attempt register](evidence/invalid/attempts.md) are retained. The copied receipt independently revalidated and reproduced the exact summary bytes. No same-protocol comparable breach or new numerical release gate was inferred; Slice 135 can rerun the fixed workloads on the published 0.8.26 candidate. |

## Final candidate observations

Times are milliseconds. With seven valid samples, nearest-rank p90 and p99
both equal the observed maximum; they are descriptive, not a tail guarantee.
Each result has seven of seven successful semantic checks.

| Path | p50 | p90 | p99 | Disposition |
| --- | ---: | ---: | ---: | --- |
| Fresh open | 21.779 | 23.808 | 23.808 | Characterized; no accepted comparable threshold. |
| Populated reopen | 13.597 | 14.478 | 14.478 | Characterized; no accepted comparable threshold. |
| Close | 4.496 | 4.684 | 4.684 | Characterized; bounded close succeeded. |
| Canonical write | 0.528 | 0.568 | 0.568 | Characterized; one new canonical row per fresh database. |
| Projection write-to-ready | 6.350 | 7.039 | 7.039 | Characterized from before write through drain; one vector row present afterward. |
| Real default-model CPU probe | 185.489 | 208.845 | 208.845 | Characterized as a bounded library compatibility cell. This metric is model load plus first inference; write-to-ready projection is separately timed. |
| Text-only search | 0.222 | 0.482 | 0.482 | Characterized; FTS result nonempty and no provider call. |
| Vector-stage search | 0.444 | 0.939 | 0.939 | Characterized; pre-fusion vector branch through the test-only seam. |
| Hybrid search | 0.460 | 1.094 | 1.094 | Characterized; real engine fused result and provider call. |
| Graph expansion | 0.926 | 0.950 | 0.950 | Characterized; known present edge and empty-edge-kind control. |
| Graph evidence resolution | 0.290 | 0.325 | 0.325 | Characterized; nonempty target evidence resolved to canonical source. |
| Source erasure | 12.452 | 12.720 | 12.720 | Exploratory SQLite I/O hotspot for the engine erasure owner to compare in Slice 135; source and projected rows absent afterward. |

The transparent provider consumed a median 0.001 ms of the 6.350 ms
write-to-ready cell; separate medians were 0.516 ms for write and 5.647 ms
for drain. The residual includes scheduling, SQLite and unattributed wait.
Vector-stage and hybrid provider medians were each about 0.001 ms, so their
residuals are chiefly search/SQLite/reader work. In the real-model cell,
median model load was 173.009 ms, first inference 12.122 ms, and the separate
one-row write-to-ready projection 19.873 ms. These stage timings do not
prove which part of the residual dominates.

GDB was used because `perf_event_paranoid=4` prevented `perf` sampling. The
selected deeper operation-frame counts were fresh open 13/24, populated
reopen 8/24, and graph evidence 17/20. Matching 15-second GDB-profiled to
unprofiled operation-count ratios were 1.338, 0.955 and 1.001 respectively. The first
ratio exceeds one, showing cache or run-order drift; no profiler-overhead
correction is applied to latency. Exploratory erasure operation stacks show
SQLite writes and `fsync`; graph-evidence operation stacks frequently show
SQLite prepare/validation. The unused `_material` clone construction is
present in `evidence.rs`, but no sampled stack attributes a material fraction
to it. Its allocation cost is unresolved and does not justify a product edit
from this slice. The canonical-write profile loop uses repeated single-row
writes to obtain operation frames; its growing state can increase contention
relative to the fresh-database latency cell. Treat that stack as qualitative
owner attribution only.

The host was `windchill3`, Linux `7.0.0-38-generic`, AMD Ryzen Threadripper
PRO 5945WX with 24 logical CPUs, performance governor and `/dev/nvme1n1p1`
temporary storage. Before/after swap counters and governor were unchanged,
and no competing heavy process was observed at the run boundaries. The local
NVIDIA driver was unavailable, so no optional GPU cell ran. The required
real-model CPU cell remained a small compatibility/repeatability probe; a
substantial embed benchmark still requires the 3090 policy and separate
evidence.

## Historical gates and disposition

Slice 90's final `1398c821d` candidate had a strict six-repetition D27 v2
PASS and named release-selector PASS receipts in its [status](../slice-90/status.md).
Earlier D27 failures and the owner-accepted throughput shortfall stay in that
historical record. Slice 115 neither replaces that decision rule nor claims
those receipts qualify `4ca09d443`. There was no concrete same-protocol drift
question requiring a new D27 campaign here. The current measurements have no
matched 0.8.26 host/build/workload pair and make no cross-release speedup
claim. Slice 135 owns that comparison and should preserve each cell's provider,
cache, feature, seed and metric semantics.

## Verification and review state

- Fifteen focused receipt/profile/model/lock/security-policy checks pass; the real-engine release
  workload passed all twelve cells and the copied receipt revalidated with an
  identical summary.
- The pre-review full `./scripts/agent-verify.sh` passed lint, typecheck,
  security, and the Rust workspace suite. It failed three test suites: Slice
  90 root reconciliation has an inventory blob mismatch already present at
  untouched base `4ca09d443`; the Python suite required a clean candidate
  worktree for its native test-hook wheel, so it stopped on the uncommitted
  Slice 115 files; its native-receipt check then failed. The full run took
  1,258 seconds. The narrow review repair was followed by focused GREEN checks
  and a new exact-binary measurement; the full gate was not rerun for that
  finding. No full-workspace green claim is made.
- The staged Gitleaks hook identified the pinned tokenizer SHA-256 as a false
  generic API key in eleven exact Slice 115 evidence paths. An exact-digest,
  path-scoped policy exception and its equality guard retain blocking for
  different high-entropy digests; the pre-commit scanner remains enabled.
- The TDD sequence and retained invalid attempts are recorded in
  [chronology](tdd-chronology.md) and the [attempt register](evidence/invalid/attempts.md).
