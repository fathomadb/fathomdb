---
title: FathomDB 0.8.29 — agent access, search at scale, and candidate selection
status: ACTIVE
target_release: 0.8.29
execution_model: direct; no Steward or Orchestrator; runs in parallel with the separate 0.8.28 effort (owner 2026-10-31)
---

# FathomDB 0.8.29 — agent access, search at scale, and candidate selection

## Goal and scope

0.8.29 finishes agent access and makes search hold up at 100,000 records.

- **Agent access.** It finishes agent-access Block 1 (the queue, hints,
  snippets and the MCP server, already merged on `dev/0.8.29`), clears its
  19 follow-up todos, and designs and builds Block 2 (agent feedback
  signals).
- **Search at scale.** It removes the per-search costs that grow with store
  size: the remaining eligibility scan and the text arm's content reads.
  It also adds a tracked AC at 100,000 records.
- **Review checkpoint.** It holds the candidate-selection review
  checkpoint from the
  [experimental review schedule](0.8.29-0.8.33-experimental-review-schedule.md).

Sources of scope:

- **Block 1.** The [Block 1 finish plan](agent-access-block1-finish-plan.md)
  covers Phase 0 (done 2026-10-28), workstreams W1–W8 and Block 2. This
  plan replaces its ladder; its W4 section keeps the AC-013 rulings.
- **AC-013.** `dev/experiments/ac013-page-cache/RECOMMENDATION.md` sets R4
  to R8. R1 to R3 and the vector-eligibility fix are 0.8.28 Slices 42 to
  90.
- **0.8.28 decisions ruled 2026-10-31.** D28-09 becomes a 0.8.29 slice.
  D28-10 and D28-01 to D28-07 go to the review checkpoint. D28-12's
  populated-open item goes to Slice 30.
- **The `dev/ROADMAP.md` § 0.8.29 entries:**
  - the candidate-selection review;
  - the two placed experiments (coalescing concurrent work, and the SDK
    hints default);
  - the compact text block;
  - CUDA context-reset survival (seq 281).

Todo numbers here are seqs on `dev/0.8.29`. Seqs fork across branches, so
the TC id is the stable reference.

## Release authority and workspace

- **Branch:** `dev/0.8.29`, cut from `dev/0.8.28` at `c02cff1f9`. Every
  slice cuts from its tip and lands back on it.
- **0.8.28 runs in parallel** as a separate effort on `dev/0.8.28`. Each
  landed 0.8.28 slice is merged forward into `dev/0.8.29`; the first such
  merge was `47683bfe2`.
- **Schema baseline:** 34 today, and 35 once 0.8.28 Slice 60 merges
  forward. A version-34 store is refused at open; there is no upgrade
  path.
- **Release state.** There is no `release-state-0.8.29.json` yet.
  `scripts/release-current.py` allows exactly one live release, and 0.8.28
  is live. Slice 0 settles how two concurrent releases are tracked. Until
  then this plan is the ladder of record.
- **Publication:** not authorized by this plan.

## Cross-release dependencies

| 0.8.29 slice | Needs from 0.8.28 | Why |
| --- | --- | --- |
| 30 | Slice 60 (store format 35) | The AC-083 baseline must be measured on the format that ships. The harness can be built first |
| 40 | Slice 70 (vector eligibility in vec0) | Slice 70 removes the vector-row scan. Slice 40 caches the scan that is left |
| 50 | Slice 60 (`search_index` rowid = `write_cursor`) | The deferred-content text arm reads content by `write_cursor` |
| 60 | Slice 80 (search view flags) | MCP exposes the flags that Slice 80 adds to the engine |
| 70 | Slice 70 | Degradation labels must describe the new fallback behaviour |
| 100 | Slice 70 | Erasure must remove the new vec0 columns |
| 80, 90 | Slice 60 (the vector-span table) | B2-8 reports coverage from the table that format 35 ships |

Slices 0 to 20 and 110 depend on nothing in 0.8.28. The Block 2 design (Slice 80) can start before 0.8.28 Slice 60 lands.

## Quality-roadmap consultation

Read [the quality testing roadmap](../notes/fathomdb-quality-testing-roadmap.md)
on 2026-10-31, as AGENTS.md requires.

- **Applies: anomaly testing at FathomDB-owned boundaries.**
  - Slice 100 (D28-09) injects failures inside an erasure transaction and
    a WAL checkpoint, then reopens the real database.
  - The queue (Block 1) is a commit boundary; Slice 20 makes its admission
    race testable (seq 300).
- **Applies: disabled-optimization tests.**
  - Slice 40 compares cached eligibility answers against the uncached scan.
  - Slice 50 compares the deferred-content text arm against
    `fuse_three_arms` on the same state.
- **Applies: resource-leak checks** for the queue and the MCP server at
  shutdown and on cancellation (Slices 20 and 60).
- **Applies: boundary-value tests.** Snippet offsets in UTF-8 bytes (Slice
  70, seq 297) and the AC-083 corpus sizes (Slice 30).
