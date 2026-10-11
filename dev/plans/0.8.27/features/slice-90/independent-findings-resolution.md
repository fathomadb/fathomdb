---
title: Slice 85 and 90 independent findings — code-grounded resolution
status: PLANNED
reviewed_candidate: ab8f43be2c9ceaa9ad19b23b23f40e9d2c484513
minor_review_candidate: ce71afe511545d963f141b5c9c051fc8638bd6b4
option_b_review_base: 9b00a980ca222d05f3af063168ed45c2b0a4a526
ruled_option_b_review_base: f7e6d8abaed2744fd58369165fe2c85ce2f0dba9
corrected_ruled_option_b_review_base: 418240673f8a22027a67330c2c21cd19b773bb47
second_independent_review_base: 3b3607155ae5445bde3a771ed9e05606c2d33155
target_release: 0.8.27
---

# Slice 85 and 90 independent findings resolution

Reviewed exact clean candidate above on `release/0.8.27`. This record is
author review and remediation of independent findings, not independent
approval. No production source changed. The review-time state had Slices 85/90
PLANNED and uncommissioned; the original verbatim recommendation and historical
reviews below are retained as chronology. Current state is recorded in the
post-review disposition.

## Findings and disposition

| Finding | Verdict | Resolution |
| --- | --- | --- |
| P1-1: five-knob work is not merely forwarding | Correct and beneficial; initial design FAIL | Cite scheduler/async/embedder authority, distinguish missing topology from existing threads, and block AC27-90B/dependent runtime commissioning pending a ruling. HITL decision `seq-293` later selected Option B; the original review finding remains historical. |
| P2-1: gate can miss indirect returns | Correct and beneficial | Whole-crate edge extraction precedes scoped enforcement; executable-root reach is a fixed point, outside globs conservatively retain candidate edges, and named errors→graph-types payload admission is item/edge-kind specific. Add transitive/outside/admitted-path fixtures and executable mutants. |
| P2-2: governed glob removal has no batch allocation | Correct and beneficial | Explicit one-module import batches inside stages 3–5, 300–1,200 mechanical changed lines, compile/focused/report gates each time, zero unapproved governed globs at enforcement. Edge-admitted modules are not wholesale governed modules. |
| P2-3: remainder owner map delegates design | Correct and beneficial | Add fixed rows for importance operations, nonce, RowKind, transition/embedder helpers, cursor/projection helpers, SQLite conversion/name helpers, public status/readiness types, telemetry/lifecycle and domain constants. Entry inventory confirms, never invents, final owners. |
| P2-4: missing current consumer/default/width map | Correct and beneficial | Add five-row code-grounded table, actual versus accepted executor distinction, reader-pool and NAPI separation, retention and slow-threshold consumers, and explicit numeric-range/overflow/precedence acceptance. |

The primary agent separately reviewed the six minor findings from the same
independent pass. All are correct and beneficial:

| Finding | Resolution |
| --- | --- |
| P3-1: projected-text ownership retained an alternative | Select the already accepted direct `Result<SearchResult, SearchReaderError>` shape and delete the unused alternative. |
| P3-2: the gate had no implementation home | Fix it as a locked standalone crate under `dev/tools/module-boundary-gate`, outside the shipping workspace, with minimal named `syn`/`proc-macro2` features and an explicit normal-lint hook/cache contract. |
| P3-3: local free-function shadowing was unchecked | Reject unlisted governed-owner function shadows and add a sole-reverse-edge fixture and production mutant. |
| P3-4: the prose boundary could be mistaken for the exact policy | Make the report-only-frozen committed policy file authoritative, with prose examples explicitly illustrative. |
| P3-5: the AC-037 decision ID named the wrong predecessor | Rename it to `final-candidate-ac-037-at-slice-150`; its ruled content is unchanged. |
| P3-6: Slice 80's historical handoff wording conflicted with the later design | Add a supersession note while preserving the historical decision and implementation record. |

## Option B independent design review

