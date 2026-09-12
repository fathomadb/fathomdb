# Slice 85 — CUDA CE Engine p95 release exception

Recorded: 2026-09-11
Granted: 2026-09-11 by the repository owner (HITL).

**Status: GRANTED. This is an accepted non-pass disposition for release 0.8.25.
It is not a PASS and must not be recorded or displayed as one.**

This records a narrowly scoped, explicit release exception covering one failing
comparison in the retained Slice 72 CE profile: the `cuda` / `engine`
`median_repetition_p95` ratio. The owner has accepted a small, quantified
first-touch performance regression for 0.8.25 on the basis of a demonstrated
mismatch between what that gate was intended to measure and what it measures.

The exception changes no threshold, relaxes no oracle, substitutes no
statistic, re-baselines nothing, and authorizes no further timing campaign.
The corrected protocol in Appendix A remains unauthorized follow-on work.

## 1. The non-pass, exactly as measured

Both attempts ran the sealed protocol against the same candidate artifact, the
same machine, the same pinned GPU UUID, the same affinity and the same
comparator cells. Both failed in the same direction.

| Attempt | Verifier result | Ratio | Limit | Evidence |
| --- | --- | ---: | ---: | --- |
| 1 | FAIL | 1.108690 | 1.100000 | `runs/0.8.25-slice-85/ce-verify.log`, cell in `candidate-cuda-attempt1/` |
| 2 (retry) | FAIL | 1.123237 | 1.100000 | `runs/0.8.25-slice-85/ce-verify-final.log`, cell in `candidate-cuda/` |

The retry budget for this cell is spent. Two same-direction failures are not
noise that can be rerun away, and no further attempt is proposed.

All four gated comparisons, comparator `4fc1b890` versus candidate `3b66e1b4`:

| Device / path | Comparator `median_rep_p95` | Candidate | Ratio | Gate |
| --- | ---: | ---: | ---: | --- |
| cpu / standalone | 4.0485 ms | 4.0541 ms | 1.0014 | pass |
| cpu / engine | 2.8567 ms | 3.0607 ms | 1.0714 | pass |
| cuda / standalone | 1.2380 ms | 1.1786 ms | 0.9520 | pass |
| cuda / engine | 1.3251 ms | 1.4884 ms | **1.1232** | **fail** |

Because `verify-slice72-ce-profile.py` raises on the first failing comparison,
**no `ce-receipt.json` exists for Slice 85**. Every surviving CE obligation must
be cited from the four cell files directly, not from a profile receipt.

### 1.1 What passed, verified independently

The correctness checks execute before the ratio gate in
`validate_and_aggregate` and genuinely passed on all four cells:

- Engine rank order `[1, 4, 0]`; standalone rank order `[1, 4, 0, 2, 3, 5]`.
- CPU/CUDA cross-device CE score agreement: `max|Δ| = 0.000000` against a
  declared tolerance of `0.01`.
- Per-call output stability across all 20 steady calls in every repetition.
- Artifact, model, device-binding and offline-policy evidence intact:
  receipt digest binds, `source_imported: false`, model SHA-256 identical
  before and after, VRAM allocation bound to the measuring PID and the pinned
  RTX 3090 UUID.

## 2. What the gated statistic actually measures

`median_repetition_p95_ns` is the median, across 5 repetitions, of
`nearest_rank(durations, 0.95)` over 20 calls
(`verify-slice72-ce-profile.py:47-52, 211, 219`). With `n = 20`, nearest rank
selects `sorted[18]` — the **second largest of twenty**.

The Engine steady series is not a stationary sample. It is a deterministic
decay:

```text
comparator rep0: 1.518 1.325 1.320 1.311 1.297 1.288 1.293 | 1.043 | 0.999 ... 0.966
candidate  rep0: 1.654 1.497 1.464 1.445 1.523 1.427 1.457 | 1.061 | 0.965 ... 0.945
```

The mechanism is in the product source, not the data alone:

- `src/rust/crates/fathomdb-engine/src/lib.rs:734` — `READER_POOL_SIZE = 8`.
- `src/rust/crates/fathomdb-engine/src/lib.rs:2655` — dispatch is strict
  round-robin, `self.next.fetch_add(1, Ordering::Relaxed) % n`.
- `src/rust/crates/fathomdb-engine/src/lib.rs:8955, 9221-9223` — the eight
  reader connections are constructed in `open_locked`, per Engine open.
- `scripts/release/slice72-ce-profile-worker.py:173` — the harness makes
  **exactly one** warmup call before the 20 measured calls.