- **Applies, bounded:** one regression per confirmed defect.
- **Deferred:** bounded fuzz smoke cases for the MCP protocol surface, to
  the 0.8.31 checkpoint or later. Block 2 changes response shapes again,
  so fuzzing now would be rewritten. There is also no exhaustive scale
  matrix (0.8.33 checkpoint).
- **New evidence:**
  - The AC-013 investigation showed that a synthetic fixture can hide or
    inflate costs. The fixture's queries matched 100% of the store, against
    about 30% on real text. That is why AC-083 uses a real corpus.
  - The roadmap's Windows VFS patch line becomes stale when 0.8.28 Slice 42
    retires the vendored crate.

## Requirements and acceptance

- **Per slice:** requirements and AC are written in each slice's
  `requirements.md` and reviewed before design. Identifiers are
  release-local (R29 and AC29).
- **One governed addition:** **AC-083**, authorized by the owner on
  2026-10-31, is added to the locked `dev/acceptance.md` in Slice 30.
  - **What it measures:** hybrid search at 100,000 records on LongMemEval
    text.
  - **Status:** tracked, not gated, with a target of p50 ≤ 80 ms and p99 ≤
    300 ms.
  - **Paired change:** a successor note to
    ADR-0.7.0-text-query-latency-gates-revised.

## Cross-cutting DoD and slice process

Every slice runs these steps, in order:

1. Requirements and AC.
2. Design.
3. Design review: Fable-medium, independent.
4. TDD, with one failing RED commit per behaviour, then GREEN.
5. Code review: codex first (`/usr/lib/chatgpt/resources/codex`), with
   Fable-medium as the fallback.
6. The full `./scripts/agent-verify.sh` once on the final clean tip.
7. Owner rulings recorded in the ledger.

Prework Slices 0 to 8 are read-only. Every measurement tool is committed
and re-runnable. bge embedding uses the RTX 3090 (`cuda:0`), never the CPU
for large corpora.

## Reserved-gap policy

New slices go in steps of 10. A gap stays empty unless a placement decision
adds a slice; it is never an implicit patch lane.

## Prework: Slices 0 to 10

These follow the 0.8.26 and 0.8.27 prework, scaled down. Each of Slices 0
to 8 is one read-only pass that produces one durable register and changes
no product code, dependency, environment, CI or accepted contract. They can
run while 0.8.28 slices land.

- **Slice 0 — environment and project infrastructure.**
  - **Two releases at once.** How two concurrent live releases are tracked:
    `release-current.py`, the release-state views and the board.
  - **Hosts and resources:** GPU use (`cuda:0` only, never the K620), disk
    headroom (the AC-013 runs reached 97%), and the LongMemEval data
    location and hash.
  - **Builds:** the CUDA build of the MCP server.
- **Slice 1 — dependency and pinning sweep.**
  - Re-sweep after 0.8.28 Slice 42 adopts `rusqlite` 0.41.1: sqlite-vec
    and `rmcp`.
  - The crate versions, which still read 0.8.27.
  - Classify each as update, retain, investigate or postpone.
- **Slice 2 — repository cruft.**
  - The experiment worktrees and branches: `ac013-*`, the Block 1 branches
    and `exp/agent-access`.
  - The three stale 0.8.27/0.8.28 branches named in the finish plan.
  - `dev/experiments/ac013-*`.
- **Slice 3 — draft contracts and allocation.**
  - R29 and AC29 drafts.
  - The AC-083 text and the ADR notes (fusion identity, eligibility cache,
    the AC-083 successor).
  - The Block 2 requirements (B2-1 to B2-8).
  - The MCP view-flag contract. Allocate each to one slice.
- **Slice 4 — architecture alignment.**
  - The fusion identity key against Slice 71's design.
  - Cached against native eligibility, after 0.8.28 Slice 70.
  - The queue against ADR-0.8.27's runtime topology.
- **Slice 5 — verification adequacy.**
  - Trace each requirement to its tests.
  - Find vacuous or parallel-unsafe tests, the AC-013 fixture realism gap,
    and the MCP conformance gaps (9 pass, 24 fail at the Block 1
    baseline).
- **Slice 6 — stale documentation.**
  - `dev/interfaces/rust.md` and `mcp.md` once the view flags ship.
  - The Block 1 design docs against the built code.
  - The AC-072 tier text.
- **Slice 7 — build and delivery evidence.**
  - Recurring gate flakes: `slice40_fresh_database_cutover`, tc57,
    `slice50_evidence`.
  - Pre-push cost.
  - Release overhead: the owner noted on 2026-10-31 that release work
    carries significant overhead. Propose cuts.
- **Slice 8 — reserved.**
- **Slice 9 — prework close-out and the candidate-selection review
  checkpoint.**
  - Score and rule the Slice 0 to 8 proposals.
  - Then hold the checkpoint (see below).
  - Revise this ladder from the results. A Fable-medium review of the
    revision comes before Slice 10.
- **Slice 10 — preparation.** Carry out the repository preparation that
  Slice 9 approved: cleanup, release-state tooling and dependency updates.

### When the review checkpoint happens

The checkpoint is a planning decision, so it sits in Slice 9.