A fresh read-only GPT-6 Astra medium review of the Option B scaffold at the
recorded base returned **FAIL as a commissionable design; Option B remained the
recommended direction**. The revised scaffold and Slice 90 design incorporated
the required corrections. HITL decision `seq-293` subsequently selected Option
B; a fresh independent review of the ruled contract is required before formal
successor-ADR codification or production commissioning:

- route open-time vector-equivalence probes as well as both query paths,
  projection and direct embedding through engine-owned dispatch;
- preserve degraded open, hybrid sparse fallback, direct-call errors and
  projection retry/terminal behavior rather than forcing one public timeout
  result;
- use nonblocking bounded admission and one absolute queue-plus-service
  deadline per provider invocation, including one fixed deadline per batch;
- keep timed-out calls in their slots without replacement threads, then reopen
  capacity when they return instead of permanently latching the default
  one-slot engine;
- use an independent absolute close budget, join every SQLite owner, and report
  incomplete embed shutdown without claiming arbitrary provider teardown;
- state exact active-plus-queued projection row units and separate durable,
  admitted, queued, active and timed-out-live observations;
- preserve Python's either/or input and TypeScript's object input, avoid
  hypothetical custom binding embedders, and integrate the configured open with
  `EmbedderChoice`;
- enumerate clause-level successor obligations across scheduler, writer,
  embedder, projection-model and async-binding authorities; and
- make the numeric ceilings, default-one embed concurrency, typed configuration
  delta and public configured-open seam explicit approval items.

The review also found a likely existing self-deadlock:
`embed_projection_batch` calls `per_job()` from breaker/error branches while
holding `embed_serialize`, and `run_projection_job` reacquires that mutex. Slice
90 now requires a bounded RED reproduction, a GREEN guard/permit-drop fix, and
a failing restoration mutant before the executor work can hide or replace the
old path. Configuration documentation is likewise an explicit Slice 90 batch
and acceptance obligation, not deferred to Slice 140.

## Ruled Option B review and correction

The same independent GPT-6 Astra medium reviewer examined the clean ruled
candidate above and returned **FAIL as a commissionable successor contract**
with two P1 and three P2 findings. It affirmed Option B as the correct ruled
direction and did not recommend reopening `seq-293`. The author applied these
required contract corrections before the next independent pass:

- narrow the 30-second guarantee to embed-runtime drain after safe database
  quiescence; preserve the existing potentially longer wait for active SQLite
  owners and add real-database full-queue/active-operation tests rather than
  claiming bounded total `Engine::close`;
- separate projection admission full, queued expiration, started-provider
  timeout, provider error and close cancellation so transient capacity cannot
  consume provider-failure retries or terminalize durable work;
- bound only additional frozen-snapshot retention caused by inference waiting,
  then run sparse fallback on the same authoritative transaction through normal
  query completion;
- attribute only the Option B architectural direction to `seq-293`; numeric
  limits, queue multiplier, default, APIs and exact errors remain proposals for
  formal successor-ADR acceptance; and
- reject TypeScript zero only for the three strictly positive runtime controls,
  while accepting and behavior-testing zero provenance retention and slow
  thresholds.

## Evidence

At the reviewed SHA, engine `Cargo.toml` has no Tokio dependency.
`projection_runtime.rs:404–427` creates an OS dispatcher and two workers;
`lib.rs:576–598` fixes worker and queue limits. The shared runtime documents
`embed_serialize` at `projection_runtime.rs:49–90`; workers hold it around
watchdog calls (`projection_worker.rs:549–569,742–779`).
`embedding.rs:26–97` spawns per-call detached threads with receiver deadlines;
`embedding.rs:119–123` and `search.rs:1205` directly invoke `embed` without
that watchdog. `lib.rs:5942` is another test-only direct path. These are not
the two pools specified by accepted
[`scheduler shape`](../../../../adr/ADR-0.6.0-scheduler-shape.md).

