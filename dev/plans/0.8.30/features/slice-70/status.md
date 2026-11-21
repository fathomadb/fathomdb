---
title: Slice 70 integrated source qualification
status: COMPLETE
slice: 70
target_release: 0.8.30
---

# Slice 70 integrated source qualification

Entry `ef254b2ab21cb293633ad8252b4050d8716550dd`; isolated branch
`impl/0.8.30-slice70`. Astra-medium passed the updated whole plan/design after
one consolidated FIX-1. Planning commit `8cab7ffca34cbd13d5d84ebb139794bd2a76b75a`.
Parent owns canonical release state, integration and cleanup. All selected
source-qualification rows are complete, including the two current-pin CUDA
rows executed jointly with75 on candidate `6d0e6c37b6cf35e3fecfe93ae3859a32d1b94a8f`.
Original failures and scoped recoveries remain as recorded. This is not final
platform/artifact readiness or a publication verdict.

## Source, security and coverage reuse

Reuse 50's clean484 source phase and three exact recoveries at 1550b96bb:
241 registered/238 ran/235 passed/3 failed/3 explicitly unselected G/M/D live
controls skipped; original gate exit1 remains. Checker16, original PyO3 lib46
with owned LIBDIR, Python approved-API boundary/parity4 all recovered. Strict
AC036/037 actually executed with 0 violations/blockers/downgrades. Later70
changes are confined to MCP rendering, private TC5 qualification and EU7
fixture/test-support/stress-example qualification; affected checks are separate,
without an unnecessary full-source replay.

Git comparison against 55's866dcad source shows model-independent canonical,
projection, query/schema, CLI and MCP implementations unchanged except50's
reviewed reranker ownership/errors. Reuse 55's actual cohesion/model, temporal/
graph, bounded real product Proptest, crash/WAL/persistent failures,17 doctors,
shared SDK/MCP and sensitivity witnesses within those boundaries. Each rejects
concrete wrong state: swapped source/revision/vector ownership; endpoint/cap
leakage; malformed authenticated cursor admission; lost committed/pending work;
false integrity/export/recovery success; or mismatched identity/Unicode/budget.
The new MCP formatting qualification below includes its shared behavioral fixture.

Reuse 50's runtime/logging and actual new Rust/Python/Node release/Busy/reload,
Orin loss/recovery and factual Node comparison. Original profile UNMEASURED,
ON slope0.068735 PASS/OFF2.158951 FAIL remain, without a plateau or causal claim.
Reuse 60's exact3509cd6b cudarc/952edb7d Candle/kernel owning CPU/native/fault/
recovery/OOM/public-five-crate proof. PTX0 remains default. No native fault,
3090 access or fork publication is repeated here.

Numerical branch coverage stays unmeasured;55's manual map and three killed
mutations retain their limits. No power-loss, broad coverage-guided or native
FFI/sanitizer/MC/DC claim is added. Erasure remains a critical-regression scope.

## Actual omitted CPU qualification

Final engine subjects use locked Rust1.95 release, with `operator,test-hooks`
for the exact long/latency selectors; AC011 uses its unchanged canonical helper
and default-feature subject. Timing ran after compilation, on windchill3/NVMe.
No quiet-host or CPU-isolation claim is made. Accepted limits stay unchanged.

| Selection | Actual result |
| --- | --- |
| AC021 exact long body |1 passed;60-second window,60 projection add/drop cycles,60 rebuilds,0 SQLITE_SCHEMA warnings. |
| AC059b exact cursor body |1 passed;1000 searches,229 committed writes/228 overlapping,999 advancing hits,0 cursor/row-count violations. |
| AC012 N=10000 |1 passed;1000 measured samples, p50/p99=1/4 ms. |
| AC013 N=10000/dim768 |1 passed;1000 measured samples, p50/p99=64/78 ms. Scale/full/drain overrides unset. |
| AC011a/b canonical5s+60s |Both passed:1042.915 commits/s at 1KiB and 266.831/s at 100KiB. |
| AC081 seven fresh processes |All 500/100 ms hard limits passed: sequential328.317–359.595 ms, concurrent48.950–58.121 ms; all seven sequential200 ms warnings retained. |
| AC081c reader progress |Original release invocation selected0 tests because reader_pool is debug-only, hence NOT EXECUTED. Only that exact missing body recovered with retained debug subject:1 passed,6 filtered, positive independent reader progress. |
| Feature inventory |Static checker passed412 targets/45 gate-only targets; this is inventory, not runtime credit. |