One warmup call touches one of eight readers. Measured calls 0–6 therefore each
land on a different cold reader connection; call 7 wraps back to the reader the
warmup consumed; calls 8+ are warm. Grouping every measured call by its
predicted touch number confirms the model:

| Cell | touch 1 (n=35) | touch 2 (n=40) | touch 3 (n=25) |
| --- | ---: | ---: | ---: |
| comparator engine | 1.311 ms | 0.997 ms | 0.972 ms |
| candidate engine | 1.452 ms | 0.975 ms | 0.946 ms |
| comparator standalone | 1.140 ms | 1.131 ms | 1.138 ms |
| candidate standalone | 1.152 ms | 1.136 ms | 1.135 ms |

**In 20 of 20 Engine repetitions across all four cells, the selected p95 value
falls at original call index 0–6** — always a first-touch call, never a settled
one. The gate cannot observe settled latency by construction.

The Slice 72 plan describes this cell as a steady-performance comparison. The
`standalone` path, which never opens the database and has no reader pool, shows
a touch1/touch3 ratio of 1.01 against the Engine's 1.50. That is the mismatch:
the statistic is named and intended for steady state and is computed entirely
from warmup.

## 3. Attribution

The added cost is device-independent in absolute terms:

| Cell | first-touch delta | settled delta |
| --- | ---: | ---: |
| cpu / engine | +0.156 ms | −0.025 ms |
| cuda / engine | +0.141 ms | −0.026 ms |

A regression inside the CE forward would scale with the forward (≈2.9 ms on
CPU, ≈1.0 ms on CUDA). A fixed ≈0.15 ms on both does not. The `standalone`
path — the CE forward with no database — is unchanged on both devices on the
same touch grouping. **The added cost is in the Engine's SQLite reader path,
not in the CUDA CE path.**

The regression is therefore correctly stated as: *a first-touch regression in
the Engine read path, reproducible on both devices, of roughly 0.14–0.16 ms per
reader connection.*

### 3.1 Candidate mechanism — hypothesis, not established

The Engine read path was restructured after the comparator commit. Slice 76/79
introduced per-reader-connection prepared-statement reuse
(`lib.rs:2694`, `set_prepared_statement_cache_capacity(10)`, absent at
`4fc1b890`), per-connection lookaside configuration (`lib.rs:9221-9232`), and
`SQLITE_CONFIG_MEMSTATUS` runtime-mode selection
(`a6650c81`, which removed `pcache2.rs`). Trading higher per-connection
first-touch cost for cheaper steady state is the expected shape of that change,
and it matches the measured direction on both devices.

**This remains a hypothesis.** The evidence establishes the per-connection
first-touch mechanism — period-8 structure, 20/20 percentile placement, and a
device-independent fixed delta — but it does not separately attribute the
0.15 ms among statement caching, lookaside configuration, and MEMSTATUS mode.
Separating them requires a profile of reader-connection first use, which was
not run and is not proposed here. No product fix is requested on the strength
of an unconfirmed mechanism.

## 4. Scope and limits of this finding

These limits are part of the disposition. The exception is granted on the
narrow claim, not a broader one.

1. **This is a first-touch regression, not an absence of regression.**
   First-touch is part of Engine performance and it has measurably regressed.
   The defensible statement is "a first-touch regression, not an *observed
   steady-state* regression" — not "not an Engine-path regression."

2. **Improvement is not universal on this workload.** Settled latency improved
   (p50 1.0004 → 0.9840 ms, −1.6%; touch2+ −2.6%), but because 7 of every 20
   measured calls are first-touch by construction, the aggregate worsened:

   | Pooled statistic | Comparator | Attempt 1 | Attempt 2 |
   | --- | ---: | ---: | ---: |
   | p50 | 1.0004 ms | 0.9709 (−2.9%) | 0.9840 (−1.6%) |
   | mean | 1.1185 ms | 1.1407 (+2.0%) | 1.1507 (+2.9%) |
   | p95 | 1.5026 ms | 1.4856 (−1.1%) | 1.6379 (+9.0%) |
   | p99 | 1.5430 ms | 1.6552 (+7.3%) | 1.6632 (+7.8%) |

   The mean and p99 of this workload are worse. That is a real cost being
   accepted, not an artifact being dismissed.

3. **The cost recurs per reader pool, not once per process.** The pool is
   constructed in `open_locked`, so every Engine open pays it again:
   ≈1.13 ms on CUDA and ≈1.25 ms on CPU (8 connections × the per-connection
   delta). Cache eviction under a wider query mix, or connection replacement,
   could introduce further preparation cost that this fixture cannot observe.