Accepted [`async surface`](../../../../adr/ADR-0.6.0-async-surface.md)
and [`embedder protocol`](../../../../adr/ADR-0.6.0-embedder-protocol.md)
require engine-owned embed dispatch, no reentrancy and deadline behavior.
The scheduler additionally names a dedicated writer/channel topology;
current `Engine::write_node_importance` (`lib.rs:6141–6165`) is one concrete
mutex-connection transaction, not that dedicated writer. The current
[`engine design`](../../../../design/engine.md) also describes the actual
mutex writer model. A contradictory implementation/current design is not an
accepted supersession of the scheduler ADR.

NAPI `call_engine` (`lib.rs:602`) uses `tokio::task::spawn_blocking`;
its five config fields (`lib.rs:2557–2561`) are optional u32s. Python's
`config.py:21–25` uses optional ints; TS `index.ts:168–172` uses numbers.
The Rust provenance cap is u64 and reaches
`write_commit::enforce_provenance_retention` (`write_commit.rs:939`)
through write/actuation. Retention counts sweepable operational mutations,
exempts erasure accountability rows and has hysteresis; tests must preserve
these semantics, not demand an exact physical-table cap.
`lib.rs:3910,5253–5254,8961` shows slow-threshold default initialization,
working setter and profile consumer; forwarding at open is still absent.

`errors.rs:198` embeds `GraphExpansionErrorV1`; its defining owner is
`graph_expand/types.rs`, with its constructor/Display/Error impls currently
in `execution.rs:88–123`. Root re-exports must resolve to the defining type,
not invent an execution edge to the facade. Conversely conversions and
formatting bodies are executable dependencies, not silently admitted data.
`errors.rs` and governed graph files currently use `super::*`; hiding their
outgoing edges would invalidate the promised boundary proof.

Root evidence for the named residuals includes `lib.rs:629–636` nonce,
`2804–2835` RowKind/transition legality, `3050–3330` projection/readiness
contracts, `4919–4951` lifecycle events, `6141–6190` importance operations,
`7651–7690` identity/status, and `8532–8706` cursor/SQLite/event helpers.
Root hex encoding also serves actuation, closure, projection generation and
write hashing, so it belongs with `identity`, not operator merely because
export uses it. No generic utility bucket or aesthetic relocation is added.

## Decision and ladder

**Question:** implement accepted scheduler topology literally (A), or accept
a narrow successor preserving synchronous projection/commit ownership while
defining real independent orchestration/embedder capacity and universal
deadlines (B)? **Recommendation: B**, because A changes writer topology and
transaction scheduling beyond a forwarding correction. B must explicitly
amend the authoritative contracts, preserve isolation/no-reentrancy, define
pool/queue/timeout/shutdown behavior, and then be implemented and tested;
current no-op knobs are not an acceptable successor. HITL decision `seq-293`
rules B; independent design approval and formal successor-ADR codification are
still required before implementation.

No Slice 91 is added. Even A can have runtime qualification as a mandatory
in-slice checkpoint before mechanical moves; no separate technical delivery
dependency has been demonstrated. A concrete dependency discovered in the
approved topology would require a reviewed ladder change, not silent debt.
Slice 90 cannot close or unblock 100 until AC27-90B passes. The design and
ruled release-state decision make that dependency explicit.

## Verification scope and verdict

The full Markdown wrapper, separate plan-status lint, generated-view check,
plan-anchor check, JSON parse and `git diff --check` passed. The stale/optional
wording scan found no permission to ignore a knob or silently defer closure;
the original verbatim recommendation has no diff. No engine/runtime correctness
or independent design approval is claimed by those checks. The P2 and P3
findings are remediated in prospective design; P1's evidence and scope are
corrected; `seq-293` supplies the HITL direction, while independent design
approval and formal successor-ADR codification remain pending.

## Corrected ruled-plan review — 41824067

This is a new review/remediation record, not a rewrite of the historical
verdicts above. The delegated reviewer verified clean
`418240673f8a22027a67330c2c21cd19b773bb47` on `release/0.8.27`, then became the
sole planning-file writer under an explicit review-and-remediate instruction.
No Steward/Orchestrator role, production change, release-state mutation or
commissioning was used. The five findings from `f7e6d8ab` were corrected in
this input. The remaining initial verdict was **FAIL**, with no P0 findings,
two P1, four P2 and one P3 below. Evidence line numbers in this table are bound
to the clean input SHA, not to later edited documents. Engine source paths are
relative to `src/rust/crates/fathomdb-engine/`.

