---
title: FathomDB 0.8.28 Slice 95 — execution status
status: IN_PROGRESS
---

# Slice 95 status

**IN_PROGRESS. Publication held.** The reviewed format-37 runtime repair is
integrated into `dev/0.8.28` at
`6d580ec57692241c28a5113e374102cddc42e559`. The unchanged canonical AC011 helper
passes both gates: **1 KB 1,315.423 commits/s** against 1,000 and **100 KB
305.975 commits/s** against 100. Actual helper/time exits are 0; separate
verification confirms source/executable identity and applicable before, after
and enclosing host readiness. No threshold, workload, WAL/NORMAL setting or
publication decision changed.

The complete [before/after history](qualification.md#ac011-before-and-after-the-repairs)
retains the original 999.651557/s failure, then 827.267649/s after lazy edge-query
preparation and 932.474344/s after compiled metadata-query reuse. Earlier 100 KB
was 307.292/s; the two intermediate helpers stopped before that test. These are
separate observations, not a paired causal speedup experiment.

## Implemented repair and documentation

The [accepted technical ADR](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md)
records the measured automatic-reprepare mechanism, alternatives, exact 57-trigger
manifest and admission inventory. Transaction-owned visibility coalescing replaces
native trigger toggles. All outer writer families, boot effects and dispatcher
cursor repair validate clean state while holding the SQLite writer lock.
Generation/nonce publication and rollback remain atomic; authenticated vector
cleanup stays active. Current-only admission advances to format 37 without a
nonempty data migration. Maintained architecture, engine/bootstrap designs,
interfaces, public compatibility and release notes describe the implementation.

Astra approved design and exact code at `6d580ec57`. The implementer passes
Clippy/typecheck, 19 visibility units, four fallback units, eight historical-token
controls, 127 tests across 13 integration targets, 10 schema tests and scoped
documentation/state checks. Separate verification executes nine Cargo commands,
all exit 0: 44 unit executions / 32 unique tests, 56 integration passes and three
actuation passes, plus the store-format parent's deliberately invoked child.
No vacuous selectors or failed tests were accepted.

Guard RED is `9bed85123`; boot RED is immutable intermediate `85fe1ba1a`;
corrected dispatcher RED is `a2d352d0e`. Original dispatcher `637496211` and its
first export/debug logs are **invalid/inconclusive fixture evidence**. The
corrected fixture proves a real clean cursor rewind before its refusal oracle.
New guard/admission/format tests remain frozen; original token fixture bytes and
assertions remain unchanged. Historical format exceptions are narrowly compiled
under test/migration-test-hooks; ordinary production and current-format
admission stay strict.

Raw implementation and separate-verification logs are copied into canonical
`target/slice95-ac011-evidence/visibility-coalescing/` and
`visibility-coalescing-verifier/`. The current passing executable is preserved
by hash beside its qualifier logs. Full local and hosted qualification has not
yet run for this new format-37 runtime.

## Final qualification still required

| Obligation | Current disposition |
| --- | --- |
| Full local and feature/security gates | Pending one final clean code-phase round with checkout-owned tools/target. Prior format-36 source `16234813c` passed 205/205 with actual child 0; outer 143 had unknown cause. Earlier feature assembly was 180 planned / 165 pass / 15 reviewed ignored. These are historical results, not format-37 qualification. |
| Other performance/reliability | Refresh AC012/013/081, long AC021 and advancing-cursor checks on fresh candidate databases. Earlier limits passed with the recorded AC081 soft warnings. No extra AC011 retry is planned. |
| Native platforms and consumers | Refresh the five supported CPU wheel/addon receipts, collector, separate Windows WAL proof and external Rust source-archive consumer against this candidate. No registry-resolved .28 claim. |
| Accepted fidelity | Rebuild candidate CUDA wheel/CLI/private benchmark and run the unchanged TC5 bridge on a fresh format-37 database. Frozen corpus/model inputs remain. Earlier source `5f668f330` measured recall .958, CI [.938,.974]; it is historical private vector-stage evidence, with the recorded post-run database-hash caveat. |
| Hosted CI | Fresh exact-candidate dispatch pending. Prior run 42005653176 on `3a05a45cb` failed only fast classification lint before its 198 suites; all five native/collector, heavy 7/7, race/security/default-provider and Windows WAL jobs passed on that older source. Classification correction `af7e570ff` and local fast 198/198 passed; no current overall CI-green claim. |
| Documentation and evidence | Slice 92 baseline consumed; format-37 design/interface/public-document deltas have scoped checks. Current decision/rationale and full measurement history are being finalized with review. Source and retained generated-evidence scans remain distinct. |
| .29 handover and cleanup | Prior repairs/tools are forwarded through `c25d045cf`. Forward this reviewed runtime and final records, run scoped consumer checks, then remove the clean merged task worktree/branch after preserving logs. |
| Nonpublishing rehearsal | Separate S95-11 disposition: aligned .28 version/Axis E and trusted-workflow preparation remain pending. Do not substitute an old-version rehearsal or claim pre-tag readiness. No version cut, main merge, tag, registry/Pages publication or release dispatch occurred. |

CI1–15 engineering is already implemented and reviewed: cold prerequisite setup,
bounded parallel builds/cache and diagnostics, installed modular TypeScript
contracts, confirmed-debt service admission, below-capacity subscriber delivery,
provider-close sensitivity and positive advancing-cursor proof. Their historical
qualification remains source-attributed in [qualification-evidence.json](qualification-evidence.json).
No unrelated erasure optimization or new test framework is allocated.

## Investigation, contract limits and retained resources

Offline profile recovery is complete and independently verified. The unchanged
recording resolves 3,254/3,284 foreground leaves (99.0865%); 858 samples are
SQLite automatic reprepare. Native DWARF recovery and independent Samply/Framehop
corroboration retain the eight explicit chain differences and incomplete-unwind
limitations. Diagnostic rates never qualify AC011. Original and failed exports
remain retained; no further capture was made.

The accepted durability ADR's WAL/NORMAL, zero-OS-crash-loss and 100 ms power-loss
wording has an unresolved contract/evidence discrepancy. No successor policy,
waiver or new owner ruling was selected. The earlier policy-choice question was
withdrawn as a prerequisite for this policy-neutral repair; it is not an open
permission request to proceed with profiling or SQL work. Do not call process-kill
proof a machine-crash/power-cut guarantee.

The canonical checkout's original `target/debug` symlink points at unmounted cold
storage. Do not discard it or change the mount; use a fresh checkout-owned target
for final verification. The standing Windows VM was observed off for AC011; its
original persistent configuration and runner remain preserved. Prior CI1–15 task
worktrees/branches, disposable VM resources and staged corpus copies were cleaned.
Current runtime task resources remain until verified handover/cleanup.

D28-09/10/11 and populated-open deferrals remain unchanged. Slice 95 stays open
until required final qualification and cleanup are complete. Publication remains
a separate owner decision and is held.
