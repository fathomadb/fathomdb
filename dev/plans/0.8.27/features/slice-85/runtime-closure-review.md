---
title: Slice 85 and Slice 90 prospective runtime-closure review
status: PLANNED
target_release: 0.8.27
reviewed_candidate: 459d528f2af008739dfb81656403d0a55e23072b
---

# Slice 85 and Slice 90 prospective runtime-closure review

The review began at clean `459d528f2af008739dfb81656403d0a55e23072b`.
Initial verdict: **FAIL — planning gaps permit incomplete runtime closure**.
This record documents the findings and their prospective remediation. The
author both reviewed and edited these documents; this is not an independent
approval of the edited candidate and does not commission either slice.

## Evidence

- The master plan's Slice 85 scoped ownership/error/gate design and Slice 90
  handoffs; release-state marks both PLANNED and Slice 100 depends on 90.
- Slice 40 design's temporary wildcard/root navigation; Slice 50's deliberate
  dependency/closure collaboration; Slice 60/70's verbatim-move reviews and
  Slice 70's open/configuration/projector carryovers.
- Slice 80 design/status and actual read/search/graph/pool source: narrow error
  separation is required because `filter::validate_filter_attributes_on_snapshot`
  and frozen/graph handlers currently consume `SearchReaderError`; moving that
  enum unchanged would retain transitive error/type cycles.
- Engine `lib.rs`: open/connection/configuration/WAL/operator functions,
  runtime report types, index projectors, root methods and cfg test seams.
  `projection_runtime.rs` retains the four shared search controls, hard-coded
  worker construction and the default timeout; `projection_worker.rs` reads
  that timeout for embedding work.
- Python `Engine.open` stores resolved configuration without forwarding it;
  TypeScript forwards options but NAPI open ignores `engine_config`; PyO3 open
  accepts only path and embedder choice. The five-knob declarations, accepted
  embedder ADR and bindings/interfaces distinguish engine-owned runtime knobs
  from host-runtime handoff machinery.
- Existing feature-complete, surface, Windows WAL, slice35, C1/removal and
  graph-wire source gates are affected by the planned moves.

## Findings and resolutions

| ID | Severity | Finding | Planning resolution |
| --- | --- | --- | --- |
| RC-1 | P1 | Slice 90 has no complete requirements/exit inventory; "decides whether", "may relocate" and "stay at root" allow named work to remain undecided before Slice 100. | Dedicated Slice 90 design provides final owner/disposition rows, exact entry/final item inventory, R27-90A–I/AC27-90A–I, and zero-open-item exit. All runtime handoffs finish within 90. |
| RC-2 | P1 | Proposing a successor ADR could be counted as closing the unimplemented pool/timeout contract. SDK forwarding is absent, and worker count alone is not evidence of an engine-owned embedding executor. | Require accepted contract implementation or an accepted and implemented successor within 90. Trace all five advertised knobs to observable effects; separate genuine RED/GREEN configuration batches and installed Python/Node proof from structural moves. |
| RC-3 | P1 | Exact public-surface preservation conflicts with potentially necessary Rust/native configuration plumbing. | Review the named API/interface delta before implementation; retain the immutable Slice 30 baseline and explicitly account only for approved configuration differences. Capture a separate post-correction comparison point for mechanical moves. |
| RC-4 | P1 | Scoped graph exclusions could hide a leave-and-return dependency path, or type admission could erase a prohibited cycle. | Type admissions cannot waive executable/capability edges or any forbidden-cycle edge. Report and resolve returning external paths by focused item analysis before closeout; no automatic whole-module scope expansion. |
| RC-5 | P2 | Field-map prose maps only "relevant" fields while AC requires exact inventory; cfg-duplicate wording rejects valid mutually exclusive definitions and does not define cfg evaluation. | Require one owner/exemption per field, reject duplicate active definitions per recorded compiler cfg/feature input, and fail on unsupported source/configuration forms. Preserve distinct target and test policies. |
| RC-6 | P2 | Gate batch ordering moves the attributed transaction before its WAL owner, and SCC allowlisting lacks an explicit edge-level bound. | Move WAL attribution first; allow only reviewed directed inherited edges, never an entire SCC or a new path through its members. Add independent review/verification and warm lint-cost evidence. |
| RC-7 | P2 | Slice 90 lacks bounded execution/review stages and could absorb binding decomposition or normalize unrelated modules. | Six ordered stages with configuration sub-batches, 300–600 non-mechanical line targets, split thresholds and per-batch checks. Only forwarding touches bindings; decomposition remains 100–130 and broad navigation remains 140. |
| RC-8 | P2 | Non-Linux, source-scraper and runtime lifecycle evidence can be postponed beyond the promised root closure. | Make exact feature/platform, scraper, fresh-process lifecycle/fault and independent verification receipts Slice 90 exit requirements, while keeping final AC-037 qualification at 150. |
| RC-9 | P3 | Living status says the Slice 85 directory contains only a recommendation. | Update prospective status/feature references; preserve the verbatim recommendation and settled historical review records. |

No production edits are part of these resolutions. Narrow filter/graph errors
and explicit read/search/graph ownership remain Slice 85 work. No new general
error framework, whole-crate cycle cleanup, or line-count-driven ownership is
introduced. The retained reader-loop arms and shared search fields are final
item-specific dispositions, not escape hatches for incomplete work.

## Ladder and verdict

**Slice 91 is unnecessary.** Configuration correction, mechanical ownership
moves, and integrated verification are reviewable sequential batches of one
runtime-closure objective. There is no independent technical prerequisite
requiring a new slice. Slice 100 continues to depend on completed Slice 90.
Future evidence that makes that impossible must block closure and trigger a
reviewed ladder change; convenience does not authorize partial completion.

Remediation verdict: **the enumerated planning findings are addressed by the
edited design; independent review of the resulting candidate is still required**.
The exact implementation owner inventory is deliberately produced at Slice
85 exit, so it reflects the files that will actually be moved. Its approval is
a mandatory pre-move gate, not authority to silently change the named owners.
Neither design is an implementation or runtime verification claim.

## Validation

The full documentation lint wrapper passed, including Markdown structure,
plan/design status, plan anchors and release-state generated views. JSON parse
and `git diff --check` passed. A targeted optional-wording scan found and
corrected the living Slice 80 field-relocation handoff; historical extraction
descriptions remain intact. The earlier `gpt-6-astra-recommendation.txt`
remains unchanged. These are documentation checks, not runtime verification.