Exact raw commands and actual subject paths are in the build/log files and
`selected-exits.jsonl` under `/tmp/fathomdb-0830-slice70-evidence`.
AC081's maintained identity-bound driver produced each separate process log.

## Hints remediation

The unchanged200 us criterion first failed at 201.8 us (limit10:55.2 us median,
58.9 us p99). That invocation used retained debug subject `c49bf6682a1511af`;
its producer SHA is not independently bound by the raw log. It is not presented
as a rigorously paired causal benchmark. Earlier289.9/366.8/345.3/374.6 us
failures and 182.124 us diagnosis remain separate.

Inspection found repeated full-response serialization while only budget digits
change. `tools::finish` now serializes the non-budget object once and iterates
only exact budget length, preserving key/comma/UTF-8/escaping and both structured
copies. Compact counting and all existing test oracles stay unchanged.
Astra passed the focused design; Sol passed the corrected code. Commit1242ec2fe.
Actual rebuilt debug unchanged test passed67.8 us at 100; limit10 warnings29.8 us
median/82.2 us p99 remain. Read-only affected tests passed: library26, formats20,
hints15, historical byte/budget oracle6, allocation1, performance1. Scoped
all-target MCP Clippy and formatting passed. No broad workspace replay.

## Current model-dependent feature proof

Execute only affected bodies after the Candle change:23 selected invocations,
78 top-level passes,0 failures/ignores/visible SKIP. Build logs bind current
source and 952edb7d; invocation uses explicit CPU/no visible CUDA, pinned copied
cache, REQUIRE_LIVE=1 and no SKIP_NETWORK control. Original assets remain intact.
Runtime/model/tokenizer SHA values matched the maintained ONNX pins before use.

Actual selections cover BGE identity/unit-norm/determinism/batch/truncation,
role-marker equivalence and UTF-8 coverage; reranker batch/per-pair/chunk/empty
semantics; actual ONNX CPU/CLS equivalence and calibration; default model/open
reports, Nomic, context-pack fusion; engine passage/CE/rerank policy; MCP default
embedding and vector mismatch; CLI warm-cache and reranker CPU diagnostic.
ONNX measured45 probes/17280 bits with 0 flips; CPU calibration retained the
same real probe count. Nomic produced768 finite unit-normalized values and
semantic cosine controls. Four context-pack Proptest persistence-path warnings
are retained. These prove CPU feature claims, not unexecuted CUDA calibration.

## Real-corpus stress and fidelity

The one current CPU EU7 run executed the original13 JSONLs/96,704,727 bytes,
18472 loaded real records, N=1000/7667,100 queries,1000 bootstrap/latency samples
and 8x250 stress. Actual exit101 after 3671.16s,0 passed/1 failed:

| N | Full-search p50/p99 ms | Stress p99/bound ms | Legacy vector recall /95% CI |
| --- | --- | --- | --- |
|1000|72/91|4156/899 FAIL|0.877 /[0.854,0.897]|
|7667|88/153|4270/1161 FAIL|0.775 /[0.746,0.801]|

The real-corpus7667 p50 misses80 ms; p99 clears300 ms. Legacy CI-high remains
below0.90, distinct from the accepted frozen TC5 bridge. No threshold waiver,
quiet-machine wait or unproved noise/lock attribution. `eu7-current.log` and
`eu7-current.json` retain every original verdict. A drained7667-node/vector
SQLite snapshot was preserved once using online backup during the original
run; diagnosis does not seed again.

The reviewed `slice70_stress_compare` experiment isolates one optional provider
mutex on fresh identical copies, explicit CPU, identical delegated provider
methods, canonical100 query texts,15-candidate override/default10 results,
1000 warmups/1000 baseline and 8x250 stress. Each measured result rejects
fallback/degradation and must match its query's ordered untimed witness;
witnesses also require actual pre-fusion vector candidates. Check witnesses
across arms before interpreting latency. This is diagnosis, not a replacement
PASS for the failed canonical body. The first setup attempt failed before timing
because the count connection initialized SQLite before runtime configuration;
that failure is retained. Actual matched arms used byte-identical witnesses, query digest and snapshot:
serialized baseline107843 us/stress4589962 us/bound1078438 us FAIL; direct
107761 us/184883 us/1077619 us PASS. Removing the outer harness lock is supported;
mutex unfairness is not established. The smallest repair shares `Eu7Bge` through
test support, retains historical batching/truncation defaults and every canonical
query/ground-truth/assertion. A single affected stress-only run against that exact
repaired provider passed at 7667:baseline105206 us,stress184865 us against
1052062 us bound. All 4000 searches preserved witnesses and no fallback; its
witness file byte-matches both diagnostic arms. The selected1000-row recovery
also passed:baseline88688us,stress174461us,bound886886us. Its exact ignored
`eu7_repaired_provider_small_stress_only` body executed1 test in321.29seconds,
reusing canonical corpus/query/seeding/drain/stress helpers and the repaired
historical provider. It seeded only1000 records, verified1000 canonical/vector
rows and a positive explained vector witness, then used1000 warmups/1000
baseline searches and8x250 stress. No ground-truth/recall or7667 reseeding.
The unchanged canonical timed sampler is retained. The original overall EU7
exit stays failed; these are focused stress-row recoveries.

