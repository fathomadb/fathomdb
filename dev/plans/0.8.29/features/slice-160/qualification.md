---
title: Slice 160 qualification matrix
status: PROPOSED
target_release: 0.8.29
---

# Slice 160 qualification matrix

[Active owner dispositions](requirements.md#owner-dispositions) select .29
identity/atomic-feedback/source-seam work, hold real resets to .30 and move new
x86 CUDA13 delivery/quantitative evidence to .31. Seq-381 additionally moves Tegra CUDA13
130A to .31;130D/E and existing CUDA12.6/platform/artifact obligations remain .29; deferral is not qualification. Owner seq-387 excludes non-Tegra ARM+GPU B/C from every .29 gate and known-working environment claim.

Rows retain their existing contracts/routes; [status](status.md) records the
current independent execution batch and its actual outcomes. Source references are [canonical requirements/AC](requirements.md),
[.28 execution inventory](../../../0.8.28/features/slice-95/qualification.md),
[current .28 closure](../../../0.8.28/features/slice-95/status.md),
[release design](../../../../design/release.md), [master](../../../plan-0.8.29.md)
and [reconciliation](../../reconciliation-plan.md). Current scripts/tests and
accepted successors control any stale historical wiring.

## Required qualification and freshness

| Row / local acceptance | Existing route and meaningful oracle | Freshness / current hold |
| --- | --- | --- |
| Q1 / A160-1 | Selected slices' actual review/exits plus condition register below; reconcile source/git and raw evidence, not run-state narration. |Current accepted exits include20/60/70/80/100 and selected110/140 source/healthy/OOM/consumer scope. Remaining selected measurements and consumers stay open; see status. No separate status-copy packet. |
| Q2 / A160-2 | `scripts/agent-verify.sh` full default: owned tools, workspace clippy/check, security and actual registered tests. Prepare clean final tree; preserve lint→type→test short-circuit. | One final code-phase broad run, scoped correction/resumption afterward. Accepted assembled source results retain their exact inputs and limits; final integrated .29 applicability still requires assessment, not automatic full replay. |
| Q3 / A160-2 | Current `scripts/check-test-target-coverage.py` inventory and `scripts/test-feature-complete.sh` with pinned live BGE/reranker/nomic/ONNX/available3090 requirements; actual missing assets fail. | Currentnormalcampaignisassembled-qualified:actual30runs/207passes/23ignored/0runtimefailures, reviewed5176860metadata/parsercorrectionandretained-logrescan. OriginalEXIT1/10metadatafailuresretained;12unchangedoptin-exclusionlistingchecksreuseoriginalnostaleevidence. Ignoredmanualscopesretainseparatevenues/proofs;final90affectedcoverageisreassessed. |
| Q4 / A160-2 | `STRICT=1 scripts/agent-security.sh`; real AC036/037 plus maintained security controls;0violations/0blockers/0downgrades. | The160 batch rebuilt current locked security_cycle and passed both missing canonical AC037 live scripts without downgrade or host changes; see status/logs. Combine only with applicable accepted unchanged other controls. This is scoped completion, not a newly executed aggregate battery; reassess changed90 inputs. |
| Q5 / A160-2 | AC011a/b existing `scripts/run-ac011-write-throughput.sh`:1KB>=1000/s,100KB>=100/s, existing5s warmup/60s measurement, canonical x64/NVMe. | Retained unchanged7d release binary passes current1482.090/s>=1000 in the owner-directed fixed OLD1495.233→NEW1482.090 comparison under one common environment/TMPDIR/profile/args, with contemporary1s context and time-v. Original760.621/993.996 failures and instrumented765 attribution remain; no statistical cause claim or production optimization. Independent first-current b passes429.322/s>=100. Exact subjects/raws/limits are in status. |
| Q6 / A160-2 | Release isolated `perf_gates::ac_012_text_query_latency_on_fts5_path` N10000:20/150ms; `ac_013_vector_retrieval_latency` N10000/dim768:80/300ms, accepted sample protocol. | Currentclean686d releaseN10000 observations pass:AC012p50/p991/3ms;AC013dim768p50/p9963/73ms,1000timed samples each. Independently accepted; no concurrent compilation, DCPUnoise disclosed underseq390. Final90 applicability remains. No100k thresholds as binding verdict. |
| Q7 / A160-2 | AC073 accepted real-corpus stress, bound=max(10×baseline,150ms). Retain legacy EU7 overall exit and superseded recall assertion independently. | Currentclean686d realcorpus stress passesN1000p9943ms<=150 andN7667p9946ms<=316,independentlyaccepted. EU7overallexit101/legacyrecallfailure remains separately unqualified; currentTC5F/Q8 is the accepted recall evidence. No synthetic stress substitute; final90 applicability remains. |
| Q8 / A160-2 | AC075 accepted `experiments.tc5_gpu_v2` frozen bridge:7667 real docs/100queries/K192/top10/exact-f32 vector truth; CI-high>=0.90. | Fresh applicable .29 candidate-bound tool/native/model/input proof. Owner seq382 supplies /mnt/data/fathomdb-performance-benchmarking/scale-02/inputs/input-pack-v1; registered index/manifest/all17372payload hashes and7667bridge identity pass static validation. Missing-input hold cleared; currentdc8f F/Q8 runtime is independently accepted:7667real/100queries/K192/top10/exactf32,recall.958/CI[.938,.974]. Final90 applicability remains. No regenerated or10k convenience substitute; legacy EU7 recall not replacement. |
| Q9 / A160-2 | AC081a/b release absolute batch <=500ms sequential/<=100ms eight-reader; current seven-process protocol. AC081c real distinct-reader progress remains separate correctness witness. | Current retained7d release seven-process cohort passes all500/100ms hard limits: sequential324.699516–345.138117ms, concurrent47.880991–50.739633ms; all7 sequential200ms warnings retained. Actual profile/source inputs remain explicit; no retired ratio verdict. AC081c ordinary source correctness scope remains parent-owned. |
| Q10 / A160-2 | Required long AC02160s warning-free schema/DDL and1000-iteration cursor fixture with positive writes; use `AGENT_LONG=1 scripts/check.sh` or reviewed exact unchanged selectors. | Current retained operator/AGENT_LONG release controls pass AC02160s/60 add-drop cycles/60 rebuilds/zero schema warnings and cursor1000 searches/212 writes/211 during reads/999 advancing hits/zero violations. These are actual long executions, not smoke or source-error-fix binary retagging. |
| Q11 / A160-3 | Real engine/SDK/MCP all-tools/error/filter-before-cap/flags/queue-close/budget/paging controls; conformance under text and structured; accepted independent field/UTF-8/span/coverage oracles. | Full90 source261c5698 is independently accepted and integrated, with current artifact boundary checks in Q14/Q15. Accepted50/60/70 and unchanged control scopes retain actual applicability; parent-owned integrated conformance/model and broad source exits remain distinct. |
| Q12 / A160-3 | Block1 exact approved12 smoke+60 item4 episodes on final integrated90; same-page bytes separately from end-to-end tokens/cost/accuracy/paired interval. | Owner seq392 permits90 source entry with actual qualified20/50/60/70 source fit. Required Block1 measurements and exact72 once on final90 remain local-close gates.40 conditional disposition is complete with no new treatment under the existing approval; no extra owner-choice stop applies. Actual final222/b3c6 candidate completed the single approved72 with all144 grades, exact served models/tool policy and no invalid/unknown results; independent Astra final ACCEPT. Reported total$0.13444673. Paired intervals and separate renderer/API metrics are in the experiment README; no universal accuracy or noninferiority claim. No further episodes are inferred. |
| Q13 / A160-2/4 |100 existing eight crash controls plus active-checkpoint/richer postcrash mapping; real survivor/history/projection/span/fence/WAL/reopen/retry proof. | Accepted coverage/native/G2 results are reused within their actual scope; seq386 active-checkpoint interruption/reopen is qualified and independently accepted, closing selected100. Coverage inventory/review alone cannot close it. No machine/power-loss or migration campaign. |
| Q14 / A160-4 | Five CPU targets: Linux x64/ARM64, macOS x64/ARM64, Windows x64; build/test, fresh installed Python/Node+strict TS consumer, wheel-size and native receipt collector. Distinct Windows WAL; external Rust archive consumer and CLI. | Fresh d226 Linuxx64/ARM64 and Windows CPU wheel/addon pairs pass actual ABI3/floor/package/strict-TS/native-digest boundaries, maintained3 installed Python+3 N-API corrupt-profile/repeated-refusal/restored-reopen cases per venue and existing collectors. Original1349 healthy62/210 and earlier WAL retain explicit unchanged successful-path applicability, not new execution identities. Fresh d226 source-archive SDK/CLI consumers pass. Current hosted e192 five-target installed consumers and native receipt collector pass, including both macOS targets; registry consumers remain Q19. See status/state for exact producers and limits. |
| Q15 / A160-4 | AC082a–d exact x64 artifact/member linkage, driverless auto/cpu lifecycle, forced CUDA typed negative, selected GPU PID/UUID forward and explicit CPU nonuse; existing installed consumers/negative controls. | Fresh d226 canonical CUDA12.6 wheel2.28/addon2.39 pass ABI3/floors/all-member linkage,3+3 affected installed cases, driverless auto/CPU BGE lifecycle and2 actual forced typed refusals. One fresh exclusive bounded GPU batch passes exact new-digest2 allocation/PIDUUID witnesses (each134217728B>=67108864B) and4 AC082d CUDA-positive/CPU-negative cells with complete traces, exit0/close/certain cleanup, final compute empty and independent physical-FD9 release. Unchanged reranker/provider proofs retain explicit successful-path applicability. Prior foreign hold/old1349 GPU evidence stay separate; no reset/new toolkit/full-readiness claim. |
| Q16 / A160-4 | Selected110 source/seam remedy and applicable healthy lifecycle controls, current artifact/library/PTX/selection, independent130 D/E local SDK pool/lifecycle/soak protocols and included140 pins. |Seq-376 policy selected; seq-377 live reset children HELD .30 and no reset-survival/original-defect closure. Seq-378 new x86 CUDA13 delivery/compiler work is .31; it is not required for .29. Selected130 D17, shipping consumers/build equivalence and the declared instrumented semi-space4 E pair are independently accepted; original default-profile failure and monitoring deviation remain disclosed. Final90 artifact applicability remains a combined qualification assessment. Seq-381 explicitly defers Tegra CUDA13 A to .31; D/E/current CUDA12.6 remain .29. |
| Q17 / A160-5 | Scoped interfaces/parity/surface/version/docs/lifecycle/state gates, strict MkDocs where public docs affected. W0.8.29, independentE; notes disclose accepted source breaks/default and support limits. | After actual changes; .29 routing repair already in10, external exact-tag checks occur when applicable. Metadata-only changes need no second full source gate. |
| Q18 / A160-5 | `scripts/release/local-dry-run.sh` and distinct `release.yml` immutable-candidate dry-run; actual package/auth/routing/required job outcomes. | Current7d035022 local steps1–4 PASS; original missing-note and offline-step5 failures retained. Only the3 canonical leaf publish dry-runs resume online and PASS/abort upload. Assembled validatable local completion is separate from hosted rehearsal, dependent publishers and registry consumers. HITL seq-394 omits three additional CUDA rehearsal jobs. Current hosted CI and scoped local completion are independently accepted as assembled qualification; original cancellations and Rust101 remain retained. Actual production delivery is authorized by seq-396/397. |
| Q19 / A160-6 | Required publishers and fresh published-version Python/npm/Rust SDK/CLI consumers, persisted read/write, missing record, separate-process reopen, close/exit. | **AUTHORIZED AND PENDING under HITL seq-396/397.** Actual product publishers and fresh registry consumers are required before closure. Candidate installs cannot close this post-publication row. |

## Conditional inclusion and no-run criteria

| Scope | Condition and automatic execution consequence | What still blocks |
| --- | --- | --- |
|40 cache/batch | If30 shows no workload benefit or reviewed format37 snapshot safety fails, no optimization implementation; preserve baseline and report result. | Missing30 measurement/safety assessment is unknown, not negative evidence. Selected independently justified batch work retains its own exit. |
|50 identity/text | Approved work retains actual30 baseline, source/consumer census and selected seq-374 identity before changed fusion semantics. | Identity selected by seq-374 and implemented source independently accepted; actual current paired measurements and exact candidate/baseline inputs remain. Private Memex population census is optional frequency/benefit characterization, not a correctness or source-entry prerequisite. |
|90 residual B2 | Build only approved80 residuals; already-built/satisfied cases need preservation rather than duplicate code. | Full80 approval seq384 and source-entry seq392 are recorded; full90 source261c5698 is independently accepted/integrated. Remaining Block1/model/40 and final affected qualification are local-close gates, not retroactive pre-90 source prerequisites. |
|100 interruption | Reuse actual unchanged mapped evidence; no additional fault run for obligations already discharged. | Seq386 active-checkpoint interruption/reopen is independently accepted and closes selected100. Seq371's narrow verification exception is explicit; actual defects still require runtime RED/GREEN. |
|110 live reset | Seq-377 explicitly holds Orin/3090 child reset execution for .30 review; .29 source/seam repairs remain selected. | Reset-survival and original TC-281155a1 closure remain unqualified; no mandatory .29 live-fault gate. |
|120 new x86 CUDA13 delivery | Seq-378 moves new delivery/compiler-safety work to .31; seq-379 moves quantitative tests/evidence there with bounded unknowns/current ratings accepted. | No accepted package choices, CUDA13 build or qualified safety; existing artifacts and130D/E remain .29; seq-381 separately allocates Tegra CUDA13 A to .31. |
|130A Tegra CUDA13 | Seq-381 explicitly moves existing TC-b267d940-1add-4d08-a596-a7669ae4a500 to .31; preserve CloudJetson vetting/venue/caps and actual artifact/consumer proof there. | No actual qualification or publication authority; A is not a mandatory .29 gate.130D/E and existing CUDA12.6 obligations remain .29. |
|130B/C non-Tegra ARM+GPU | Owner seq-387 removes this unavailable configuration from known-working environments and all .29 gates. | Unqualified, not passed; no cloud procurement, initialization study or future release is required/assigned for .29 close. |
|140 fork A | The110 A repair is selected; reuse accepted cudarc/Candle/product source, healthy, OOM and consumer proof within its actual scope. No broader fork maintenance or new CUDA13 work follows. | Selected110/140 scope is independently accepted; final distributed-artifact applicability remains mandatory. No reset-survival or original live-defect closure is inferred. |
|Fork B/C | Already moved to.30 by owner seq367. No .29 run or fresh deferral vote. | A concrete necessary A repair cannot silently expand into B/C. |
|150 optional studies | No actually selected authorized study means no experiment/spend/product promotion and no product-readiness dependency on execution. Preserve checkpoint items as unselected/open, not owner-deferred by inference. | Any actually selected study needs its preregistration/budget/fixtures/verdict; product promotion needs explicit placement. A checkpoint disposition request is distinct from a mandatory runtime gate. |
|Unpromised cells | No musl/32bit/WindowsARM/new toolkit/2M/fullCPU mirror or calibrated-verdict program arises from160. | Required Tier1/selected130 rows cannot be relabeled unpromised to bypass failure. |

A required-row exclusion requires its existing contract/authority; absence of
resources is BLOCKED/UNMEASURED. Routine conditional no-run outcomes above need
no new blanket human approval. No exclusion is described as PASS.

## Current hosted-rehearsal disposition

HITL seq-394 disposes the proposed additional three CUDA release-rehearsal jobs
for this nonpublishing objective. Accepted local CUDA/package/GPU proofs retain
their original subjects; current hosted exact-candidate CI supplies the remaining
five-target/macOS installed-consumer outcomes. GitHub CUDA runner admission,
rebuild/transport and downstream release orchestration remain unperformed, not
a passing `release.yml` result or READY_TO_PUBLISH claim. Future publication and
registry consumers retain their separate authority and applicable readiness gates.
Seq-395 grants one protected-main rehearsal exception; it is currently unused
and does not itself grant new runner registration or revive the omitted runs.

## Production publication authority

HITL seq-396 authorizes publication of0.8.29. Seq-397 authorizes one temporary
production CUDA worker, exact-tag route and reviewed protected-main exception
under the existing job and cleanup bounds. The omitted three rehearsal jobs stay
omitted. Fork registry dependencies are published and resolve-only verification
passes. Current hosted five-platform consumers and assembled source qualification
are independently accepted; the two cancelled verifier jobs and original Rust101
remain disclosed. Actual shipping builds, product publication and fresh Q19
registry consumers remain pending.