4. **The break-even figure is fixture-specific.** Against this six-passage,
   single-query fixture the trade pays back after roughly 43 searches per
   Engine open on CUDA and 50 on CPU. That number does not generalise to a real
   query mix and should not be quoted as a product characteristic.

5. **The comparator's age is not the objection.** Detecting the effect of
   implementation change is what a regression comparator is for, and 1083
   intervening commits are context, not grounds. The objection is specific:
   one warmup call is insufficient to reach the steady state this gate was
   written to compare, for an Engine with eight round-robin readers.

6. **AC-020 is a precedent for explicit owner revision, not for automatic
   retirement.** AC-020 divided a candidate's concurrent performance by the
   same candidate's sequential performance, so improving one term could worsen
   the ratio by construction. This gate compares candidate against comparator
   on the same statistic; its defect is warmup contamination. The relevant
   lesson from `seq-277` is that a defective performance oracle is corrected by
   an explicit owner decision on the record — which is what this document is.

7. **AC-072 and AC-081a/b/c do not replace this coverage.** They are absolute
   read-performance gates over 1,600 and 1,000 queries on the same reader pool,
   they currently pass, and they amortise the cold-pool term. That makes the
   residual product risk small. It does not demonstrate that this CE-integrated
   Engine workload meets an equivalent performance requirement, because no such
   requirement is stated for it. This exception leaves a genuine, if narrow,
   coverage gap, closed by Appendix A rather than by those gates.

8. **The comparator sensitivity result is sensitivity analysis, not a
   probability.** The comparator's five repetition-p95 values are
   `[1.3251, 1.3161, 1.3175, 1.5019, 1.5026]`. Removing any one of the two
   higher values leaves the ratio at 1.053–1.056 (pass); removing any one of
   the three lower values leaves it at 1.1265 (fail). This shows the estimator
   is unstable at this sample size. Five repetitions cannot support a
   calibrated probability of passing, and none is claimed.

## 5. Why an exception, and not the alternatives

- **Re-measuring the comparator** is forbidden by the execution matrix
  ("never historical baseline or experiment matrices") and would require owner
  authorization it does not have.
- **Changing the threshold or the statistic** is an oracle relaxation, which
  `plan.md` forbids ("no oracle relaxation"), and would destroy comparability
  with the retained Slice 72 record.
- **A product fix** would be a new performance optimization campaign, which
  `plan.md` forbids in this slice, and would rest on an unconfirmed mechanism
  for a ≈1.13 ms per-Engine-open cost.
- **Another attempt** is unjustified: two same-direction failures, a spent
  retry budget, and an identified deterministic cause.

`plan.md` Completion permits "explicitly authorized non-pass disposition." This
document is that authorization, on the record, with the cost named.

## 6. Terms of the exception

The exception is limited to the following and nothing else.

**Covered:** the `cuda` / `engine` `median_repetition_p95` comparison of the
retained Slice 72 CE profile, for release 0.8.25 only.

**Accepted cost:** a measured first-touch regression of ≈0.14 ms per reader
connection on CUDA and ≈0.16 ms on CPU — ≈1.13 ms and ≈1.25 ms per Engine open
— together with the worsened aggregate mean (+2.0% to +2.9%) and p99 (+7.3% to
+7.8%) of this fixture's workload.

**Also noted, not separately excepted:** the `cpu` / `engine` comparison passes
but has degraded from 1.0508 (Slice 72) to 1.0714 on the same mechanism. It is
recorded here so the trend is visible, and it is not re-dispositioned.

**Conditions:**

1. Both failed attempts are preserved unmodified, with their verifier logs, in
   `runs/0.8.25-slice-85/candidate-cuda-attempt1/` and
   `runs/0.8.25-slice-85/candidate-cuda/`.
2. The Slice 72 receipt and all historical cells remain unmodified. No result
   file is rewritten to resemble the final candidate.
3. This is recorded as an accepted non-pass. It is **not** a PASS, and the
   generated release-state views must not show it as one.
4. The settled-latency, touch-grouping and pooled-percentile figures in this
   document are **diagnostic evidence supporting the attribution only**. They
   are not substituted acceptance criteria and no gate is recomputed from them.
5. CE correctness, cross-device score parity and both `standalone` comparisons
   remain independently verified passes and are unaffected.
6. Every other Slice 85 obligation is untouched, including the fact that the CE
   profile produced no receipt.