| ID | Initial finding and evidence | Planning remediation |
| --- | --- | --- |
| P1-R1 | Cancellation was deferred until after database-owner joins: scaffold lines 293–311, versus `src/projection_runtime.rs:849–885` and `src/reader_pool.rs:669–680`. A worker waiting on a long configured inference timeout or retry could therefore hold phase one hostage before the advertised drain even started. | Close embed admission and cancel queued/running-result waiters and retry/capacity waits before any join. Keep database quiescence unbounded by the embed budget; start one shared 30-second provider-drain budget afterward. Clarify worker-local callback/connection cleanup before exit and shared-context/sidecar release after joins. Add a timeout-greater-than-drain-budget test and preserve the active-SQL/full-reader-queue tests. |
| P1-R2 | Scaffold lines 279–280 and 388–393 promised all executor cleanup while also accepting a degraded open after a hung probe. The probe precedes fallible later startup (`src/lib.rs:3886–3936`); arbitrary providers cannot be forcibly reclaimed, and successful degraded open still needs its live engine. | Separate engine publication from provider termination. Successful degraded open keeps its executor/occupied slot and writes no accepted baseline; later startup failure cancels dispatch, unwinds every database resource and preserves the original open error, with only bounded provider-only retention. Require distinct degraded-success and post-probe-failure tests. |
| P2-R3 | The failure-stage table omitted panic and invalid-output outcomes; design lines 283–288 allowed only provider errors/timeouts to spend retries. Current dimension rejection retries with `EmbedderDimensionMismatchError` (`src/projection_worker.rs:802–804`), while watchdog panic transport and the projection panic boundary are explicit (`src/embedding.rs:40–50`, `src/projection_worker.rs:411–428`, `src/vector_equivalence.rs:37–42`). | Preserve existing result validation/retry codes and operation-specific panic boundaries. Timely panics are transported, not converted to sparse fallback or retryable errors; late panics lose to cancellation/timeout, release accounting and leave fixed workers reusable. Specify close-cancellation versus already-winning completion without changing earlier refusal precedence. |
| P2-R4 | Supersession remained conditional for projection-model (scaffold lines 54–60), despite conflicts with its Granularity/Backpressure/Restart clauses; async-surface still prescribed internal Arc/async and binding-pool sizing, while engine.md called total close bounded. The runtime constant map still assigned a session-latch threshold after retiring that latch. | Make the exact successor clause map mandatory, including scheduler observation/adapter-shed policy, async mechanism, deadline scope and engine close wording. Retain current readiness reporting and private testable executor accounting without changing the locked public `CounterSnapshot` keys (`src/lib.rs:2845–2853`). Keep the freshness SLI, generation/mean/commit authority and Slice 85 boundaries. Name the concrete private dispatch owner, retire obsolete constants, and include scheduler design in configuration documentation. Scope universal dispatch to `Embedder` calls, not independent reranking or SDK utilities. |
| P2-R5 | Scaffold lines 312–317 introduced a detached per-session reaper without specifying whether it added a thread, exact close error or repeated-close behavior. That could add needless retained ownership and renew a nominally absolute budget. | Remove the reaper: detach only unfinished fixed workers, whose provider/request/accounting state remains counted until exit. Propose existing `EngineError::Scheduler` for incomplete drain; serialize teardown, never restart its budget, and distinguish later still-incomplete versus completed close. State the per-session—not process-wide—retention residual and exact engine-owned thread/connection/queue accounting. |
| P2-R6 | Scaffold lines 415–418 required historical PR-9 tests to pass without naming the incompatible terminal-failure oracle. `tests/pr9_embed_watchdog.rs:191–226` expects successful drain plus terminal failure for a hung provider; fixed default-one capacity instead leaves later attempts admission-unavailable. Capacity deferrals also lacked an explicit rule preserving already-spent retry attempts. | Record the successor oracle delta: pending durable work and bounded `Scheduler` drain failure until capacity returns, with existing no-late-commit/write-liveness/thread-bound/recovery assertions retained. Capacity waits neither reset nor increment the retry count; retain admitted-generation state inside the row bound rather than an unbounded side map. |
| P3-R7 | The RED description conflated breaker-open and actual fallback reacquisition (scaffold lines 419–426; design lines 309–317). In `src/projection_worker.rs:560–575`, breaker-open sets the flag and the per-job route can fast-fail, while the returned-error/timeout arm really attempts the held guard. The scaffold also called post-commit dispatch a SQLite requirement at lines 102–104. | Target the error/timeout arm for the bounded RED and restoration mutant; separately test breaker fast failure. Require a subprocess/cancellation-safe mutant harness so Drop cannot hang the suite. Attribute post-commit dispatch to the product contract rather than SQLite. |

