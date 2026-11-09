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
by hash beside its qualifier logs. The broad local format-37 collection has now run; its nonzero result and focused
follow-ups are recorded below. Hosted qualification remains pending.

## Harness qualification follow-up

Initial local attempt at `022341dbc` stopped before workspace Clippy/typecheck/tests on missing
module classifications. Reviewed metadata-only `8798ba038` passes its exact
boundary gate: 118 modules, 18 governed and 104 configurations. The resumed
attempt passed lint/typecheck, then strict security reported 0 violations,
2 blockers and 0 downgrades: the wrappers searched the hardcoded cache path rather
than the owned Cargo target. Live namespace catch itself passed; application
security is not claimed from that incomplete battery. No broad suites ran.

CI run42542957407 on `8798ba038` rejected both macOS ARM image cells before steps.
It was cancelled after preserving the failure annotations and actual results:
22 successful, 11 cancelled and one skipped job. No overall CI pass is claimed.
Reviewed harness repair `c02990583` resolves Cargo target paths and uses supported
ARM runner images while keeping delivery/deployment/budget contracts unchanged.
Its 20 path cases, policy/platform/native fixtures and permanent registration
pass independently. The resumed clean collection passes real strict security
with zero violations, blockers and downgrades; full qualification remains open.
See the [bounded design](design.md#ci17-owned-target-paths-and-supported-macos-arm-runners).

## Final qualification still required

| Obligation | Current disposition |
| --- | --- |
| Full local and feature/security gates | Clean `ff253a0a0` collection: 206 ran / 202 passed / four failed; lint, typecheck and real strict security passed. Rust originally failed seven targets; corpus_graph now passes its unchanged 1/1 recheck after verified input staging. Six targets still require focused diagnosis; root inventory is stale. Generated contributor map refreshed; Pages-site 20/20 and retention 45/45 resumed passes. Feature qualification still pending. Prior format-36 source `16234813c` passed 205/205 with actual child 0; outer 143 had unknown cause. Earlier feature assembly was 180 planned / 165 pass / 15 reviewed ignored. These are historical results, not format-37 qualification. |
| Other performance/reliability | Refresh AC012/013/081, long AC021 and advancing-cursor checks on fresh candidate databases. Earlier limits passed with the recorded AC081 soft warnings. The 6d source pass remains retained; no same-binary retry is planned. One final version-aligned canonical pair is allocated for the final artifacts. |
| Native platforms and consumers | Refresh the five supported CPU wheel/addon receipts, collector, separate Windows WAL proof and external Rust source-archive consumer against this candidate. No registry-resolved .28 claim. |
| Accepted fidelity | Rebuild candidate CUDA wheel/CLI/private benchmark and run the unchanged TC5 bridge on a fresh format-37 database. Frozen corpus/model inputs remain. Earlier source `5f668f330` measured recall .958, CI [.938,.974]; it is historical private vector-stage evidence, with the recorded post-run database-hash caveat. |
| Hosted CI | Fresh exact-candidate dispatch pending. Prior run 42005653176 on `3a05a45cb` failed only fast classification lint before its 198 suites; all five native/collector, heavy 7/7, race/security/default-provider and Windows WAL jobs passed on that older source. Classification correction `af7e570ff` and local fast 198/198 passed; no current overall CI-green claim. |
| Documentation and evidence | Slice 92 baseline consumed; format-37 design/interface/public-document deltas have scoped checks. Current decision/rationale and full measurement history are committed and reviewed. Source and retained generated-evidence scans remain distinct. |
| .29 handover and cleanup | Prior repairs/tools are forwarded through `c25d045cf`. Forward this reviewed runtime and final records, run scoped consumer checks, then remove the clean merged task worktree/branch after preserving logs. |
| Nonpublishing rehearsal | Owner authorizes local .28 version preparation; Axis-W setter/lockstep checks pass at .28. Axis E stays 0.8.0 with its existing not-yet-on-registry/publish-time compiled-check disposition; final artifacts remain pending. Hosted trusted route still requires main ancestry, while main pushes can deploy Pages. Keep that prerequisite and publication held. No main merge, tag, registry/Pages publication or release dispatch occurred. |

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

## Current focused follow-up

Actual local full exit is 1 (`target/slice95-final-format37/local-verify-ff253.log`),
not an uninterrupted pass. The seven Rust targets cover schema refusal/format
expectations, physical transaction retryability, corpus graph and profiling
counts. Corpus_graph now passes its unchanged 1/1 rerun after verified input staging; six original targets remain. Diagnose before changing assertions. The two Pages failures share the
stale generated contributor-map hash and now pass their affected reruns. Verified
corpus copies are staged for the existing local corpus route, with acquisition
limitations unchanged. TC5 inputs are independently verified; parser final-version
support has Astra approval and 22/22 independent passes and is integrated at `48345c193`; its canonical recheck also passes 22/22.
Mechanical local Axis W is now 0.8.28; lockstep checks and the unchanged Axis-E
0.8.0 guard pass. No artifact or release-workflow pass follows from that metadata.
No new Actions run has been dispatched while local qualification is incomplete.

Astra approved focused CI18–19 corrections: real current-schema rollback fixture,
format37 refusal expectations and quiet idle-probe admission. The latter has an
observed contention RED and reviewed GREEN; remaining targeted verification is
pending. Current wheel evidence also has a reproduced stale supported34 assertion;
its bounded format37 producer/verifier and .28-aware manifest correction is planned
without changing historical .26 receipts. Local versioned release preflight at
`48345c193` passes; full local rehearsal and final artifact qualification remain.