- **Why Slice 9.** Any experiment it authorizes still gets its own slice in
  the ladder, with time to preregister and run alongside the engine
  slices.
- **What it rules on:**
  - the schedule's three families (entity and alias expansion,
    complementary-evidence selection, and MMR and diversity);
  - the two placed experiments (coalescing concurrent work, and the SDK
    hints default);
  - D28-01 to D28-07;
  - D28-10;
  - two overflow-chunking measurements: the answer-level check behind
    tripwire T1, which needs no chunk vectors, and chunking's latency
    re-measured on format 35 at overflow-multiplied row counts
    (`dev/design/overflow-chunking-reevaluation.md` §§ 3, 4.4).
- **One constraint.** An authorized retrieval experiment preregisters
  against a build that contains 0.8.28 Slice 70, because restoring the
  vector arm in stores with superseded nodes changes retrieval baselines.

## Slice ladder

| Slice | Short | Contents | Folds in |
| ---: | --- | --- | --- |
| 0–10 | PREWORK | Above | — |
| 20 | TEST-HYGIENE | Parallel-safe tests (`tc57`, `slice50_evidence`, `slice40_fresh_database_cutover`); the `fathomdb-mcp` dead_code warning; the `before_enqueue` hook and queue test 23; the chunking tripwire T2 test, pinning the fusion weights, the default embedder and its 512-wordpiece cut, whose failure names the re-evaluation record | W1: 309 (tc57 half; the slice135 half is 0.8.28 Slice 42), 290, 307, 300 |
| 30 | SCALE-MEASUREMENT | AC-083 harness and entry; drain-linearity check; re-measured baselines; the Block 1 measurement todos; the 300k and outlier traces; query-embedding cost; populated-open latency | R7; W5: 298, 299, 303, 304; 355, 356, 357, 358, 360 (TC-a3ad0d49) |
| 40 | ELIGIBILITY-CACHE | Cache the remaining eligibility answer (`all_nodes_directly_eligible`) per read-visibility generation; read the vector attribute columns once per worker batch | R4; R6 (354, TC-0be48ca9) |
| 50 | TEXT-ARM | Fusion identity ADR first (distinct records with byte-identical bodies; the vector stage yields one candidate per record, so overflow chunks cannot crowd the pool); then the deferred-content text arm with its fusion variant; record-start snippets skip full-body segmentation | W7: 295; R5 (code half); W4: 292 |
| 60 | MCP-SURFACE | The compact text block with `text` as default (17 RED tests); `include_superseded` and `include_inactive` on MCP search | W6: 316/317; the MCP half of 0.8.28 Slice 80 |
| 70 | DEGRADATION-SDK | EP-03 degradation labels for an arm timeout and an absent embedder; facade re-exports; UTF-8 byte offsets documented; `SnippetOptions` binding parity | W2: 301; W3: 297, 310 |
| 80 | BLOCK2-DESIGN | One design for B2-1 to B2-8; check B2-7 against the built queue; B2-1 agrees with compact text. B2-8 reads the span table that 0.8.28 Slice 60 ships, and reports the snippet's scored extent as a byte span | B2-8 (TC-5b647921, approved 2026-10-31) |
| 90 | BLOCK2-BUILD | Build Block 2, including B2-8's `orient` coverage and snippet span | — |
| 100 | ERASURE-INTERRUPTION | D28-09: erasure interrupted inside a transaction or WAL checkpoint, with reopened-state and physical-erasure oracles | D28-09 |
| 110 | CUDA-CONTEXT-RESET | Survive a CUDA context reset, designed from the 0.8.28 Slice 30 instrumentation data | 281, 285 |
| 120 | CLOSE | CHANGELOG (the `#[non_exhaustive]` break, the `text` default, the view flags); LoCoMo agent smoke (priced, approved); release qualification | W8: 308/319, 305 |

**Why it is grouped this way:**

- **Slice 20 first.** Every later gate relies on stable tests.
- **Slice 30 before 40 and 50.** It builds the baseline that those two slices
  must beat.
- **Slices 40 and 50 are separate.** Slice 40 changes
  `dependency_closure.rs` and `projection_commit.rs`; Slice 50 changes the
  `search.rs` text route and its route names, and needs its ADR first.
- **Slice 60 can run alongside 40 and 50,** because it touches only
  `fathomdb-mcp`.
- **Slice 70 comes before Block 2,** because B2-3 ("hybrid fell back to
  text") needs truthful degradation labels.
- **Engine slices land one at a time:** 40, 50, 70, 100 and 110.
- **Overflow chunking** stays parked. Its tripwires and the costs to weigh
  are in `dev/design/overflow-chunking-reevaluation.md`.
- **Todo 306** (AC-013 headroom) closes with 0.8.28 Slice 60. **Todo 289**
  (drain) moved to 0.8.28 Slice 50.

## Open questions for the owner

- **Concurrent releases.** How two live releases are tracked. Slice 0
  proposes and Slice 9 rules.

## Immediate next slice

Nothing starts before the owner's explicit "go". Then Slices 0 to 8 and
Slice 20 can start while 0.8.28 runs.