All corrections above are prospective contract/TDD changes in the scaffold,
Slice 90 design and master plan; no production oracle was edited or executed.
Numeric limits, default-one concurrency, the configured-open seam, private
observation policy and exact errors remain proposals for formal successor-ADR
acceptance, not additional decisions attributed to seq-293. The ledger entry
was read via `ledgerwatch --dry-run --select seq=293`; release state continues
to record only the ruled Option B direction.

The initial focused verification after remediation passed
`scripts/agent-lint-md.sh` (including planning/design/findings/anchor gates),
the generated release-state view check and `git diff --check`. The final
candidate-bound re-review and record-only verification follow below after the
remediation commit is fixed. No JSON changed; no runtime, installed-binding,
full-workspace, freshness-performance or deadlock-mutant pass is claimed.

### Final candidate-bound re-review

**Reviewed candidate:** `920cfdc9f6da8200eeaa8fad176d6b8e29aadb7c`, clean on
`release/0.8.27`. **Verdict: PASS for the prospective Option B design.** All
seven findings above are resolved; no P0/P1/P2/P3 finding remains in this
contract review. This is the commissioned independent reviewer's re-review
after its own authorized planning remediation, not a claim that a second
reviewer independently examined those edits.

Re-review checked the complete corrected contract against scheduler-shape,
async-surface, embedder-protocol, single-writer, projection-model and freshness
authority; engine open/probe/watchdog/projection/reader/close code; NAPI's actual
`spawn_blocking` and ignored configuration input; Python's either/or open
shape; TypeScript's object input; and the existing PR-9 failure assertions.
It confirms that the successor preserves SQLite connection ownership,
primary-writer/projection-worker/commit-gate authority, generation checks,
mean recomputation and frozen-snapshot authority without adding a generic
executor, writer thread, reaper, binding custom-provider bridge, or Slice 85
ownership redesign. The configuration/TDD/documentation obligations are
falsifiable and remain mandatory implementation work, not completed evidence.

The plan is ready to be folded into and formally accepted as a successor ADR.
That acceptance must explicitly approve the proposed numeric/default/API/
error/observation changes and the exact supersession map; this PASS is not ADR
acceptance. Slice 90 is not commissioned. After Slice 85 closes, formal
successor acceptance, the required exact-entry inventory/receipts and explicit
commissioning remain prerequisites. No independent technical delivery boundary
justifies Slice 91; runtime qualification stays ahead of mechanical extraction
inside Slice 90.

Checks at the reviewed commit: `scripts/agent-lint-md.sh` **PASS**;
`scripts/check-release-state-views.sh --check` **PASS** (12 generated blocks,
seven state files); `git diff --check 41824067 HEAD` **PASS**. Git confirmed no
change to `src/`, accepted ADRs, release state or the Steward ledger, and a
clean worktree. JSON parsing was not separately required because no JSON was
edited; the view gate parsed the state files. The normal commit hooks ran,
including secret scanning and AST-guarded Markdown fixing; none was skipped.

