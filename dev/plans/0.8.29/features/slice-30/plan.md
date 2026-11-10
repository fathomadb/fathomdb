---
title: 0.8.29 Slice 30 — current scale and access-path measurements
status: PROPOSED
target_release: 0.8.29
slice: 30
---

# Slice 30 — current scale and access-path measurements

## Current checkpoint scope

Scope approved in Slice9 Batch3: current measurements with existing targets. Owned setup and dependable20 tools/tests remain hard inputs; no baseline pass is inherited.

The [decision log](../../prework/slice-9/decision-log.md) owns current rulings;
original draft observations below retain their historical evidence limits.

## Goal, maturity and boundary

- Establish reproducible current-format measurements that guide Slices 40/50 and
  Block 1 follow-up qualification; this draft authorizes no runs or product fixes.
- Follow the [brief](../../planning-brief.md), [reconciliation](../../reconciliation-plan.md)
  and [release plan](../../../plan-0.8.29.md). Baseline is format 37, .29
  `6edc1155f039f87cd0a37660cb27ae4373bbce71`, including published .28.
- MCP/queue/Block 1 are integrated. CS64, bounded pending scans, native vector
  lifecycle/validity and FTS rowid identity shipped; do not rebuild these fixes.
- [Historical W4](../../../agent-access-block1-finish-plan.md) and the
  [recommendation](../../../../experiments/ac013-page-cache/RECOMMENDATION.md)
  contain old-contract evidence: LME 100k hybrid 283→149→54ms was a prototype comparison, not a current baseline or proof of write-interleaved benefit.
- [.28 Slice 60](../../../0.8.28/features/slice-60/status.md) measured bare vec0
  chunks64/128/256 through 2M vectors. It did not settle the old CS64/1024
  crossing; no new ANN, layout, page-cache budget or ranking change belongs here.

## Preparation: draft needs, requirements and acceptance

- Draft `requirements.md`, `acceptance.md`, `design.md` and `test-map.md` beside
  this plan during authorized preparation; local labels below create no canonical ACs.
- N30-1/R30-1: developers need an attributable 100k natural-text baseline; record corpus selection, query match rate, native identity and raw distributions.
- N30-2/R30-2: agent writes precede searches; characterize read-mostly AND committed write→search/batch-write→search schedules, including projection activity.
- N30-3/R30-3: attribute MCP/SDK, admission wait, open/idle/store-switch and query-embedding costs without treating different fixtures/backends as equivalent.
- N30-4/R30-4: distinguish writer/worker overlap and drain costs at 300k; preserve outliers and report explanation limits rather than silently discard them.
- A30-1: a second operator can rerun each selected cell from committed tools,
  identified inputs/configuration and native artifacts, obtaining the same workload.
- A30-2: every scheduled cell has samples, counts, degradation/errors, resource
  records and disposition; missing/aborted cells are explicit, never a green claim.
- A30-3: AC-083 is owner-authorized, **tracked, not gated**: 100k LongMemEval
  text, hybrid p50≤80ms / p99≤300ms targets. Report distance to target; add its
  canonical entry only in separately authorized implementation, never in prework.
- A30-4: report read-mostly and write-interleaved cache suitability separately, with current direct-node proof and source-dependency fallback costs distinguished.

## Architecture, research and design revision

- Inspect current [search](../../../../../src/rust/crates/fathomdb-engine/src/search.rs),
  [eligibility](../../../../../src/rust/crates/fathomdb-engine/src/dependency_closure.rs),
  worker scans/commits, SDK facade, MCP registry/bridge/admission and embed dispatcher.
- Bind snapshot/generation observations to the accepted [format 37 ADR](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md):
  coalesced publication, external writers, flags and time boundaries matter.
  Do not restore the prototype's obsolete global vector eligibility scan.
- Planned primary research: inspect exact pinned SQLite FTS5/rusqlite profile
  semantics and Tokio/rmcp queue/timing behavior where instrumentation relies on
  them; inspect pinned Hugging Face dataset metadata/license and actual source bytes.
- Fit review must compare each historical probe's SQL, provider assumptions,
  schema and tracing semantics with current source. Revise the measurement design
  before execution; review does not approve historical runtime hooks or whole merges.
- Challenges: shared GPU/CPU noise, overlapping projection, cold versus warm
  state, query embedding versus retrieval, time-sensitive eligibility and tracer cost.
  Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
  to measurement math and real-database boundaries; defer broad fuzz/matrix work.

## Sources, tools and identity

- Reuse the [W4 tool index](../../../../experiments/ac013-page-cache/README.md),
  `scripts/perf-experiments/run-ac013.sh`, engine `exp_ac013_page_cache.rs`,
  MCP `examples/ac013_mcp.rs` and sweep.py only after current-contract fit checks.