## Orin TC5 remediation and actual limits

The standing seq412 route uses the frozen seq382 bridge:7667 documents,
100 queries,K192,top10, exact-f32 and unchanged CI-high>=0.90. Actual50 wheel
is `0.8.29+tegra` with SHA6d042455cdfad87a47827a440a703a29e9213523dd519298f7394b4afa760524;
it is a development source proof, not a final75 shipping artifact. Exact model,
input pack and CUDA UUID remain pinned in retained configurations.

Focused genuine RED→GREEN repairs, independently reviewed by Astra-medium
for design and Sol6.2-high for code:

- .30 package-cohort admission:5 actual failures→34 controls passed.
- ARM factory admission:Auto!=On failure→14 private-executor unit passes;
  explicit3GiB typed pool, early initialization, pre-weight probe and passive
  postconstruction observation. No producer PTX default change.
- A known child exiting between checked GPU observation and a second lookup:
  real CPU parent/child/reap RED→4 ownership controls pass. Count the checked
  snapshots conservatively; unknown or foreign records still reject.
- Explicit completed-ingest recovery:byte/manifest/WAL/count/model/executor
  validation; preserve ingest wheel/configuration separately. Old and new
  executor digests are distinct. No prior query result is used as fresh proof.
- Bare valid Tegra UUID normalization and forbidden pinned-CUDA→CPU fallback:
  each actual ARM RED recovered;16 units, scoped all-target Clippy and final
  private executor build passed. The historical contradictory fallback test
  now covers only unpinned CPU; the new pinned-error oracle stays unchanged.
- Typed nonmeasurement and missing-resume fail-fast:actual100-versus1 and
  launcher preflight REDs recover; reject before new ingest/output/lease.
- Engine open changes SQLite operational metadata:real SQLite query2 hash RED
  recovers by giving every process a fresh hash-checked input copy. Resume
  recognizes exactly the two known first-manifest layouts but copies only
  immutable ingest bytes. Each query copy is retained; no ingest replay.

Final affected Python tests:18 passed in0.32s; Ruff and Rust formatting passed.
No full gate or previously passing native fault campaign is repeated.

| Retained cell | Actual result |
| --- | --- |
|`tc5-final`|Complete7667-node/vector GPU ingest;6 typed nonmeasurements, then observer second-lookup ProcessLookupError. Failed.|
|`tc5-recovery`|Reused immutable ingest;100 typed device-preflight nonmeasurements because Tegra SMI UUID lacked GPU-prefix. Failed, no fidelity measurements.|
|`tc5-device-recovery`|First actual CUDA query completed, recall1.0;query002 executor failed because opening the previous query changed the shared main-file hash. Failed.|
|`tc5-isolated-query-recovery`|77/100 measurement_complete; observer then saw PID3358714 disappear before its first ownership snapshot and correctly aborted. Original wrapper failed.|
|`tc5-native-completion`|Only 078–100 executed, each native process pinned directly as the guard leader. All 23 guards exited0 in3.2–4.25s; assembled100-query fidelity proof passed.|

All 100 current measurements use7666 eligible rows,K192/rerank10,expected
CUDA UUID, direct vector route1 and search/FTS/fusion/graph/cross-encoder routes0.
The completion driver reconstructs every frozen ordinal's manifest/spec and
checks query/settings/selection/fixture/index/model/runtime/seed/executor binding
before the unchanged aggregate.91 distinct query texts among100 ordinals are
part of the frozen pack; text uniqueness is not the identity oracle.
Actual recall 0.958,95% CI [0.938,0.974],1000 bootstrap resamples,
sigma 0.009210423443: unchanged CI-high>=0.90 PASS. This is assembled fidelity
proof, not an uninterrupted wrapper or resource-protocol PASS.

