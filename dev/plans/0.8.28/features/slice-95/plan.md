---
title: FathomDB 0.8.28 Slice 95 — CI completeness and final release readiness
status: PLANNED
target_release: 0.8.28
planning_baseline: 8aa724ba0a1f6c7030fd43e85d936d8ddb1f2272
---

# Slice 95 — CI completeness and final release readiness

Owner placement on 2026-11-04 (steward ledger seq-364): after completed Slice 90,
close the release's CI correctness/completeness gaps and perform bounded final
candidate qualification. This is the release-closing engineering slice;
publication remains a separate owner decision. Creation of this plan performs
no CI repair, qualification run, version cut or publication. Astra reviewed this
planning scope and acceptance: APPROVE, no remaining findings.

## Historical model and reconciliation

The model is the documentation/contract convergence in
[0.8.27 Slice 140](../../../0.8.27/features/slice-140/plan.md), integrated
qualification in [0.8.27 Slice 150](../../../0.8.27/features/slice-150/plan.md),
and [0.8.25 final verification/CI/non-publishing rehearsal](../../../0.8.25/features/slice-85/plan.md).
Reuse their candidate binding, installed-consumer checks, truthful non-pass
accounting and narrow reruns. Do not copy obsolete test totals, retired temporary
oracles, historical exceptions, model requirements or approval handshakes.

