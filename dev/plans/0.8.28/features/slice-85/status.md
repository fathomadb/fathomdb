---
title: Slice 85 — implementation and qualification status
status: COMPLETE
date: 2026-11-04
---

# Slice 85 status

**COMPLETE_ON_RELEASE_BRANCH.** Qualified source is
`a2fb4a6475500b70676f9f54aff251df60f3e07c`, integrated on `dev/0.8.28` and
`release/0.8.28` and pushed to the canonical release branch. Astra approved
requirements/design and final code; an independent verifier passed S85-1–8.
No tag, publication, main-branch landing or Slice 90 implementation is claimed.

The [approved contract](../../../../design/explicit-maintenance-erasure/contract.md),
[accepted successor ADR](../../../../adr/ADR-0.8.28-explicit-erasure-maintenance.md)
and [production design](../../../../design/explicit-maintenance-erasure/production-design.md)
own format 36 and the implemented API. Queued intent retains existing visibility;
new ordinary admission returns NeedsMaintenanceAdmission when bounded proof is
unavailable, and the handle provides heavy admission. Post-delete debt preserves
global mutation fences through handle release/reopen until actual completion.

## Acceptance results

Tests use real SQLite files, existing physical/independent-survivor oracles and
observed behavior-specific RED/GREEN evidence. No historical golden baseline was
regenerated. All Astra findings were resolved and independently reviewed.

| Acceptance | Result and decisive evidence |
| --- | --- |
| S85-1 | PASS: persisted policy validation, mismatch before mutation, fresh format 36 and no explicit-mode worker. Actual frozen format 35 executable refused an actual final-candidate format 36 store; final native code refused an actual old-binary-created format 35 store. Main/WAL/SHM checksums unchanged in both directions. |
| S85-2 | PASS: exact key/conflict/retirement, withdrawal/reopen,16 queued and 4,096 reserved receipt capacity, full dependency closure,16,385-target refusal, generated histories, same-batch old/new endpoints/provenance/dependency guards and resource growth preserving Queued without deletion. Legitimate late-original 8 MiB boundary RED→GREEN preserves reopen; UTF8 bytes and future status growth are reserved. |
| S85-3 | PASS: outer operation leases, counted projection scanner publication, thaw/owner ordering, fast newcomers, callback/provider self-drain and close watchdogs, stale handles, caller loss and existing close cancellation. |
| S85-4 | PASS: all four FTS families, mapped non-head vectors and dependent rows populated before removal; exact zero, queue identity/main/WAL absence and independent survivor-only BM25 score/order. Atomic ticket/root/delete, identity-free redaction and actual final receipt commits. |
| S85-5 | PASS: actual process exits at eight queue/delete/redaction/receipt boundaries, rollback and failure/reopen fences, external WAL reader, corrupted policy/root/ticket/phase authority and durable sanitation status. |
| S85-6 | PASS: later second original, wrong/substituted/unavailable sink refusal, retained multiple originals and authenticated redaction. Offline completion publishes the same activated receipt and leaves queued intent untouched; corruption refuses before mutation. |
| S85-7 | PASS: Rust facade/SDK, Python and TypeScript policy/config, queue/status/withdraw, owned handle, sink reattachment and typed outcomes. Fresh final-candidate native Python 10/10 and supported Node 25.9.0 TypeScript 6/6 targeted checks passed without skips, including caller loss and actual byte removal. Exact87 governed spellings/53 operations; recovery denylist unchanged. |
| S85-8 | PASS within the limits below: bounded first empirical screen after correctness/review, all outcomes retained, resource ceilings respected, independent accounting and exact assembled-gate provenance. Owner accepts actual observed timings; no invented numerical target or SLA. |

Withdrawal admitted before acquisition drains and wins before activation.
Newcomers while owned receive MaintenanceBusy; after activation/release their
retry returns AlreadyActivated and cannot clear debt or fences. Identity-free
Finalizing retires admission keys; the original ticket remains the completion
retry authority. Status reports current global service restrictions for sibling
and completed tickets.

## Verification provenance

This is an **assembled gate**, not one uninterrupted green invocation:

1. Initial full verification at `f0d4b1253` stopped at module-boundary lint after 58 s.
2. Corrected source at `f04b84602` passed assembled lint, typecheck and strict
   security checks in the checkout-owned environment.
3. Full all-tier collection ran 205 registered components: 193 passed, 12 failed.
   Failures were exact integration inventories, approved surface pins/registries,
   generated product docs, four current-format expectations and associated guards.
4. Narrow corrections at `a2fb4a647` passed all 12 failed component families via
   independent named rechecks. Production callback wiring additionally passed the
   standalone 17 and callback 3/watchdog tests. Historical Python baseline/native
   stub checksum, old runtime-edge mutants and physical success oracles remain.
5. Final native artifacts passed Python 10/10, TypeScript 6/6, actual bidirectional
   format admission and the independent physical/ownership/recovery proof audit.
   Git push hooks passed workspace strict clippy and actionlint.

