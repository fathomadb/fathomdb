---
title: FathomDB 0.8.28 Slice 95 — execution status
status: IN_PROGRESS
---

# Slice 95 status

**IN_PROGRESS. Publication held.** Local core qualification is complete as an
assembled result at `e090418becabd9610cea3b372f249e9f8bd3ccf9`. The final versioned
AC011 pair passes at **1,215.559 commits/s for 1 KB** and **291.286 commits/s for
100 KB**, with unchanged floors of 1,000 and 100. Native platform/package and CI
qualification, execution-location clarification, final handover and resource
cleanup remain. There is no published-release or overall CI-green claim.

## Implemented and reviewed

The [accepted technical ADR](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md)
records transactional visibility coalescing, all 57 triggers, writer-lock
admission, fresh format37 and unchanged external invalidation/vector cleanup.
The repair at `6d580ec57` and the final idle-probe correction at `674ec8a51`
have Astra design/code approval and independent verification. The latter wraps
only initial maintenance writer admission in the existing quiet-probe scope;
actual cleanup and foreground telemetry remain observable. The real-schema
rollback fixture now proves callback/insert, failed deferred COMMIT, rollback,
autocommit and retry. Approved current-format fixture updates retain refusal
and immutability oracles. Independent focused verification passes 35 tests.

CI1–19 include cold setup, bounded diagnostic resources, installed TS export
checks, safe diagnostics/manual routing, confirmed-debt admission, subscriber
and cursor controls, owned Cargo paths, supported macOS ARM images and format37
package evidence. Astra approved each bounded change; separate checks pass.
Package correction `8c5f543a4` is merged: current wheel evidence requires37,
while historical .26/34 and Slice65 controls remain unchanged.