7. Appendix A is scheduled as follow-on work, not performed in Slice 85.

## 7. What this exception invalidates

Stated precisely, because the same reasoning applies backwards.

**No longer usable as forward-carrying evidence:**

1. The `cuda engine` (1.096168) and `cpu engine` (1.050833) comparisons in
   `runs/0.8.25-slice-72/receipt.json`. They remain historically true for that
   candidate and stay unmodified, but they can no longer be cited as evidence
   that the Engine path carries no steady-state regression: they measured
   first-touch as well, and the CUDA margin was 0.35%.
2. Any statement that the Slice 85 CE profile produced a passing receipt.
   `ce-receipt.json` does not exist and cannot be produced without an oracle
   change.
3. The single-retry allowance for this cell, now spent.

**Explicitly not invalidated:** CE rank order and CPU/CUDA score parity
(`max|Δ| = 0.000000`); both `standalone` comparisons; the ungated cold-path
records; artifact, wheel and native hashes and their receipt bindings; the CUDA
UUID / PID / VRAM allocation witness; model identity before and after; and
AC-081a/b/c and AC-072, which are independent absolute gates.

## 8. Residual risk

Small but real, and not zero. Any workload that opens many short-lived Engines
and issues few searches per open pays the full ≈1.13 ms without earning the
settled-state return. This fixture cannot bound behaviour under a wider query
mix, where a bounded statement cache may evict and re-prepare. AC-072 and
AC-081 bound the amortised case well; neither bounds the many-opens case. The
corrected protocol in Appendix A is what closes this, and it is the reason the
exception is scoped to one release.

---

## Appendix A — Corrected CE benchmark protocol

The defect is that one statistic conflates two regimes with opposite behaviour.
The correction is to measure both explicitly and gate each on its own budget,
under an identical protocol for comparator and candidate.

This appendix is a proposal for follow-on work. **It is not authorized by this
document and must not be executed in Slice 85.**

## A.1 Principles

1. First-touch and fully warmed performance are separate contracts. Report and
   gate them separately; never let one statistic straddle them.
2. Warmup adequacy is derived from the Engine's actual reader-pool size and
   enforced by the verifier. It is never a constant assumed to still be right.
3. The comparator and the candidate must be measured by the same revised
   worker, at the same protocol version. Cross-protocol comparison is rejected
   by the verifier, not left to reviewer discipline.
4. Everything sound in the current protocol is retained unchanged: fixture,
   score tolerance, model identity, GPU UUID pinning, thread environment,
   affinity, offline policy, artifact provenance, and per-call output-stability
   checking.

## A.2 Worker changes

Three phases per path, all in one process so that CE and CUDA state are already
warm and only the reader pool varies:

- **Phase C — cold** (retained, ungated). Unchanged: one timed call on a fresh
  process. Diagnostic only.
- **Phase F — first-touch** (new, gated). For each repetition, open a *fresh*
  Engine, write and drain the fixture, then time exactly `reader_pool_size`
  consecutive calls. Because the pool is built per `open_locked`, this measures
  the cold-pool term deliberately rather than incidentally. Run it after a
  completed Phase W so that CE, CUDA and model state are warm and the pool is
  the only cold component.
- **Phase W — warm** (new, gated). Issue `warmup_calls` untimed calls, then
  time `steady_calls` calls. All timed calls are then genuinely warm.

Warmup sizing follows from the measured data: touch1 ≫ touch2, and touch2 is
still 2.6% above touch3. Two passes over the pool is not enough. Require
`warmup_calls >= 3 * reader_pool_size` (24 at the current pool size of 8).

## A.3 Manifest changes — `fathomdb.slice72.ce-profile-manifest/v2`

| Field | Change |
| --- | --- |
| `reader_pool_size_expected` | new; the pool size the protocol was sized for |
| `warmup_calls` | new; must satisfy `>= 3 * reader_pool_size_expected` |
| `first_touch_calls` | new; must equal `reader_pool_size_expected` |
| `first_touch_repetitions` | new |
| `steady_calls` | raise 20 → 100 (see A.4) |
| `p95_regression_ratio` | replaced by `budgets.warm.p95_regression_ratio` and `budgets.first_touch.p95_regression_ratio` |

## A.4 Estimator

Retain the `median_repetition_p95` shape — it is a reasonable
repetition-robust estimator — but give it enough samples to be a percentile.
At `n = 20`, nearest-rank p95 selects the second-largest observation. At
`n = 100` it selects the 95th of 100. The comparator sensitivity in §4.8 is
a direct consequence of the small sample, and raising `steady_calls` to 100 is
what fixes it.

