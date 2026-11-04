---
title: Slice 85 — implementation and qualification status
status: ACTIVE
date: 2026-11-04
---

# Slice 85 status

**Implemented on `dev/0.8.28`; final review and independent qualification pending.**
The approved [contract](../../../../design/explicit-maintenance-erasure/contract.md)
and Astra-reviewed [production design](../../../../design/explicit-maintenance-erasure/production-design.md)
are implemented in format 36. No publication or Slice 90 work is authorized here.

Baseline reconciliation at `db1fdb6dc` retained accepted Slice 83 production and
rejected 84/84b optimizations. The plan enumerates the seven actual baseline facts,
requirements, acceptance and mutation/ownership inventory. Normal new admission
returns NeedsMaintenanceAdmission because the existing exact envelope scans the
store; the handle provides heavy admission. Queued data retains existing visibility.

## Acceptance audit before final gate

All tests below use real SQLite files; existing physical and compiler oracles were
preserved. RED/GREEN behavior commits retain their actual failure evidence.

| Acceptance | Implemented behavior and scoped evidence | Remaining final check |
| --- | --- | --- |
| S85-1 | Policy/format tests pass; fresh36 metadata mandatory, mismatching policy refused before mutation, explicit reopen has no worker. Independent actual format35 executable ↔ actual format36 native refusal passed with main/WAL/SHM unchanged. Current-format/cutover/WAL fixtures updated mechanically and scoped suites pass. | Final integrated native artifact provenance |
| S85-2 | Queue7 tests, scope6 tests: exact key/conflict/retirement, withdrawal/reopen,16 queue and4096 reserved receipt capacity,16385-target refusal, full logical dependency closure, generated writer histories, same-batch old/new endpoint/provenance/dependency guards; resource growth retains Queued without deletion. | Independent rerun |
| S85-3 | Ownership5, owner-races3, callback3 and provider2 tests: complete outer operation leases, counted scanner publication, release/thaw ordering, fast newcomers, self-drain/close watchdogs, stale handles, original provider close cancellation. | Independent rerun |
| S85-4 | Physical family/scope witnesses populate all4 FTS families, registered non-head vectors and dependent rows; exact zero, queue identity scrub, main/WAL absence and independent survivor-only BM25 score/order pass. Native deletion/root/ticket activation and redacted/final receipt transitions share transactions. | Final qualification |
| S85-5 | Eight actual process exits at queue/delete/redaction/receipt boundaries replay correctly. Rollback, committed failure/reopen fences, pinned external WAL reader, corrupt policy/root/ticket and partial finalization authority tests pass. Sanitation progress is durable and truthful. | Independent rerun |
| S85-6 | Late second original, wrong-sink refusal, retained multiple originals and redaction pass. Offline completes the same activated receipt and never activates queued intent; corrupt offline authority refuses before main-file mutation. | Independent rerun |
| S85-7 | Integrated Rust/Python/TypeScript config, policy, queue/status/withdraw, handle, sink reattachment and outcomes; SDK native tests passed on the binding branch, including caller cancellation/drop. Public interfaces/docs and conformance maps updated. | Integrated full gate/native rebuild |
| S85-8 | Owner accepts measured timings. Frozen empirical screen compiled; no timing run or performance conclusion yet. Engine library clippy with warnings denied passed. | Astra final review, independent full gate, empirical screen and supported platform limits |

The withdrawal ordering is explicit: an admitted withdrawal completes inside the
acquisition drain and wins before activation. Newcomers during ownership receive
MaintenanceBusy; after activation and release, retry returns AlreadyActivated and
cannot clear debt or fences. Astra confirmed this precedence before qualification.

## Re-runnable qualification

Run the bounded empirical screen only after correctness/review, with no other
build/test running:

```sh
cargo test -p fathomdb-engine --test slice85_qualification \
  --features test-hooks,operator -- --ignored --nocapture
```

The committed screen uses18 matched policy/corpus cells (three stores per cell),
100 reads/writes and applicable exact-key retries/newcomer refusals, explicit
acquisition/drain, physical blackout/completion, external-reader and original-sink
incomplete/retry controls. It records every outcome and raw latency to a new JSONL
file under `/tmp`, plus peak RSS and scratch. Limits are120 s per cell,15 min total,
4 GiB RSS and2 GiB scratch at safe API boundaries; they are not product deadlines.
No p99, SLA, large-corpus or platform-general claim follows from this small screen.

## Landing and limits

The existing development checkout remained the sole core writer. The isolated SDK
branch was integrated and its temporary worktree/branch removed. The existing
canonical release checkout is to be fast-forwarded only after actual final checks;
release-state/board completion remains pending. Shared caches and unrelated files
were not cleaned or staged.

Current local evidence is Linux x86_64. Final verification must report native
artifact, operator/test-hook coverage, any platform/CI limits and exact resumed
checks honestly. No full-workspace green or slice-complete claim is made yet.
