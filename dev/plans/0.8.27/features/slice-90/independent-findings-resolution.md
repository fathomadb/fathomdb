---
title: Slice 85 and 90 independent findings — code-grounded resolution
status: PLANNED
reviewed_candidate: ab8f43be2c9ceaa9ad19b23b23f40e9d2c484513
minor_review_candidate: ce71afe511545d963f141b5c9c051fc8638bd6b4
option_b_review_base: 9b00a980ca222d05f3af063168ed45c2b0a4a526
target_release: 0.8.27
---

# Slice 85 and 90 independent findings resolution

Reviewed exact clean candidate above on `release/0.8.27`. This record is
author review and remediation of independent findings, not independent
approval. No production source changed. Slices 85/90 remain PLANNED and
uncommissioned; the original verbatim recommendation and historical reviews
are unchanged.

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
