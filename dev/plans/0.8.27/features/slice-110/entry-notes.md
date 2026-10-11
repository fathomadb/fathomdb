---
title: FathomDB 0.8.27 Slice 110 - entry notes from Slice 100
status: HANDOFF
target_release: 0.8.27
---

# Slice 110 entry notes

These notes supplement the [Slice 110 design](design.md). They record what made
Slice 100 slower to close and the facts the NAPI implementer should have at
entry. They do not commission Slice 110, approve a TypeScript subscriber
successor, or replace its required entry inventory and review.

## Lessons from the Slice 100 session

1. **Reconcile the draft against the actual entry commit before approving it.**
   Slice 100's useful starting point was the closed Slice 90 release commit,
   not the older prospective design baseline. List source items, generated
   registrations, loaded runtime identities, feature gates, packaging paths,
   assigned work and already landed corrections before choosing move batches.
   Begin 110 from the current `release/0.8.27` head and verify the worktree's
   actual branch and commit from Git. The Slice 100 closeout is
   `c2014502f4dad1f65eba07cf2a4207907c4997e1`.
2. **Settle behavior and authority before extraction.** Slice 100's inert
   Python subscriber was a separate contract defect, not a mechanical move.
   The SQLite callback, caller logger, heartbeat and operation-ID questions
   needed an architecture review, RED tests, an accepted ADR and active design
   updates. In 110, decide the NAPI subscriber's exact contract before moving
   its owner. Do not let a module move conceal a callback change.
3. **Read supersession clauses, not only old ADR decision bullets.** The draft
   110 design treated Tokio `spawn_blocking` as an unresolved mismatch with
   the original async ADR. The accepted 0.8.27 runtime ADR expressly replaces
   that ADR's exact ThreadsafeFunction and binding-pool mechanisms. The
   TypeScript interface and active binding design now name `spawn_blocking`.
   Its off-event-loop and failure outcomes still require tests; a custom pool
   is not required merely by superseded wording.
4. **Include active architecture and design convergence in the initial exit
   list.** Slice 100 required a final follow-up to document the implemented
   callback path and rationale in the active data-plane, binding, lifecycle and
   engine designs. For 110, update affected active designs, TypeScript
   interface and public API documentation with any accepted subscriber change
   before declaring the slice closed. Review that final wording against code.
5. **Plan candidate-bound verification around clean commits.** Slice 100's
   first full gate reached the two Python suites that correctly refused its
   dirty checkout. Its clean candidate then passed the strict full gate.
   Keep RED tests visible before fixes, commit a clean production candidate
   before exact-candidate receipts and independent final review, and reserve
   focused documentation/state checks for later documentation-only edits.
   Do not describe environment skips as tested routes.
6. **Treat artifact provenance and source scanners as part of the move.** Slice
   100 found stale citations and a Windows source guard after code review;
   mutation checks proved each retarget. Inventory NAPI source readers before
   moving any owner and make a hidden-owner mutation fail. Generated
   declarations, loaded runtime exports and installed packages are separate
   oracles; none can stand in for another.

## Slice 110 purpose and immediate decisions

- Decompose `fathomdb-napi/src/lib.rs` into private semantic owners while
  preserving the one native Engine identity, JS names, sync/Promise shapes,
  conversions, error envelopes, feature gates and release package layout.
  Complete native work before Slice 120's TypeScript SDK decomposition; no
  Slice 111 is allocated. Use the owner map and R27-110A–F in the design.
- Record the executor disposition from
  [`ADR-0.8.27-engine-owned-runtime-topology`](../../../../adr/ADR-0.8.27-engine-owned-runtime-topology.md)
  and the active [TypeScript interface](../../../../interfaces/typescript.md):
  Tokio `spawn_blocking` is the current permitted NAPI handoff, separate from
  the five engine-owned runtime knobs. Prove responsiveness with a controlled
  native rendezvous and independent event-loop work; prove panic, join,
  in-flight close and ownership outcomes. Avoid a new executor mechanism
  without a failing contract witness.
- `Engine.attach_subscriber` in the NAPI crate currently discards its callback
  and options. The TypeScript wrapper exposes
  `attachSubscriber(callback, { heartbeatIntervalMs? })`, while existing tests
  only establish that the method can be called. Add a real delivery RED test
  before implementation. The accepted
  [Python subscriber ADR](../../../../adr/ADR-0.8.27-python-subscriber-delivery.md)
  explicitly does not alter TypeScript's contract. Determine the TypeScript
  heartbeat disposition separately; the current engine has no operation-scoped
  heartbeat producer, so a timer must not be reported as database progress.
  Any TypeScript public signature change needs reviewed interface, design,
  public-doc and accepted decision coverage.
- Design the callback path for FathomDB's SQLite data plane. A profiled
  `rusqlite` statement can enter the engine registry through SQLite's C
  callback while a writer, projector or reader owns database resources. Never
  invoke JS on that thread or hold it for a slow callback. Decide and test
  bounded enqueue/delivery, JS-thread affinity, overload reporting, callback
  failure, same-call-path reentry, replacement, close and process exit.
  Separate exact engine events from best-effort host delivery. Keep actuation
  `operation_id` (a durable idempotency key) distinct from diagnostic
  correlation; add no synthetic public operation ID.
- Preserve the shipping path. Build production NAPI with the checked
  `src/ts/scripts/build-native.mjs` wrapper and pinned Node 25.9.0/npm
  11.12.1. After a test-hooks debug build, prove hooks absent from both the
  clean production declaration output and the loaded runtime. Stage and pack
  the thin `fathomdb` package plus its matching platform `.node` package;
  install both tarballs in a fresh external consumer and verify binary hashes
  and loader resolution. The main package alone, repository imports, a direct
  `.node` load or test-build `dist/src` declarations are insufficient.
- Freeze the required feature/platform rows from CI and release contracts at
  entry. Use focused per-owner tests during moves and the full workspace gate
  on the clean candidate. An unavailable required row remains open. Record
  exact candidate, target, features, toolchain, artifact hashes, commands and
  exits for the independent code review and read-only verification.

## Starting references

- [Slice 100 completion](../slice-100/status.md),
  [inventory reconciliation](../slice-100/reconciliation.md) and
  [verification](../slice-100/review-verification.md) show the useful evidence
  pattern and its limits. Reuse the method, not Python-specific signatures or
  worker policy.
- [Binding design](../../../../design/bindings.md) § 2 and § 8,
  [lifecycle design](../../../../design/lifecycle.md),
  [Slice 30 comparator design](../slice-30/design.md),
  [TypeScript interface](../../../../interfaces/typescript.md),
  [async ADR](../../../../adr/ADR-0.6.0-async-surface.md) and its
  [accepted successor](../../../../adr/ADR-0.8.27-engine-owned-runtime-topology.md)
  are the contract map. Read the supersession text before editing.
- The repository memory entries `agent-worktree-stale-base-trap.md`,
  `shared-checkout-branch-can-be-stale-vs-session-env.md` and
  `release-dod-requires-full-workspace-gate.md` explain the worktree, shared
  environment and verification traps. They are in the out-of-repo memory
  store indexed by `MEMORY.md`.