Final Python native SHA256 is
`0c023e2e0c439cd53880a0962453a92dc201d5284eade07d2574977fa268e792`;
TypeScript addon SHA256 is
`5252194a6f1c3ae5c8b0501e1596711090a566d59099898fc1eb5ad5a3244ab9`.
Node 25.9.0 is the supported TypeScript proof; earlier Node 26 results are supplemental.
The final native/format receipts and Node25 log remain local under `/tmp`.

No broad Rust/Python/TypeScript regression campaign was repeated for these narrow
repairs. All timing ran after builds/tests stopped. Raw local gate logs include
`/tmp/slice85-final-verify.log` and
`/tmp/slice85-agent-test-all-unconfined.log`; they are local diagnostics, not
tracked portable qualification artifacts.

## Observed small-store screen

The committed rerunnable screen is
[src/rust/crates/fathomdb-engine/tests/slice85_qualification.rs](../../../../../src/rust/crates/fathomdb-engine/tests/slice85_qualification.rs):

```sh
cargo test -p fathomdb-engine --test slice85_qualification \
  --features test-hooks,operator -- --ignored --nocapture
```

The first serial screen completed 18 trials in 8.42 s, with three fresh stores per
policy/backlog cell and 100 reads/writes per trial. All 1,800 reads and 1,800 writes
succeeded. Recorded outcomes: 900 newcomer refusals, 9 held-reader acquisition
refusals, 600 exact Accepted retries, 9 ordinary NeedsMaintenanceAdmission results,
54 heavy Accepted results, 9 explicit and 9 Autonomous physical Complete results.
External-reader Incomplete→retry Complete and original-sink
Incomplete→authenticated reattachment→Complete each passed once. No retry of the
campaign, ceiling breach or discarded failed/incomplete attempt occurred.

| Explicit queued backlog | Recorded read median (ms) | Recorded write median (ms) |
| ---: | ---: | ---: |
| 0 | 0.3156 | 1.4898 |
| 1 | 0.3153 | 2.1185 |
| 16 | 0.3227 | 5.9686 |

Autonomous control read/write medians were 0.3156/1.5005 ms. Queued scope checks
increased observed write cost as backlog grew; this is not a no-regression claim.
Across the nine regular explicit trials, physical completion ranged
38.683–40.729 ms and whole owned blackout 53.611–55.654 ms. The external-reader
blocked attempt took 137.743 ms and remained Incomplete before its successful
retry; it is separate from those regular completion ranges.
Drained acquisition ranged 0.0057–0.0152 ms and newcomer refusal
0.0009–0.018 ms. Peak process RSS was 33,832,960 bytes; maximum scratch 4,865,985 bytes.

The protocol ceilings were 120 s/cell, 15 min total, 4 GiB RSS and 2 GiB scratch at safe
API boundaries, not active SQL deadlines. Results cover small Linux x86_64 CPU
stores without an embedder only. No p99, large-corpus, sustained-load,
cross-platform, physical-deadline or storage-remanence claim follows.
Raw JSONL `/tmp/slice85-qualification-a2fb.jsonl` has 5,264 records and SHA256
`c50ba6e0a7304fa1e367540cec241c1fc9ebb2fec70caefc5430007aa1aac982`.
The exact empirical executable SHA256 is
`6fcd22287848b3a20e0fc78b0d7c50412375351b844fb4fab4a30a92f0b724fb`.
Raw JSONL stays local; the committed harness and these reported observations
preserve rerun instructions and the accepted result, not an exported raw trace.

## CI limits and follow-up

[CI run41885604931](https://github.com/fathomadb/fathomdb/actions/runs/41885604931)
used exact source `a2fb4a647` and has five red jobs. Verify lacked the required
Actionlint 1.7.12, Ruff 0.15.17, Pyright 1.1.410 and root js-yaml setup. The parallel
race runner encountered linker BusErrors and exhausted disk before its tests.
Three native-artifact validation jobs (macOS ARM, Linux x64 and Linux ARM) hit
an unchanged legacy smoke assertion in
`scripts/release/smoke/smoke-local-native-artifacts.sh:481`: it expects
GraphEvidenceResolveRequestV1 inline in index.d.ts despite the modular re-export
already present at baseline `db1fdb6dc`. These are recorded setup/smoke limitations;
no CI-green or hosted-platform qualification claim is made. CI infrastructure
repair remains follow-up, outside this slice's implemented product scope.
The optional CI scope question was unanswered; this records the stated default
scope assumption and does not invent an owner deferral or new HITL ruling.

## Landing and cleanup

The existing development checkout remained the sole core writer. The isolated SDK
branch was merged and its temporary worktree/branch removed. Canonical
`release/0.8.28` contains the qualified code; Slice 90 is next. Documentation/state
closure is checked with scoped Markdown/public-doc/state validators only.
Unrelated files and shared caches were not cleaned or staged. Release-wide
qualification and publication remain separate.