Pooled p50/p95/p99 and throughput remain computed and reported, and remain
ungated and descriptive.

## A.5 Cell changes — `.../ce-profile-cell/v3`

Each cell records the `reader_pool_size` actually observed from the Engine
under measurement, plus the three phases separately. Recording the observed
pool size is what lets the verifier prove warmup adequacy rather than assume
it; it requires a non-test-hooks accessor, since `reader_worker_count_for_test`
is gated behind `test-hooks`.

## A.6 Verifier changes

1. Gate `warm` and `first_touch` separately, each against its own ratio.
2. Reject any manifest with `warmup_calls < 3 * reader_pool_size`.
3. Reject any cell whose observed `reader_pool_size` differs from
   `reader_pool_size_expected`.
4. Reject any cell set mixing protocol versions. This is the rule that would
   have caught the present failure mode.
5. **Warmup-leakage guard**: within Phase W, the median of the first quarter of
   timed calls divided by the median of the last quarter must be `<= 1.05`.
   Cheap, non-fragile, and it fails loudly on exactly the contamination
   diagnosed here.

## A.7 Required RED tests

Per `plan.md`, any new validator needs focused RED coverage for missing
coverage, zero-test/skip, stale artifact, relaxed threshold and false reuse.
Additionally, for this protocol:

- manifest with `warmup_calls < 3 * reader_pool_size` → rejected;
- cell whose observed pool size disagrees with the manifest → rejected;
- mixed-protocol cell set (v2 comparator, v3 candidate) → rejected;
- a Phase W series synthesised with a decaying head → rejected by A.6.5;
- a Phase F series shorter than `reader_pool_size` → rejected.

## A.8 Comparator obligation and cost

The revised protocol **cannot** be applied to the existing comparator cells:
they contain no phase split and were produced with a one-call warmup. Adopting
it therefore requires rebuilding the comparator artifact at `4fc1b890` and
measuring it with the revised worker, then measuring the candidate the same
way.

That is precisely what Slice 85 forbids, and it is the reason this is follow-on
work. It needs owner authorization for exactly one comparator rebuild, one
comparator measurement and one candidate measurement, serialized against other
timing work.

Cost is not the obstacle: a full profile run took about 13 seconds of wall time
(`ce-candidate-cuda.log`, 20:09:01 → 20:09:14). The total is dominated by the
one comparator artifact rebuild, on the order of minutes. The obstacle is
authority and slice scope. No broad rerun is justified.

## A.9 Out of scope, flagged separately

`cpu_affinity: [0]` pins the calling thread and all eight reader threads to a
single core, which serializes the pool and folds a same-core dispatch round
trip into every measured Engine call. This is a deliberate determinism choice
and changing it would further break comparability with the retained record. It
is noted as a distinct question for a separate owner decision, and is **not**
folded into this protocol revision.

---

## Appendix B — Reproducing the analysis

All figures in this document derive from five committed or preserved cell files
and are recomputable read-only, with no measurement:

| Role | Path |
| --- | --- |
| comparator cpu | `runs/0.8.25-slice-72/baseline-cpu/baseline-cpu.json` |
| comparator cuda | `runs/0.8.25-slice-72/baseline-cuda/baseline-cuda.json` |
| candidate cpu | `runs/0.8.25-slice-85/candidate-cpu/candidate-cpu.json` |
| candidate cuda, attempt 1 | `runs/0.8.25-slice-85/candidate-cuda-attempt1/candidate-cuda.json` |
| candidate cuda, attempt 2 | `runs/0.8.25-slice-85/candidate-cuda/candidate-cuda.json` |

The gate is reproduced by applying `nearest_rank(durations, 0.95)` per
repetition and taking the median across repetitions, exactly as
`verify-slice72-ce-profile.py:211, 219, 286-290` does. The touch grouping in §2
assigns measured call `i` to touch number `(i + 1) // 8 + 1`, following the
round-robin dispatch at `lib.rs:2655` with the single warmup call at
`slice72-ce-profile-worker.py:173`.

The run manifest `runs/0.8.25-slice-85/slice72-ce-manifest.json` differs from
the sealed `../slice-72/ce-profile-manifest.json` in exactly one field — the
`candidate_sha` repin from `2e14f5ba` to `3b66e1b4` — which is the legitimate
Slice 85 candidate substitution. Fixture, thresholds, environment, affinity and
GPU selection are byte-identical.