Maintained architecture, interfaces and pending public documentation describe the
format break and ownership boundaries. The [full measurement history](qualification.md#ac011-before-and-after-the-repairs)
retains every earlier non-pass and source; distinct runs are not a causal speedup
experiment. No threshold, WAL/NORMAL setting or durability policy changed.

## Current local proof

Raw current evidence is under canonical `target/slice95-final-format37/`.
[Qualification evidence](qualification-evidence.json) records source and scope.

| Obligation | Verified result and limit |
| --- | --- |
| Broad local collection | Clean `ff253a0a0`: all 206 ran, 202 initially passed, four failed. Lint/typecheck and real strict security passed,0 violations / 0 blockers / 0 downgrades. Root inventory8/8, Pages20/20 and retention45/45, corpus graph1/1 and the affected Rust cases pass focused resumptions after reviewed corrections. This is assembled coverage, not an uninterrupted final full-gate pass. |
| Feature coverage | Source 2e2189429: 23 runs, 180 planned, 165 passed, 15 exact allowlisted ignored,zero failures/skip markers/missing results. Independent audit confirms live prerequisites and retained GPU receipts. Ignored tests remain exemptions, not passes. Relevant inputs are identical at e090. |
| Fresh artifacts and installed Python | Final 0.8.28 CUDA wheel/CLI/private executor built from 2e2189429; 637 relevant artifact inputs identical at e090. Binaries preserved separately from mutable Cargo caches. Isolated noneditable wheel: 19 Python files/native bytes match; 24 graph rows/reopen pass, schema 33 refuses supported 37 with unchanged DB/lock bytes. No private test hooks/reranker in this wheel. CPU execution of a CUDA-capable local wheel is not the five-platform CPU delivery matrix. |
| External Rust consumer | Exact 2e source archive: 616 exported objects match Git. Offline isolated build and real write/read/erase/control-survival/close/reopen/exit pass. Six Fathom dependencies resolve inside the archive; no test hooks or added patches. Relevant Rust/Cargo inputs unchanged at e090. |
| Accepted TC5 fidelity | Fresh database, 7667 real documents, 100 queries, 1000 bootstrap resamples, zero synthetic: recall 0.958, 95% CI [0.938, 0.974]; accepted CI-high >=0.90 passes. Raw executor0 and independent query/hash/bootstrap audit pass. CUDA embeddings used the explicitly pinned second RTX 3090; CPU sqlite-vec candidates/exact-f32 rerank. Private-stage fidelity only, no latency/public-search/whole-hybrid claim. Post-run DB hash differs from the pre-query hash; retained without an immutability claim. Raw ID overlap remains executor-attested. |
| AC011 final version | Unchanged canonical helper 0, 1215.559/s and 291.286/s; compiler-selected default/test-hooks optimized binary matches its sealed copy. Before/after/enclosing readiness applicable. Machine-wide swap-in 142 pages is retained diagnostic data, not a zero-swap claim. |
| AC012/013 | N=10000 and 1000 samples: text p50/p99=1/2ms; vector 56/86ms with configured/default dimension 768. Numerical gates pass. Dimension provenance is the actual driver environment plus unchanged source default; the raw marker does not print it. Only outer pre-run readiness exists; no immediate-end/enclosing readiness claim. |
| AC081 absolute reads | Seven fresh applicable observations of one sealed binary pass hard 500/100ms limits. Sequential/concurrent medians 259.829636/43.159602ms; all seven retain sequential soft warnings. Initial numeric-pass observation at 90.375C is separately retained as environment-invalid. Astra approved one fresh campaign after natural cooling with readiness before every cell; no threshold change, replacement loop or select-best result. Fresh-campaign swap-in 3 pages is diagnostic. |
| Long correctness | Independent AC021: 60s, 60 DDL cycles, 60 rebuilds, zero SQLITE_SCHEMA warnings. Cursor: 1000 searches, 199 successful writes/198 during reads, 999 advancing hits, zero cursor/hit violations. Both actual exits0. |
| Legacy corpus stress | Actual overall exit 101 from superseded recall 0.775/CI [0.746, 0.801]; never an overall pass. Separate accepted AC073 stress passes 46ms <301ms with 8x250 requests, N=7667, no padding. Real setup 18472 documents, 100 queries/1000 bootstrap/1000 latency samples. 13 staged actual-hash inputs retain the MuSiQue acquisition mismatch and absent Hotpot limitation; MuSiQue contributes no accepted rows in this route. |
| Local release rehearsal | Existing full local dry-run once: workspace optimized build 0, three leaf packages and three publish dry-runs pass with all uploads aborted. API 0.8.0 / schema 0.8.28 / query 0.8.28 archives retained. Axis-E guard passes its unpublished-version branch, not a compiled published-API comparison. |
| .29 runtime handover | Clean 577d243a3: Astra-approved conflict resolution and 42 independent scoped MCP/visibility tests pass; version/inventory/module/Markdown checks pass. Final package/record handover remains. No .29 full-gate or .28 platform claim from these tests. |

## Remaining work

- Finish the five supported CPU wheel/addon installed-consumer rows and collector,
  separate Windows WAL proof, and applicable local Tegra package/runtime checks.
  Current source is not registry-qualified or macOS-qualified by local Linux proof.
- Resolve the scope of the owner's local-only condition before any hosted
  dispatch. No Mac/platform waiver is inferred. Preserve all required evidence
  in the plan rather than silently narrowing the platform matrix.
- Complete exact-candidate CI and the separate nonpublishing release-workflow
  rehearsal subject to that execution condition. The reviewed skip-marked
  candidate route can keep push-triggered Pages/duplicate CI dormant while
  manual dispatch remains available. Existing environment controls remain.
- Run final source and retained-evidence safety scans, finish scoped documentation
  checks and .29 forwarding, then close the status/state only after all required
  qualification and cleanup are proven.

The owner confirmed coreyt and approved environment review **only for local
execution**, explicitly including the Windows VM and Tegra 10.83.10.13. Do not
request that same approval again once its condition is confirmed. The pending
question concerns whether remaining hosted CPU/macOS jobs are permitted; no
hosted run has been dispatched under this conditional approval.

## Retained limits and resources

The original task worktree/branch is clean, fully merged and removed; 73 evidence
files were copied byte-for-byte. Canonical dev/release checkouts remain. The
standing Windows VM's owned 64GiB task disk is still configured; original XML,
runner hashes/service/policies must be restored at cleanup. No task runner
registration/token/listener was created yet. A local Linux X64 GPU runner is
also not registered; do not start or modify the standing Fathom/Coral services.
Tegra's existing dirty checkout was inspected read-only and must remain untouched.

The original target/debug symlink still points at absent cold storage; preserve
it. Current qualification uses owned targets and sealed artifacts. Staged raw
corpus copies remain until dependent work finishes. The restored TC5 input pack
and qualified evidence remain retained for replay. Older /tmp 86fb evidence paths
are absent; their tracked historical transcriptions are not present raw logs.

Offline profile recovery is complete; the original recording and byte-identical
recovered ELF remain preserved. No extra profile was collected. The literal
accepted durability ADR's zero-OS-crash/100ms wording retains its documented
contract/evidence discrepancy; no successor policy or waiver was selected, and
process-kill tests are not machine-crash/power-cut proof. D28-09/10/11 and
populated-open deferrals remain unchanged. Publication is a separate held step.