| Current fact or allocated item | Slice 95 disposition |
| --- | --- |
| Slice 90 completed D28-14 at reviewed source `9b6fc8b6f` plus approved signature delta `14d710093`; local closure is `8aa724ba0`. | Treat product implementation as complete. Reconcile its assembled local evidence; do not implement more retrieval features. |
| Slice 90 changed no CI workflow, bootstrap or parallel-runner configuration and explicitly made no hosted-CI green claim. | A local pass does not resolve the cold hosted setup or parallel report failures. Both are assigned below. |
| CI run [41885604931](https://github.com/fathomadb/fathomdb/actions/runs/41885604931) used earlier code `a2fb4a647`, not the Slice 90 candidate. | Inspect current job outcomes at execution; retain this run as failure evidence, never as final-candidate qualification. |
| The heavy verify job invokes `bootstrap-heavy.sh` then the full verifier's heavy tier. Its preflight lacked Actionlint 1.7.12, Ruff 0.15.17, Pyright 1.1.410 and repo-root js-yaml. | S95-CI1: align cold setup with the actual command's pinned prerequisites and executable paths. Keep strict preflight. |
| The parallel Rust report encountered linker BusErrors and exhausted hosted disk before producing test evidence. | S95-CI2: bound and verify build/cache/scratch consumption while preserving the diagnostic parallel exercise. A setup failure is not a race-test result. |
| Native-artifact smokes assumed GraphEvidenceResolveRequestV1 was inline in index.d.ts although the package already re-exported modular declarations before Slice 85. | S95-CI3: repair this known consumer-check defect and qualify the packaged exported type and runtime, preserving sensitivity to an actually absent export. |
| Existing 0.8.28 scope retains final performance, security, feature and installed-artifact qualification; owner rulings defer D28-09/10/11 and parts of D28-12. | Build one applicable evidence matrix from current contracts and rulings. Preserve deferrals; do not import earlier release obligations wholesale. |
| Owner considers physical erasure done except for defects and says it must no longer dominate release planning. | No new erasure feature, optimization, research or latency campaign. Exercise existing applicable regression coverage as ordinary release verification; repair only concrete defects. |

Before execution refresh ancestry, checkout ownership, current CI history, assigned
follow-ups and tool/platform availability. Enumerate changes since this baseline
and adjust only affected obligations. Use the existing development checkout if
it has one writer; isolate concurrent writers and remove any task worktree after
landing. Do not adopt Steward/Orchestrator roles.

## Needs, requirements and local acceptance

The release owner needs a reliable account of what CI actually checks and one
candidate-bound release-readiness verdict. S95 labels are local acceptance;
`dev/acceptance.md` and accepted thresholds remain unchanged.

| ID | Required outcome and decisive acceptance |
| --- | --- |
| S95-1 coverage and truth | An inventory maps applicable workflows/jobs and local release checks to their current requirement, platform/runtime/features, command, disposition and evidence. Explain path filters, fast/heavy coverage, duplicate work, allowed skips and diagnostic jobs. Required checks cannot disappear through filtering, zero tests, unavailable tools, masked failures or artifact fallback. Intentional diagnostics remain explicitly diagnostic. |
| S95-2 cold verify setup | S95-CI1 is fixed: a clean hosted setup supplies every pinned preflight dependency, including the correct checkout-owned Python tools, repo-root JS tooling and Actionlint. The exact intended verify command passes preflight and actually runs its assigned checks. Negative controls still reject missing/wrong tools; no preflight bypass or weakened pin. Provision prerequisites before expensive compilation. |
| S95-3 bounded parallel report | S95-CI2 is fixed: record initial free bytes, build/cache/scratch policy, available memory and build concurrency before the run; select a bounded treatment from existing CI facilities. The intended parallel tests reach execution and retain actual exit/count/log evidence without disk exhaustion. Preserve the report's advisory scheduling semantics; do not serialize away its purpose or call a green wrapper a passing test run. Cache/upload cleanup must not obscure primary failures. |
| S95-4 installed consumer quality | S95-CI3 and other concrete package-route defects are resolved. Fresh candidate Python wheels, Node packages/addons and the supported Rust consumer route work outside source checkouts. Prove exported modular TypeScript types via a real consumer compilation and execute native calls; a missing-export control fails. Bind source, package/native hashes, dependency resolution, toolchain/runtime and flags. No stale installed artifact or unpublished dependency substitution is a pass. |
| S95-5 docs and contracts | As-built interfaces, generated operation docs, examples, manifests, supported runtime/platform claims and release notes agree. Run existing surface/parity/feature/test-target comparators and documentation validators. Preserve frozen historical baselines and declare only approved deltas. Correct stale references or claims; do not start a new structural refactor or oracle framework. |
| S95-6 final candidate qualification | One clean reviewed candidate has passing applicable canonical local, feature, security, performance and platform/installed-artifact evidence under the matrix below, or an existing explicit owner-authorized non-pass disposition. Record actual counts, skips, commands, exits, inputs and artifact identity. Missing required evidence remains a named readiness blocker; earlier-slice receipts are reused only with an explicit relevant-input comparison. |
| S95-7 hosted CI result | Use gh to dispatch/inspect the exact reviewed candidate. Every applicable job is accounted for: success, actual failure, intentional diagnostic/non-applicable, or explicit unavailable/queued blocker. Inspect failed logs/artifacts and false-green paths. Do not treat cancelled/queued/skipped or zero-test results as passes, or silently invent a waiver. No branch protection, merge queue or soak-period policy is added. |
| S95-8 closure and hand-over | Independent Astra review and separate verification find no unresolved material defect in changes/evidence. Status records readiness, residual risks, source/artifact identities, exact broad/resumed provenance, and remaining owner decisions. Update release JSON and regenerate its views; land and clean up task-only worktrees/branches. Keep publication authorization separate. Preserve the existing 0.8.29 hand-over rule at final verification and forward later verification fixes as already directed. |

## Bounded design and repair sequence

### A. Converge CI and consumer checks

1. Audit actual job commands and setup rather than job names. Reuse the canonical
   pinned installers, environment binding and existing runners; avoid a second
   job-specific bootstrap or a new orchestration framework. Draft the exact
   YAML/script/resource changes and have Astra review before implementation.
2. S95-CI1: reconcile heavy bootstrap with verifier prerequisites. Keep clean
   checkout/venv ownership, proper PATH resolution and early actionable failures.
   Validate workflow schema with actionlint, not YAML parsing alone.
3. S95-CI2: measure the runner's actual space and choose bounded build/debug,
   concurrency, cache or scratch changes based on that evidence. Preserve test
   concurrency and semantics. Any cleanup is limited to explicitly task-owned
   ephemeral runner data; do not delete shared caches, live evidence or host CUDA
   installations. Resource failure remains explicit until the route succeeds.
4. S95-CI3: follow the package export graph instead of demanding an obsolete
   declaration layout. Test the installed consumer route and retain a controlled
   absent/wrong-export failure. Never make the smoke pass by skipping its check.
5. Examine the rest of the CI inventory for real coverage gaps, duplicate work,
   failure masking, missing artifacts and unsupported claims. Fix only bounded
   defects necessary for this release; record unrelated improvements separately.

Use the observed failing jobs as RED evidence, supplemented by focused failing
behavioral tests/reproducers where needed. Stage/commit RED before GREEN for each
new behavior. Keep substantive test oracles read-only during fixes; approved
contract/inventory updates remain explicit. Review implementation with Astra.
This planning slice adds no new product API, schema or erasure behavior.

### B. Freeze and qualify the final candidate

After repairs and scoped checks, freeze source and executable gate inputs. Use
existing runners and record a compact matrix rather than build a new manifest
validator, scheduler or general test framework.

| Obligation family | Existing authority and execution |
| --- | --- |
| Local correctness and coverage | Full `scripts/agent-verify.sh`, applicable `scripts/test-feature-complete.sh` and declared feature/test-target routes; all-workspace check/Clippy, release/default/operator/test-hooks routes as applicable. Deduplicate commands already positively executed by canonical gates. Use a checkout-owned environment and a ptrace-capable executor for strict checks. |
| Security and documentation | Existing strict security/live host requirements and source-bound receipts; public/internal docs, generated content, examples, surface and removal-policy checks. Unavailable host capabilities are not live passes. |
| Accepted performance and reliability | Reconcile `AGENT_LONG` checks, eu7 and relevant 0.8.28 acceptance against the current release plan, accepted replacements and owner dispositions before timing. Use existing thresholds, inputs and runners; no new competitor or physical-erasure optimization campaign. Existing owner deferrals remain intact. |
| Candidate packages/platforms | Fresh supported Rust/Python/Node consumer smokes, applicable CPU/driverless and warranted GPU/Tegra/platform routes, exact native registration and close/exit. Keep supported runtime pins and existing CUDA policy; no driver/toolkit change or claim for an unavailable host. Version-dependent dependent-crate packaging boundaries must be disclosed instead of substituting older registry code. |
| Hosted CI and release readiness | Exact-candidate workflow/job evidence through gh, job coverage and raw failure/artifact inspection. Separate product, runner, diagnostic and unavailable results. Report a readiness verdict rather than add mandatory GitHub status checks or automatic release authorization. |

At execution, bind each row to its actual required scope, executor, inputs,
command, acceptance and available evidence before costly work. This is the
existing final release qualification, not a reopened feature backlog. The
[quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md) applies
through composition/boundary checks, clean installed consumers, resource cleanup
and a few failure-sensitivity controls. Defer global MC/DC/OOM/VFS sweeps, new
fuzz frameworks and unrelated performance work. New evidence is the recurring
cold-setup, disk and obsolete-smoke failures despite earlier local slice passes.

Run one final broad local round after the code phase is ready. Stop on a genuine
failure, preserve it, fix with RED/GREEN and rerun only failed/invalidated checks;
if collection continues by canonical design, consume its results rather than
restart it. A broad source change requires an explicit blast-radius reassessment.
Run timing only after builds/tests stop. No skip/exclusion, baseline rewrite,
retry or changed threshold may silently turn a non-pass into a pass.

### C. Review, record and stop

Astra reviews design/changes; an independent verifier checks the final matrix and
actual installed/hosted executions. Record assembled/resumed qualification
honestly. Later records-only commits prove tested source and executable inputs
unchanged; code or gate-input fixes invalidate only affected evidence.

Write [status](status.md), update the release state and regenerate views. Keep
status concise with exact commands/evidence locators and outstanding blockers.
Physical erasure is complete except for defects. No version cut, v* tag, registry
or Pages publication, main merge, paid/GPU campaign beyond existing authorization,
or post-publication smoke is authorized by creating Slice 95. Publication and its
registry-installed end-to-end proof follow the separate owner ruling and release
runbook; do not import historical oracle-retirement work already completed.
