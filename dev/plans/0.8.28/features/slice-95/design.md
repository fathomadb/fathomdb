---
title: Slice 95 — bounded CI repair and candidate qualification design
status: APPROVED
target_release: 0.8.28
---

# Slice 95 design

## Execution reconciliation

Execution baseline: `6ae55c2e42e5aa447bc9f13265f6bf5a9e97f6e2`.
The [improved plan](plan.md) remains the scope and S95-1–11 acceptance authority.
Changes since its reconciliation baseline `48b23c479` are:

1. Slice 92 completed documentation convergence, all 27 maintained-owner audits,
   pending .28 public-doc truth, source-bound format-36 checking and actual CPU
   consumer examples. Its [status](../slice-92/status.md) bounds reuse; no final
   platform/optimized-artifact qualification was supplied.
2. Three existing plan/board renderers were registered; 13 generated regions
   now agree. Preserve their ownership rather than hand-editing current pointers.
3. The full Slice 92 collection ran all 205 suites: 196 initially passed and
   nine recovered by narrow unchanged rechecks. Prose/state-view corrections and
   disk cleanup are accounted for; there is no uninterrupted full-gate claim.
4. Refreshed gh evidence on 2026-11-04 leaves run 41885604931 at older
   `a2fb4a647`: four native legs failed, Windows native queued, fast/heavy/race
   failed and separate Windows WAL succeeded. None qualifies this candidate.
5. Source review confirms CI1/3/4/6/7, CI5 is fixture isolation, and CI2 is a
   resource failure rather than evidence of a product defect. No runtime repair
   is allocated. The seven repairs below remain needed.
6. The actual host supports live namespaces/ptrace and two RTX 3090s outside
   the sandbox. The checkout has its own pinned venv/Node tools and about 11 GB
   free, but feature builds change profile settings and need a storage budget.
   EU7 corpus is absent here although available in the primary checkout; stage
   byte-verified disposable inputs before timing. Do not borrow another venv.
7. Immutable nonpublishing release dispatch already exempts main ancestry in
   `verify-release-gates.sh`. Version/Axis E and trusted workflow prerequisites
   remain; the pending .28 docs and unchanged .27 manifests are distinct.
   No version cut, main integration or publication is authorized here.

Accept the plan with these source-grounded adjustments. Retain owner deferrals,
ordinary regression coverage and Slice 92's documentation baseline. No new
policy, scheduler, general validator or erasure campaign is needed.

Astra design review APPROVE on 2026-11-04: the bounded repairs, focused tests
and qualification/publication limits are accepted. Resource settings remain
provisional until hosted measurement; spill isolation is invocation-owned.

## Repair design and RED/GREEN

Each behavioral repair gets an observed, staged/committed RED before GREEN.
Keep substantive test assertions fixed during implementation. Explicitly replace
old fixture assumptions that contradict the reviewed intended contract.

| Repair | Smallest intended change | Decisive tests |
| --- | --- | --- |
| CI1 cold setup | Extract canonical pinned Actionlint installation from full bootstrap for reuse. Heavy setup uses Python dev extra, root/TS locked npm installs and existing ShellCheck installer. Provision cheap prerequisites before native compilation; avoid full bootstrap's unrelated Cargo/OS tools. Keep verifier preflight strict. | Extend existing heavy-bootstrap fixture for actual prerequisites, cold/cache-restored execution and correct paths; preserve missing/wrong-tool controls and installer failure propagation. Replace the obsolete rule forbidding needed developer tools explicitly. |
| CI2 parallel resources | Scope test debug=0, incremental=0 and Cargo build jobs=2 to diagnostic job; preserve libtest concurrency. Disable diagnostic target-cache restore/save with the pinned action's supported option, retaining dependency caching. Record free bytes/memory/effective settings and target/scratch sizes before/after build/test/upload/post-processing; do not delete runner/shared tool installations. | Existing efficiency/race runner controls prove configuration and environment/arguments with fake Cargo. Hosted run must reach tests and supply real exit/count/log and resource transitions. Chosen settings are provisional until measured. Advisory wrapper reports Cargo failure separately from upload/cache failure. |
| CI3 package exports | Replace inline declaration regexes with one strict installed-package TS consumer importing GraphEvidenceResolveRequestV1 and calling the public method with meaningful typed input. Compile in isolated consumer without source path mappings; use the same proof on Unix/Windows, then retain native calls. | Real modular package must compile; disposable installed package missing only its public export must fail while internal declaration and checkout remain. Retain package/declaration hashes separately from wheel/addon receipts. No source compilation substitutes for this proof. |
| CI4 Cargo color | Pass --color never at cargo-tree command boundary, retaining exact dependency/version/source and forbidden-feature parsing. | CARGO_TERM_COLOR=always legitimate fixture must pass; all wrong package/version/source/feature controls remain decisive. |
| CI5 ShellCheck fixtures | Restrict fixture tool lookup/HOME/cache instead of changing the correct production resolver. Force installer/cache branches independently of host shellcheck. | Wrong/missing binaries fail for named reasons; pinned HOME/wrong PATH and wrong HOME/pinned PATH pass as intended; cache hit/miss/corruption controls cannot be rescued by the host. |
| CI6 real safe diagnostics | Run actual verifier preflight through existing capped-output capture. Add optional invocation-owned spill directory shared by CI producer/collector, preserving local /tmp default. Exclude empty/symlink and foreign-directory logs. Keep primary verify failure distinct from secondary collection/upload errors. | Missing-tool preflight exits nonzero with genuine nonempty redacted evidence; long failure keeps full diagnostics and exit/count/time. Empty/symlink/foreign-only inputs cannot become proof. Existing collect-all and default local behavior stay intact. |
| CI7 manual routing | Validate lowercase full candidate SHA, release ref and equality to dispatched SHA early in hosted changes, before checkout/classification. Expose validated qualification output. Successful manual qualification bypasses diff/lite selection for all eight drivers; manual mode is normal. Retain native precheckout guard and cancellation semantics. | Existing pinned-matcher routing test covers empty-diff/already-merged/manual-lite, malformed/mismatched SHA/nonrelease ref, classifier failure after valid input and cancellation. Invalid manual validation cannot launch fast/heavy qualification; genuine classifier failure retains fast diagnostics. Automatic docs/platform/trusted-lite behavior stays proportional. |

## Qualification and evidence

After Astra approves implementation and focused checks, freeze executable/source
inputs and commit. Run one broad local verifier and feature-complete round with
live prerequisites, named scratch and explicit resource budget; on failures run
only affected scopes after repair. Parent owns shared release-state edits and
remote gh dispatch; implementation is isolated in one disposable worktree.

Use the existing feature/target matrix and reviewed allowlists, strict live
security, accepted long/performance/EU7 routes, five native-target receipts plus
distinct Windows WAL proof, source-archive Rust consumer and no-private-hook
shipped artifacts. Record real outcomes, unavailable/queued blockers and exact
inputs. Time performance only after competing builds stop. Source Gitleaks and
retained generated-evidence scans are separate, pinned and advisory as before.

Local dry-run and immutable release-workflow dry-run get separate dispositions.
If .28 version/Axis E/provenance preparation remains unauthorized, record that
prerequisite and pending rehearsal; do not substitute an old-version rehearsal.
No tag, registry/Pages publication or publish workflow with dry_run=false occurs.

The final [status](status.md) maps S95-1–11 to commands, artifacts, source/input
identities, reviews and actual results. Report assembled/resumed evidence and
readiness blockers honestly. Forward final verification fixes under the existing
.29 hand-over rule, then clean up only the task implementation worktree/branch.
