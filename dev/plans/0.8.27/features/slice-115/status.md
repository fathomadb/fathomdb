---
title: FathomDB 0.8.27 Slice 115 — engine characterization status
status: COMPLETE_ON_RELEASE_BRANCH
target_release: 0.8.27
engine_source_sha: 4ca09d443d7bcc24072e39fb3c683905a9134f27
measurement_sha: 012e132920147396ac195f14af74444dd698f48e
---

# Slice 115 status

Slice 115 is complete on the local release branch under the HITL's instruction
to proceed while Slice 110 was open; Slice 110 subsequently completed. The
measurement-only workload ran
against engine source `4ca09d443d7bcc24072e39fb3c683905a9134f27` and
is committed at `012e132920147396ac195f14af74444dd698f48e`. No engine or
public API behavior was edited. The [final receipt](evidence/final) validates
its exact source, dependency lock, protocol, runner, corpus, model and binary
bindings. The raw run has twelve paths, one warm-up and seven valid semantic
samples per path. [Independent code review](code-review.md) and Terra
verification pass. At the original closeout, the broader repository gate had
an inherited Slice 90 inventory failure, so Slice 115 did not claim a
full-workspace green result or close Slice 110.

## Post-Slice 110 Tegra check (2026-10-06)

Slice 115 reviewed the merged Slice 110 changes and determined that no
additional Slice 115 acceptance or measurement work is required. Slice 115
remains closed; the lock-test correction below preserves verification of its
historical receipt.

The merged Tegra repair at `a25d063cd` changes the vendored CUDA allocator,
CUDA probes, and NAPI module-load initialization. It does not change engine
source. The Slice 115 runner is a standalone Rust engine workload with
`test-hooks,default-embedder` and a forced CPU model probe; this feature graph
does not include `cudarc` or the NAPI module. Its frozen source, binary and raw
receipt therefore remain valid for the measured candidate. The optional GPU
cell was never run, so the Tegra fix does not revise a Slice 115 GPU result.

Slice 110 changed `Cargo.lock` only to replace the CUDA-only `cudarc` registry
source with the vendored path. The frozen Slice 115 protocol still correctly
pins the measured lock (`03a88a539f59...`), and the runner correctly refuses a
new run from the changed checkout. Its focused lock test now retrieves the
measured commit's lock instead of incorrectly comparing the historical pin
with the current checkout. All 15 focused Slice 115 tests pass after that
test correction. A copied final receipt recomputes the identical summary
(SHA-256 `d238ba528f5af8e217e249f0d0f4a5f8addb84febf56b553c7beb85a92a8d210`).
No latency rerun or acceptance rework is required; Slice 135 still owns a
current-candidate cross-release comparison.

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

- Fifteen focused receipt/profile/model/lock/security-policy checks pass; the
  real-engine release workload passed all twelve cells and the copied receipt
  revalidated with an identical summary. Independent `gpt-6-sol` high code
  rereview closed three findings and found no remaining issue. Independent
  Terra high read-only verification recomputed the receipt, passed 15 focused
  tests, Ruff and Markdown lint, and confirmed the inherited Slice 90 mismatch
  at the untouched base.
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
- After the implementation commit, a focused clean-checkout Python run with
  an isolated, non-editable wheel and `PYTHONPATH=src/python` passed
  **1,579 tests, 27 skipped**. The clean candidate test-hook wheel receipt
  passed its SHA, module digest and nonce check at documentation-only head
  `157dbe8a71460d58d1e8d35c2c1d19d4fb32a2b6`; 19 targeted projection
  tests passed. These resolve the pre-commit Python environment failure but
  do not turn the earlier full run green. The Slice 90 inventory mismatch was
  corrected after Slice 115 closed.