The direct route was added only after rethinking the repeated observer race.
It keeps the actual fixed native executable as the unreaped leader, checks nested
owned manifest/database paths and rechecks the shared deadline before dispatch.
Batch cancellation forwards to its active guard and waits for native cleanup;
real CPU parent/guard/worker and foreign-database admission each supplied a
failing assertion before repair. Eight affected controls passed after FIX-1 and
Astra/Sol re-review. Only 23 missing queries ran; no ingest or77-query replay.
Each native cell has120 seconds and shares one30-minute outer budget. Unknown
observed peers still abort. Owned cleanup completed and a separate subsequent
fuser observation was empty.

All recorded resource samples were within stated RAM/swap/disk/RSS bounds.
The23 new cells have81 samples, maximum recorded RSS613638144 bytes; GPU-used/
free remain unavailable and cadence about1second. No total GPU byte or
between-sample excursion claim. Original cell exits remain failures.

Native query producer0efd5142d2b062ef1993a8c59b35bba3de299093;
query-runner copy fix40bb73356; direct custody/completioneb7befcae; original
ingest producer9af27adb. The tested
query executable SHA is c02146c857f9a2a8feea4862547e3f118b96ea4230f58a2aa99ba202210e488c.
Local raw/native retention: `/tmp/fathomdb-0830-slice70-evidence/orin-final-raw`,
`orin-tc5-final-raw.tar.gz`, `fathomdb-tc5-benchmark-device-fix`. The earlier
`orin-raw` and `orin-tc5-raw.tar.gz` remain intact. Original ingest and
all runtime database copies remain under `/tmp/fathomdb-0830-slice70/` on Orin.

## Remaining mandatory qualification and handoff

1. AC073 selected stress rows have passing scoped recoveries at1000 and7667.
   Original full-body failure, legacy recall and all numerical limits remain.
2. AC075 is complete as assembled fidelity proof; original wrappers remain failed.
3. Under renewed3090 availability seq-415, actual current-pin calibration passed
   both selected tests with45 CLS probes. CPU↔CUDA and CUDA↔ONNX cosine means
   and minima printed1.000000000; raw/centered flips0, maximum L2 about1.30e-6.
4. The actual source72 basic/overlap bodies passed2/2 with no skips or guard
   abort, real BGE/CE work and UUID/PID/memory receipts. Both commands exited0;
   postflight observed no compute or graphics peers and the unchanged driver
   and primary CUDA12.6. The established per-UUID lease was used.

Terra independently executed these exact bodies on the frozen candidate.
Actual test binaries contain the maintained sm75 PTX families and PTX0 policy;
inspection also identifies sm75 static cubins separately. This closes the
source-feature requirements, with no transfer or waiver. It supplies no byte
identity or complete-kernel claim for future shipping wheel/addon/CLI builds.
Raw commands, results and receipts are retained at
`/tmp/fathomdb-0830-slice75-evidence/terra-cuda`; final artifact qualification
remains75. Parent Codex inspected the actual pass logs and architecture scope
and accepted70 closure.

75 retains final installed platforms/artifacts, Windows WAL/hosted cancellation,
Node support, renderer and public-contract/docs convergence.80 retains actual
product publication and registry consumers under its later exact-candidate grant.

## Reviews, focused validation and retained subjects

Updated whole plan/design: Astra PASS after one FIX-1. Every focused design and
code repair has Astra/Sol PASS after its stated correction; no public API,
storage-format, dependency-pin or product requirement changed. Terra independently
verified the CPU selections, actual failures, all100 native query bindings/routes
and the deterministic bootstrap. Parent Codex passed coherent source integration and the final shared CUDA
closure. Canonical release state remains parent-owned.

Final changed Rust scopes `eu7_real_corpus_ac` and the private stress example
passed scoped release Clippy in25.26seconds; source checks remain scoped,
without another full gate. Python affected controls
are18 passing cases across TC5 resume, ownership and completion; Ruff passed.
All existing failures, skips, warnings and setup errors are retained separately.
`./scripts/agent-lint-md.sh` passed; the full source gate is reused above. Raw source/security/model/long/numerical
proof stays under `/tmp/fathomdb-0830-slice70-evidence`; actual repaired CPU
subjects are `slice70_stress_compare-historical` and `eu7_real_corpus_ac-repaired`
there. Prior50/55/60 evidence paths in the commissioning handoff remain reusable
within the source/feature/native boundaries above.
