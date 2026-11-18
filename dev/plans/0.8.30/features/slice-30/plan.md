---
title: Slice 30 correctness bug hunt and remediation
status: PROPOSED
slice: 30
target_release: 0.8.30
---

# Slice 30 — Correctness bug hunt and remediation

Make retrieval, attribution, coverage and open failures agree with their contracts on a small set of real-store cases.
This draft refines R30-COR/S50 and owns the correctness portion of R30-TEST under the [release plan](../../plan.md).
It takes its first cases from the [findings](../../testing-robustness-findings.md), [recommendations](../../quality-performance-robustness-recommendations.md) and [receiving allocations](../../release-followups.md).

## Entry and preparation

- Reconcile the commissioned checkout with reviewed draft `f7fb1da9a81ee226c70ae347fa2fab3e64ac3c25` and current `origin/main`. Inventory commits since drafting that affect assigned functions, contracts, fixtures, pins, tooling or allocations. Keep, adjust or reject the affected plan items against the actual changes.
- Consume the local needs/requirements/acceptance, architecture and testing proposals from prework3–5 and the Slice9 decisions allocated to30. Record adopt/adjust/reject for each, update the corresponding local contract/design/test draft and identify any prerequisite owner before implementation.
- Require commissioned Slice 30 and the applicable approved Slice 10 preparation, including a checkout-owned test environment. Consume Slice 20 diagnostics when needed to establish first failure or actual execution scope. Slice 20 owns diagnostic repairs.
- Reconcile seq-393 S50 and seq-375 occurrence-label review, the [identity decision](../../../../adr/ADR-0.8.29-retrieval-record-identity.md), and the [.29 typed admission proof](../../../0.8.29/features/slice-160/status.md#typed-open-admission-fix-and-current-artifacts). Consume the shipped ambiguity refusal as an existing contract. Reset and module lifecycle work belong to Slice 50.
- Map existing boundaries from the [architecture](../../../../design/fathomdb-data-plane-architecture-v2.md) to the actual paths below. Reconcile any contract delta with the [ADR index](../../../../adr/ADR-0.6.0-decision-index.md). State the current mechanism, proposed change, owning function and prerequisite for each selected case.
- The scope is current correctness on the existing store format and routes. New caches, schema changes, precision changes and retrieval backends are outside this slice. This draft grants no paid, GPU, native-matrix, live-fault or production execution.

## Local requirements and acceptance

These are release-local refinements. The canonical acceptance suite remains authoritative.

### R30-S50 — Explain or delimit the natural-write score difference

- Need: distinguish rank movement between reader states from a same-state wrong result. First inspect the retained reports, both sample records, source pins and final databases indexed by `/mnt/data/fathomdb/0.8.29-S50-data/README.md`. Keep the archival evidence intact.
- The G-W b10/s1 SDK-hybrid diagnostic differs at sample 860, query 360, operation 9971. Eager scores are 1.240362% and 0.850694% higher for two hits, with matching ordered document IDs and text. Both calls used the reference route. The cursors were 108610 and 108600. Reconstructed ranks support a hypothesis, while the missing historical candidate lists and reader states leave asynchronous projection and tie order unresolved.
- If commissioned, use one bounded CPU reproduction on an owned real store with one query and a controlled projection step. Capture the selected route and relevant candidate record IDs, raw vector distances, original arm positions, tie groups and resolved tie order, reader snapshot witness and projection cursor. Capture both sides at the ranking read, with the same query inputs and validity window. A cursor value alone cannot identify the complete reader state.
- Pair the moving-state case with a fixed-state control exercising the supported reference and optimized routes with route witnesses. Require [S50-A3](../../../0.8.29/features/slice-50/requirements.md) exactness for ordered fields, score bits, cursor, sidecars and applicable error/fallback metadata on matched state and inputs.
- Acceptance: independently recompute affected scores from captured original ranks and identify the input or ordering difference, or report precisely what the bounded probe leaves unresolved. Discuss a state-aware natural-write comparison design and the evidence it would require. Preserve the original failed comparison, qualification credit and charges. Add no blanket tolerance, retrospective pass or full 160-member replay.

### R30-COR — Preserve stored-record and response attribution

- Need: readers can distinguish records with identical stored bytes, and feedback identifies the occurrence actually returned in the named response.
- Trace candidate ranking and late hydration in [search](../../../../../src/rust/crates/fathomdb-engine/src/search.rs), and `FusionHit`, `RecordKey` and `fuse_records` in [fusion](../../../../../src/rust/crates/fathomdb-engine/src/fusion.rs). Use canonical node/edge plus cursor, or the actual ownerless FTS locator. Start with the [record identity suite](../../../../../src/rust/crates/fathomdb-engine/tests/slice50_record_identity.rs) and [lightweight controls](../../../../../src/rust/crates/fathomdb-engine/tests/slice50_lightweight_controls.rs).
- Acceptance: equal-body records retain distinct owners, cursors, source and sidecars, including matching public refs and tagged bodies. One record seen in both arms gets one contribution per arm at its first original position. Repeats within an arm leave later original ranks intact. Assert exact rank and representative provenance from fixture rows. Preserve the selected duplicate-ID refusal in context packing.
- Trace `fold_response_refs` in [session state](../../../../../src/rust/crates/fathomdb-mcp/src/session.rs) and scoring-page retention and `feedback` in [MCP tools](../../../../../src/rust/crates/fathomdb-mcp/src/tools.rs). The map covers kept hits in one response. Check repeated refs within a page and across continuation responses against the [MCP contract](../../../../interfaces/mcp.md).
- Acceptance: in the [feedback suite](../../../../../src/rust/crates/fathomdb-mcp/tests/feedback.rs), an otherwise recordable call mixing unique and ambiguous refs across `used` and `unused` returns the typed refusal with zero label writes. A repeated occurrence of one cursor remains unique. A unique label in its own response reaches its expected query and cursor. Preserve store, membership, telemetry, text, acquisition and final epoch precedence.
- Review exact occurrence labels only. Describe candidate schema and compact rendering, binding to response/store/epoch, paging and budget lifetime, validation and the distinction from latest-eligible logical fetch. Record the owner choice or defer any ABI change. For a selected change, require exact attribution and atomic handling of stale, foreign and mixed labels in both formats.

### R30-COR — Report committed coverage, scored extents and native failures truthfully

- Need: coverage reflects committed mapped heads, snippets report extent from the actual participating record, and callers can distinguish native admission failures.
- Trace `capture` and `scored_extent` in [vector spans](../../../../../src/rust/crates/fathomdb-engine/src/vector_spans.rs), token validation and publication in [stored coverage](../../../../../src/rust/crates/fathomdb-engine/src/stored_coverage.rs), and [snippet selection](../../../../../src/rust/crates/fathomdb-engine/src/snippet_query.rs). Use the [stored coverage contract](../../../../interfaces/rust.md#stored-mapped-head-coverage-0829), [snippet design](../../../../design/agent-access/match-centred-snippets.md) and [existing engine tests](../../../../../src/rust/crates/fathomdb-engine/src/stored_coverage_tests.rs).
- Acceptance: independently enumerate attributable mapped heads. Check `R=K+U`, truncated known count, body and unrepresented bytes, exact fractions and denominator-zero nulls. Distinguish NULL from known zero and count unattributed heads separately while preserving original corruption refusals. Include retained history and an unmapped record.
- With a deterministic reader rendezvous and existing transaction controls, distinguish committed results, rollback, conservative invalidation and frozen refusal. A pinned reader returns the old exact endpoint or the contract's frozen refusal. A new ordinary read sees committed facts. A failed or rolled-back capture cannot publish staged counts.
- Assert original-body UTF-8 boundaries, including role bytes, and exact scored extent for the selected canonical vector participant. Reject a mid-codepoint or malformed tuple. Unknown endpoints and nonparticipants omit the extent, and known zero survives. Keep bounded indexed extent work and snippets-off behavior.
- Review [open admission](../../../../../src/rust/crates/fathomdb-engine/src/open.rs) against the existing [Python](../../../../../src/python/tests/test_typed_errors.py) and [Node](../../../../../src/ts/tests/typed-errors.test.ts) installed cases. Those cover malformed profile name/revision BLOBs and negative dimension. Select one discriminating malformed-native case only if reconciliation exposes a gap. A candidate is a malformed stored profile plus a bad coverage token, if existing controls leave their joint precedence unproved.
- Acceptance for a selected open case: exercise actual native open, assert original class, kind, stage, hint and anchor, relevant precedence, repeated refusal without mutation or lock retention, then restore only the injected fixture fields and reopen the persisted body in a fresh process. Use one owning engine case and thin boundary proof where affected.

## R30-TEST and stop rules

- After Slice 9 selection, Slice 30 implements the approved independent correctness oracles. Extend the existing suites with a short real-store property over known/unknown endpoints and commit/rollback/reopen, and hand-authored rank and ownership expectations for the selected same-state cases. Use fixture facts and original positions to derive expectations independently of production SQL and fusion helpers.
- The oracle must reject a plausible wrong result, such as treating NULL as zero, publishing rollback counts, or merging body twins and renumbering later ranks. Retain minimized seeds. Use the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md) for scoped property and mutation choices.
- End after the named cases and at most one discriminating minimized follow-up unless new evidence justifies extending the scope. Classify each as confirmed product defect, fixture/operator defect, expected behavior, bounded non-reproduction or unresolved observation with consequence. There is no bug quota.
- Remediate confirmed defects in this slice. Critical wrong results or data loss require repair or explicit owner disposition before release. Give Slice 70 each selected test gap, its implemented oracle or authorized disposition, and the concrete wrong result the oracle rejects.

## Design, implementation and closure

1. Draft the smallest design that meets the accepted contracts. Resolve capture placement and snapshot witness, tie and cap handling, feedback response scope, and commit-only publication. Where source evidence leaves a question, consult primary sources for the pinned SQLite WAL and FTS5 behavior and sqlite-vec tie/cap behavior, then check fit against the local source and fixture. Prepare applicable design, interface or ADR deltas and obtain direct Astra-medium design review before implementation.
2. Submit the reconciled plan to direct Astra-medium critical review, with one FIX-1 and re-review at most. Commit the updated plan after PASS. If it still fails, retain the findings and hold the affected transition. Proceed with implementation only after the slice is commissioned and its dependencies and authority are satisfied.
3. For each confirmed behavior defect, commit a real-database regression that fails for the intended assertion and include the observed failure in the RED commit. Follow with a separate GREEN commit and refactor if needed. Keep passing characterization and oracle-strength controls explicitly separate from defect REDs. Preserve human-authored expectations.
4. Obtain direct Sol6.2-high code review and direct Terra independent verification of affected runtime tests and any affected installed artifacts. Use scoped lint, type and test checks during work. Run the required full gate once on a stable source phase, then recheck failed, affected or unrun scopes after narrow fixes. Preserve full diagnostics and actual exits, skips and limits.
5. Record concise dispositions and update affected contracts, changelog, SDK parity and owning release state once. Merge any temporary branch into the owning release branch and clean only owned inactive worktrees and scratch after preserving needed evidence. Serialize shared source/state edits and isolate concurrent writers. Spawn reviewers and verifier directly.

## Prerequisites and open choices

- Entry depends on approved Slice 10 preparation and later Slice 30 commissioning. Confirm retained archive access, availability of controlled local replay and actual native inputs before relying on that proof.
- Select the bounded replay's state witness and limits, any genuinely uncovered malformed-native case, and the Slice 9 oracle improvements. Exact occurrence labels remain an explicit owner choice. This draft claims no runtime or artifact result.
