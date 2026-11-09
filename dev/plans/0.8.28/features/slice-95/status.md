---
title: FathomDB 0.8.28 Slice 95 — execution status
status: IN_PROGRESS
---

# Slice 95 status

**IN_PROGRESS. Publication held.** Full manual CPU CI42621641800 passes at
`3f80b1c96f0ffc75bb37119664d6c761dedda75a`:34 jobs (33 success, one intentional
Markdown skip),206/206 suites and all five native platform rows/collector.
The separate Tegra workflow failed its allocation witness. CI20 repairs checked
CUDA completion and failure-evidence retention; independent compilation and
focused tests pass, but fresh hardware qualification still fails on its second
process (9539584 <67108864 bytes). The campaign stopped without retry. CI22
investigates same-context private-pool accounting before any witness replacement.
CI21 repairs12 negative elapsed-time reports without changing test verdicts.

Local engine/performance qualification remains bound to its recorded sources,
including final AC0111215.559/s and291.286/s against unchanged1000/100 floors.
The final repaired exact-source CI, Tegra proof, nonpublishing release rehearsal,
records handover and task-resource cleanup remain. Hosted CPU/macOS is approved;
all GPU work stays local. The exact old task disk has been deleted and verified.

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
| CPU installed platform rows | Source e090: Linux x64, Linux ARM64 and Windows x64 production wheel/addon consumers pass independent audits, with actual isolated public TypeScript and real database controls. Linux wheels and ARM Node meet glibc2.28; the host-built x64 Node requires2.39, with its failed2.28 probe retained. Windows runs 186/186 tests across13 modules, zero skipped. This is three local runtime rows, not the five-platform matrix or collector pass; both Mac rows remain. |
| Windows WAL | All ten exact engine controls plus the external-reader parent/child pass; installed current serial/binding/retained controls pass, including five owned-reader busy cases. Released0.8.22 comparison returns the existing accepted exit66/clean completion/zero frames. All11 enclosing qualification/receipt phases exit0. Production artifacts and disposable hook wheel stay separate. |
| Local x64 CUDA packages | Both existing preflight variants and installed-package rehearsals pass. Actual Python/Node GPU records bind the second RTX3090 UUID and process; reranker package smoke executes one embedding and one reranker forward per consumer. CLI doctor/driverless controls pass; no CLI GPU inference or trusted-workflow claim. Export-only Cargo jobs2 is a recorded build configuration delta. Initial package-smoke exit1 from an incorrect CLI archive basename is retained; identical archive bytes under the required filename pass the corrected scoped attempt. |
| Tegra packages | Python0.8.28+tegra and separately named Node0.8.28 build/install and glibc2.35/linkage checks pass. Independent Python/Node audits confirm installed bytes, six model pins, CPU/auto/forced-CUDA policies, embedding allocation witness, actual Node reranker results, loader decoy precedence and heap3/3. The generic ARM CPU row is separate. The maintained driverless Tegra-addon helper explicitly refuses a Tegra host; its genuine non-Tegra ARM control remains pending. |
| External Rust consumer | Exact 2e source archive: 616 exported objects match Git. Offline isolated build and real write/read/erase/control-survival/close/reopen/exit pass. Six Fathom dependencies resolve inside the archive; no test hooks or added patches. Relevant Rust/Cargo inputs unchanged at e090. |
| Accepted TC5 fidelity | Fresh database, 7667 real documents, 100 queries, 1000 bootstrap resamples, zero synthetic: recall 0.958, 95% CI [0.938, 0.974]; accepted CI-high >=0.90 passes. Raw executor0 and independent query/hash/bootstrap audit pass. CUDA embeddings used the explicitly pinned second RTX 3090; CPU sqlite-vec candidates/exact-f32 rerank. Private-stage fidelity only, no latency/public-search/whole-hybrid claim. Post-run DB hash differs from the pre-query hash; retained without an immutability claim. Raw ID overlap remains executor-attested. |
| AC011 final version | Unchanged canonical helper 0, 1215.559/s and 291.286/s; compiler-selected default/test-hooks optimized binary matches its sealed copy. Before/after/enclosing readiness applicable. Machine-wide swap-in 142 pages is retained diagnostic data, not a zero-swap claim. |
| AC012/013 | N=10000 and 1000 samples: text p50/p99=1/2ms; vector 56/86ms with configured/default dimension 768. Numerical gates pass. Dimension provenance is the actual driver environment plus unchanged source default; the raw marker does not print it. Only outer pre-run readiness exists; no immediate-end/enclosing readiness claim. |
| AC081 absolute reads | Seven fresh applicable observations of one sealed binary pass hard 500/100ms limits. Sequential/concurrent medians 259.829636/43.159602ms; all seven retain sequential soft warnings. Initial numeric-pass observation at 90.375C is separately retained as environment-invalid. Astra approved one fresh campaign after natural cooling with readiness before every cell; no threshold change, replacement loop or select-best result. Fresh-campaign swap-in 3 pages is diagnostic. |
| Long correctness | Independent AC021: 60s, 60 DDL cycles, 60 rebuilds, zero SQLITE_SCHEMA warnings. Cursor: 1000 searches, 199 successful writes/198 during reads, 999 advancing hits, zero cursor/hit violations. Both actual exits0. |
| Legacy corpus stress | Actual overall exit 101 from superseded recall 0.775/CI [0.746, 0.801]; never an overall pass. Separate accepted AC073 stress passes 46ms <301ms with 8x250 requests, N=7667, no padding. Real setup 18472 documents, 100 queries/1000 bootstrap/1000 latency samples. 13 staged actual-hash inputs retain the MuSiQue acquisition mismatch and absent Hotpot limitation; MuSiQue contributes no accepted rows in this route. |
| Local release rehearsal | Existing full local dry-run once: workspace optimized build 0, three leaf packages and three publish dry-runs pass with all uploads aborted. API 0.8.0 / schema 0.8.28 / query 0.8.28 archives retained. Axis-E guard passes its unpublished-version branch, not a compiled published-API comparison. |
| .29 handover | Clean0a47524ca forwards the final package correction/earlier records; Astra approves the preserved root inventory and explicit .28/.29 format37 candidate mapping. Added acceptance/refusal cases fail before the bounded correction and pass after it. Six independent focused package/version/inventory checks pass; runtime inputs remain identical to the prior577d243a3 baseline with42 MCP/visibility passes. Axis W remains the .28 preparation baseline. Current local records will be forwarded too; no .29 full-gate or .28 platform claim. |