- Inspected original refs: scale-search `07095a16d` (README/PREREG,
  lme_corpus.py, ssweep.py, analyze.py); scale-drain `aa30085fc`
  (README/PREREG, drain_sweep.py); vector `cadcff392`, seed `8666ee72a`
  are recommendation source identities. Preserve refs; branch-only paths are not .29 links.
- Adapt selected scale/drain tools into a committed indexed experiment directory;
  record origin path/ref and changes. No experimental provider/environment hooks,
  old schema modules, F32 relayout, data migrations or capacity changes to compile them.
- Pin LongMemEval-S `xiaowu0162/longmemeval-cleaned` snapshot and raw-file hash;
  preserve converter's first-seen unique nonempty-turn order, select first 100k,
  and pin all 500 questions/order plus resulting counts/digests. Historical full
  conversion yielded 189,514 turns; verify actual inputs rather than assume it.
- Confirm LME workload fit by match-count/body-length/duplicate statistics;
  keep text outside tracked source. A synthetic control stays separately labelled.
- Record source sha/dirty state, lockfile/features/profile, executable/native binding
  paths/hashes, format, vector dimension/model/tokenizer/backend, dataset/query hashes,
  seeds, warmups/samples/repeats, hardware/toolchain and relevant env/configuration.

## Measurement sequence and decision rule

1. Freeze preregistered cells and resource limits; prebuild once outside timings.
   Use fresh database per seed repetition; do not borrow another checkout's binding.
2. Run untraced 10k control and LME 100k hybrid/text/vector attribution controls;
   retain original AC-013 semantics/defaults and manual/AGENT_LONG posture.
   Use two interleaved repeats, reversing order; preregister bounded noisy reruns.
3. Measure read-mostly and write→search schedules at 1/10/100 searches per
   committed write batch, with identical query order and separately stated drain policy.
   Record generation changes, scan/window-check time and projected cache hit potential;
   any actual cache treatment requires separately reviewed Slice 40 design/authorization.
4. Use the same BGE 384 store/backend for SDK search, embed-only and real stdio MCP
   c1 plus in-flight 4/8/16/32; record throughput, success/error counts, queue wait
   denominator and p50/p99/mean. Do not subtract percentile values as component costs.
5. Measure fresh-process/populated open, first query, steady warm calls, idle expiry
   and alternating-store switches; record open/drain/close, RSS/handles and embedding
   initialization. Preserve queue cap 4/depth 32/run 30s/deadline 10s unless a cell states otherwise.
6. Revisit TC-1807591f/TC-9a8e5f68/TC-dbf46d9e/TC-b5959972: compare held and
   overlap seed shapes at 10k/100k/300k; separate write/drain/end-to-end, pending rows,
   per-vector work, worker concurrency and WAL. Re-trace the disputed 300k outlier.
7. Pair limited traced/untraced diagnostic cells on identical inputs; report callback,
   logging and trace overhead. Traced SQL profiles explain costs; untraced samples supply latency claims. Bound debug logging's effect on MCP queue observations.
8. Decide cache suitability from observed generation reuse and total miss/window-scan
   costs for both schedules. Inconclusive/no benefit means redesign/defer with evidence;
   maintained counters need a separate storage/ADR decision, never an automatic fallback.
   Findings guide proposals; neither historical effect-size rules nor targets add release gates.

## Dependencies, resources, verification and delivery

- Hard: approved Slice 10 owned tools/native setup and Slice 20 dependable harness/race
  work; approved local design/preregistration before runs.30 evidence precedes40/50.
- Prefer settled Slice 10 dependency pins and quiet serialized measurements; later
 120/140 CUDA/fork choices are conditional rerun triggers, not blanket prerequisites.
- Resource proposal: one shared measurement lock, no compilation/concurrent campaigns,
  scratch≤10GB including DB/WAL, each run≤15min and campaign≤4h; review feasibility
  before execution and stop/report on bounds, never expand or clean others' scratch silently.
- BGE seed/query workloads use available RTX 3090 cuda:0 (or cuda:1), verify actual
  engagement; wait when busy. Exclude K620; preserve windchill3 R580 and primary CUDA 12.6.
- Later tool changes require meaningful RED→GREEN for counts, math, cell ordering,
  fresh-DB/resume behavior and real-DB smoke; no tamper/path-drift test bureaucracy.
  Independently review design and adapted code; source/script changes get required full
  agent-verify once on the final owned tree, plans get scoped Markdown validation.
- Deliver local drafts, committed indexed tools/commands/origin shas, preregistration,
  sample/config manifests, results and uncertainty report with Slice 40/50 recommendations.
  Open prerequisites: dataset access/fit, compute reservation, exact artifact/provider
  compatibility and approved run budgets; no benchmarks, environment/Git/ledger changes now.