This final receipt is the only post-review edit. Its recording commit does
not change the reviewed scaffold/design/master-plan contents and is checked
again with the same focused planning/view/whitespace gates. Runtime tests,
installed native artifacts, the existing deadlock reproduction/mutant,
performance/freshness gates and full-workspace verification were not run in
this planning-only task. Their absence is not implementation conformance and
does not close AC27-90B, authorize production changes, tags, push or publication.

## Second independent review — 3b360715

A second reviewer, independent of the author of the Option B revisions above,
reviewed clean `3b3607155ae5445bde3a771ed9e05606c2d33155` on
`release/0.8.27` and returned **PASS-WITH-FIXES** for the revised Slice 90
design and the Option B scaffold: no P0/P1, two P2, three P3. It confirmed
against source that the five-knob substrate table, the existing
`EngineError::{Scheduler, Overloaded, Closing, Embedder}` variants, the
`Embedder: Send + Sync` bound, the PR-9 oracle delta, and the R27-90J
deadlock (real on the `Err(_) => return per_job()` arm, fast-failing on the
breaker arm, reachable only with `FATHOMDB_PROJECTION_BATCH`) are accurately
described. The same reviewer then remediated its own findings under an explicit
instruction; this section is that remediation record, not a third review.

| ID | Finding | Remediation |
| --- | --- | --- |
| P2-1 | Successor-ADR acceptance (ceilings, default-one concurrency, `4×N` queue, `EngineConfig`/configured-open delta, typed error, Number-safe caps, `Scheduler` drain outcome) was an open HITL decision not registered in `decisions.unruled`; the board's generated count read ONE. | Added unruled `D27-successor-adr-acceptance` to `release-state-0.8.27.json`; regenerated the board count (TWO); listed the decision on the board, in the master plan's Slice 90 section, in the design's ruled-direction section and in the scaffold's remaining-approvals section. |
| P2-2 | Stage 2 bundled executor construction, four call-path migrations, the two-phase close and lifecycle into one sub-batch with no release-state-bound verification boundary before stage 3. | Enumerated stage 2 as ordered sub-batches 2a–2k in the design. The original prose-only checkpoint fields were later superseded by the structured `runtime_checkpoint` object and `scripts/check-runtime-checkpoints.py`, which bind candidate/binding SHAs, hashed PASS receipts and the first stage-3 commit. AC27-90I requires that checkpoint before any stage-3 move. This is the candidate-bound boundary that makes Slice 91 unnecessary. |
| P3-1 | The reliability-posture change carried by default-one embed concurrency (one hung call stalls all dense work in the session) was accepted in the scaffold but not stated plainly for the acceptance package. | Stated it explicitly in the design's ruled-direction section, the scaffold's remaining-approvals section and the decision title. |
| P3-2 | R27-90J said "likely" and did not name the reachability condition or the exact arm. | Design, scaffold, master plan and AC27-90J now name the `FATHOMDB_PROJECTION_BATCH` opt-in, the line-575 arm at `ab8f43be`, and state the defect is confirmed by reading. |
| P3-3 | Design cited `459d528f` as the reviewed baseline while its substrate table cited `ab8f43be`. | Design now cites `ab8f43be` with the note that engine source is identical between the two. |

The preceding paragraph records the review-time state: no production source or
accepted ADR changed, Slice 91 remained unnecessary, Slices 85/90 were PLANNED,
and `D27-successor-adr-acceptance` was still unruled.

## Post-review HITL disposition

The paragraph above is the review-time state. The repository owner subsequently
approved `D27-successor-adr-acceptance` at `seq-295` on 2026-09-28. The accepted
authority is
[`ADR-0.8.27-engine-owned-runtime-topology`](../../../../adr/ADR-0.8.27-engine-owned-runtime-topology.md).
This closes the D27 successor-design block but does not commission Slice 90.
Slice 85 subsequently completed at `7a2f9bf9` with closeout `8cd3389d`; only an
explicit Slice 90 execution ruling plus its entry/checkpoint work remain.