Generated-evidence safety scans retain actual advisory exit1: the initial38
findings and two final-metadata findings are corroborated model/toolchain/artifact
SHA256 values, with zero credential findings. Tracked-source scan0 found no leaks. No scanner policy,
suppression or integrity artifact changed.

## Remaining work

- Complete both Mac CPU wheel/addon runtime rows and the five-target collector,
  plus the genuine non-Tegra ARM driverless Tegra-addon control. Existing local
  machines cannot satisfy those host requirements; no platform waiver is inferred.
- Resolve whether the owner's local-only condition covers GPU execution only or
  all execution, before any hosted CPU/macOS dispatch. The owner already approved
  environment review for local execution including Windows and Tegra; do not ask
  for that same approval again within its confirmed condition.
- Complete exact-candidate CI and the separate nonpublishing release-workflow
  rehearsal. The reviewed skip-marked/manual route preserves Pages/publication
  holds and existing environment/ancestry controls. Local helper passes do not
  supply trusted-workflow provenance. No hosted run was dispatched under the
  conditional approval.
- Finish task-resource cleanup after the remaining execution gates. The local
  baseline records passed scoped checks and were forwarded to .29. Close
  status/state only after all required qualification.

## Retained limits and resources

The original task worktree/branch is clean, fully merged and removed; 73 evidence
files were copied byte-for-byte. Canonical dev/release checkouts remain. The
standing Windows VM is gracefully off; original runner hashes, Auto/Stopped
service, policies and model cache are verified unchanged. Its original domain
definition is restored exactly; the owned64GiB task disk is detached and retained.
Automatic approval review rejected irreversible task-disk deletion without
explicit owner approval for that exact volume; the deletion question is pending. No task runner
registration/token/listener was created yet. A local Linux X64 GPU runner is
also not registered; do not start or modify the standing Fathom/Coral services.
Tegra's existing dirty checkout is untouched. Isolated task exports/environments
and targets remain until final audit/cleanup; all task test containers have exited.

The original target/debug symlink still points at absent cold storage; preserve
it. Current qualification uses owned targets and sealed artifacts. All13 task-staged raw corpus copies
were removed after both original/copy hashes and sizes were checked; original
inputs are preserved. The restored TC5 input pack
and qualified evidence remain retained for replay. Older /tmp 86fb evidence paths
are absent; their tracked historical transcriptions are not present raw logs.

Offline profile recovery is complete; the original recording and byte-identical
recovered ELF remain preserved. No extra profile was collected. The literal
accepted durability ADR's zero-OS-crash/100ms wording retains its documented
contract/evidence discrepancy; no successor policy or waiver was selected, and
process-kill tests are not machine-crash/power-cut proof. D28-09/10/11 and
populated-open deferrals remain unchanged. Publication is a separate held step.
