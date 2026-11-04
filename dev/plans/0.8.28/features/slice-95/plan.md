---
title: FathomDB 0.8.28 Slice 95 — CI completeness and final release readiness
status: PLANNED
target_release: 0.8.28
planning_baseline: 8aa724ba0a1f6c7030fd43e85d936d8ddb1f2272
reconciliation_baseline: 48b23c479826c647affde9e9a1b6c69ee8f37f4c
reconciled_on: 2026-11-04
---

# Slice 95 — CI completeness and final release readiness

Owner placement on 2026-11-04 (steward ledger seq-364): after completed Slice 90,
close the release's CI correctness/completeness gaps and perform bounded final
candidate qualification. This is the release-closing engineering slice;
publication remains a separate owner decision. Creation of this plan performs
no CI repair, qualification run, version cut or publication. Astra reviewed this
initial planning scope and acceptance. The comparative CI/release review below
strengthens that scope; implementation and new qualification remain unstarted.

## Dependency and documentation handoff

Owner placement of [Slice 92](../slice-92/plan.md) on 2026-11-04 (ledger
seq-365) inserts documentation audit/convergence after 90 and before this slice.
Consume its reviewed user/contributor/project documentation baseline and example
checks. S95-5 remains final semantic/structural consistency verification and any
correction caused by subsequent CI/package repairs; do not repeat the full audit.
The CI repair and qualification scope below is retained.

## Historical model and reconciliation

The model is the documentation/contract convergence in
[0.8.27 Slice 140](../../../0.8.27/features/slice-140/plan.md), integrated
qualification in [0.8.27 Slice 150](../../../0.8.27/features/slice-150/plan.md),
and [0.8.25 final verification/CI/non-publishing rehearsal](../../../0.8.25/features/slice-85/plan.md).
Reuse their candidate binding, installed-consumer checks, truthful non-pass
accounting and narrow reruns. Do not copy obsolete test totals, retired temporary
oracles, historical exceptions, model requirements or approval handshakes.

### Lessons retained from 0.8.26 and 0.8.27

| Historical evidence | Explicit Slice 95 obligation |
| --- | --- |
| [0.8.26 Slice 50 platform/package design](../../../0.8.26/features/slice-50/design.md) and [its failed first candidate](../../../0.8.26/features/slice-50/status.md) exposed a debug-only hook in a shipped-artifact test. | Qualify the current five supported CPU wheel/addon targets with the existing receipt collector, keep installed Windows WAL evidence separate, and prove shipped tests do not depend on private hooks. |
| The same Slice 50 design distinguishes tracked-tree and generated-evidence Gitleaks scans. | Scan both scopes separately with the pinned tools; tracked-source scanning cannot stand in for checking generated receipts/logs. Preserve the existing advisory policy and report actual findings. |
| [0.8.26 Slice 65](../../../0.8.26/features/slice-65/plan.md) separates catalog validation from semantic correctness. | Validate current-owner authority and lifecycle metadata, then review maintained contract claims against code; catalog success does not prove prose current. |
| [0.8.27 Slice 150 design](../../../0.8.27/features/slice-150/design.md) retains permanent feature-complete/test-target gates, external Rust consumers and strict live security. | Require every non-exempt current target with live prerequisites, an external candidate source-archive Rust consumer, and actual AC-037 live proof; do not substitute default-only or all-features compilation. |
| [0.8.26 release closeout](../../../runs/STATUS-0.8.26.md) and [0.8.27 publication preparation](../../../plan-0.8.27.md) distinguish ordinary CI from release-workflow rehearsal. | Name local dry-run and immutable-candidate Release dry-run separately, with unmet ancestry/version/authorization prerequisites recorded explicitly. Ordinary CI cannot qualify the release workflow. |
| Surviving CI routing, tier, observability and run-hygiene controls encode earlier false-green and slow-failure incidents. | Exercise those existing controls against the repaired workflow: proportional routing, failed classifier/cancellation behavior, collect-all exit status, complete failure diagnostics and resource-bounded caches. |

The temporary hidden-surface oracle was retired in 0.8.27 Slice 150; retain its
permanent feature/target checks, not its deleted tools or baselines. Do not copy
0.8.26 schema fixtures, fixed test/operation totals, its accelerator exclusion,
the 100 GB temporary-oracle prerequisite or old retention surveys. Current
0.8.28 capabilities and explicit owner deferrals decide applicability.

| Current fact or allocated item | Slice 95 disposition |
| --- | --- |
| Slice 90 completed D28-14 at reviewed source `9b6fc8b6f` plus approved signature delta `14d710093`; local closure is `8aa724ba0`. | Treat product implementation as complete. Reconcile its assembled local evidence; do not implement more retrieval features. |
| Slice 90 changed no CI workflow, bootstrap or parallel-runner configuration and explicitly made no hosted-CI green claim. | A local pass does not resolve the cold hosted setup or parallel report failures. Both are assigned below. |
| CI run [41885604931](https://github.com/fathomadb/fathomdb/actions/runs/41885604931) used earlier code `a2fb4a647`, not the Slice 90 candidate. | Inspect current job outcomes at execution; retain this run as failure evidence, never as final-candidate qualification. |
| The heavy verify job invokes `bootstrap-heavy.sh` then the full verifier's heavy tier. Its preflight lacked Actionlint 1.7.12, Ruff 0.15.17, Pyright 1.1.410 and repo-root js-yaml. | S95-CI1: align cold setup with the actual command's pinned prerequisites and executable paths. Keep strict preflight. |
| The parallel Rust report encountered linker BusErrors and exhausted hosted disk before producing test evidence. | S95-CI2: bound and verify build/cache/scratch consumption while preserving the diagnostic parallel exercise. A setup failure is not a race-test result. |
| Native-artifact smokes assumed GraphEvidenceResolveRequestV1 was inline in index.d.ts although the package already re-exported modular declarations before Slice 85. | S95-CI3: repair this known consumer-check defect and qualify the packaged exported type and runtime, preserving sensitivity to an actually absent export. |
| The cited run's completed fast tier reports 195/198 suites passing: `test-check-single-cudarc`, `test-agent-lint-shellcheck-version` and `test-install-shellcheck` failed. Cargo output contains ANSI-colored `(*)`; ShellCheck fixture outcomes depend on an installed host binary. | S95-CI4/CI5: reproduce the CI environment and make dependency-output handling and tool-lookup fixtures deterministic without weakening actual version/source/duplicate-dependency checks. |
| Heavy verification failed preflight before creating a spill log; collection and upload then failed on missing files. The parallel report returned 101 and uploaded its log, then its cache post-step failed with disk exhaustion. | S95-CI6 and S95-CI2: retain the primary failure and useful preflight diagnostics, distinguish secondary collection/cache failures, and budget post-job peak space as well as compilation. |
| At this review, all four non-Windows native-artifact smoke legs have failed; the Windows native-artifact leg remains queued, while the separate Windows WAL job succeeded on the earlier SHA. | Re-inspect via gh at execution. Queued Windows and the missing aggregate matrix are not passes; the older WAL result cannot qualify the final candidate. |
| A read-only evaluation of current job predicates with successful manual empty-diff outputs (docs_only=false, all path categories=false, ci_mode=normal) selects only verify-fast among that verifier and the eight scoped drivers, despite the workflow comment promising full manual qualification. This is a static routing counterexample, not a dispatched CI result. | S95-CI7: make valid exact-candidate manual qualification independent of branch-diff/lite shortcuts. Extend the existing executable routing guard to assert the required selection for empty-diff and already-merged candidates. |
| Existing 0.8.28 scope retains final performance, security, feature and installed-artifact qualification; owner rulings defer D28-09/10/11 and parts of D28-12. | Build one applicable evidence matrix from current contracts and rulings. Preserve deferrals; do not import earlier release obligations wholesale. |
| Owner considers physical erasure done except for defects and says it must no longer dominate release planning. | No new erasure feature, optimization, research or latency campaign. Exercise existing applicable regression coverage as ordinary release verification; repair only concrete defects. |

These CI facts were read through `gh run view` and completed-job log APIs on
2026-11-04, including [fast verifier job](https://github.com/fathomadb/fathomdb/actions/runs/41885604931/job/126769296585),
[heavy verifier job](https://github.com/fathomadb/fathomdb/actions/runs/41885604931/job/126769296857)
and [parallel report job](https://github.com/fathomadb/fathomdb/actions/runs/41885604931/job/126769296892).
They are a failure baseline, not proof of repairs or the current candidate.

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
| S95-2 cold verify setup | S95-CI1 is fixed: a clean hosted setup supplies every pinned preflight dependency, including the correct checkout-owned Python tools, repo-root JS tooling and Actionlint. The exact intended verify command passes preflight and actually runs its assigned checks. Negative controls still reject missing/wrong tools; no preflight bypass or weakened pin. Provision prerequisites before expensive compilation. Exercise cold and restored-cache setups. Repair CI4/CI5 presentation/fixture failures with focused RED/GREEN; accepting a correctly installed host tool must not break fixture isolation or suppress a genuine wrong-version failure. |
| S95-3 bounded parallel report | S95-CI2 is fixed: record initial free bytes, build/cache/scratch policy, available memory and build concurrency before the run; select a bounded treatment from existing CI facilities. The intended parallel tests reach execution and retain actual exit/count/log evidence without disk exhaustion. Preserve the report's advisory scheduling semantics; do not serialize away its purpose or call a green wrapper a passing test run. Measure free-space transitions through cache restore, build, test, artifact upload and cache post-processing. Reserve measured peak headroom; cache/upload failures remain distinct from the actual parallel test exit/counts. |
| S95-4 installed consumer quality | S95-CI3 and other concrete package-route defects are resolved. Fresh candidate Python wheels, Node packages/addons and the supported Rust consumer route work outside source checkouts. Prove exported modular TypeScript types via a real consumer compilation and execute native calls; a missing-export control fails. Bind source, package/native hashes, dependency resolution, toolchain/runtime and flags. Use the existing five-target receipt collector plus the distinct Windows WAL receipt, and reject private-hook dependencies in shipped-artifact tests. The external Rust source archive includes manifests, lockfile, Rust sources and active patches; record consumer lockfile and resolved paths. This is not registry-resolved .crate proof. No stale installed artifact or unpublished dependency substitution is a pass. |
| S95-5 docs and contracts | Consume Slice 92's completed documentation baseline; check later repair deltas and final consistency. As-built interfaces, generated operation docs, examples, manifests, supported runtime/platform claims and release notes agree. Run existing surface/parity/feature/test-target comparators and documentation validators. Preserve frozen historical baselines and declare only approved deltas. Run lifecycle/current-owner authority checks and code-grounded semantic review separately. Correct stale references or claims; do not start a new structural refactor or oracle framework. |
| S95-6 final candidate qualification | One clean reviewed candidate has passing applicable canonical local, feature, security, performance and platform/installed-artifact evidence under the matrix below, or an existing explicit owner-authorized non-pass disposition. Record actual counts, skips, commands, exits, inputs and artifact identity. Permanent feature-complete and target-coverage checks account for every non-exempt target with live prerequisites and reviewed skip/ignored allowlists; unavailable assets/platforms are not implicit exemptions. Missing required evidence remains a named readiness blocker; earlier-slice receipts are reused only with an explicit relevant-input comparison. |
| S95-7 hosted CI result | Use gh to dispatch/inspect the exact reviewed candidate. Every applicable job is accounted for: success, actual failure, intentional diagnostic/non-applicable, or explicit unavailable/queued blocker. Inspect failed logs/artifacts and false-green paths. Do not treat cancelled/queued/skipped or zero-test results as passes, or silently invent a waiver. No branch protection, merge queue or soak-period policy is added. |
| S95-8 closure and hand-over | Independent Astra review and separate verification find no unresolved material defect in changes/evidence. Status records readiness, residual risks, source/artifact identities, exact broad/resumed provenance, and remaining owner decisions. Update release JSON and regenerate its views; land and clean up task-only worktrees/branches. Keep publication authorization separate. Preserve the existing 0.8.29 hand-over rule at final verification and forward later verification fixes as already directed. |
| S95-9 CI routing and harness robustness | Existing executable routing/tier/hygiene controls pass the cases below, with real pinned matcher behavior. Fast/heavy registrations are total and disjoint, and a failing registered suite keeps its nonzero verdict. No retries, quarantine, broadened exclusions or changed diagnostic scheduling disguise a failure. |
| S95-10 diagnosable and safe evidence | A forced preflight failure before the first suite retains useful real diagnostics; a truncated later failure retains full redacted spill evidence and its actual exit/count/time data. Empty/symlinked/foreign logs are not uploadable proof. Tracked-tree and generated retained-evidence scans have separate pinned-tool outcomes; generated secrets are corrected/redacted before retention, with no broad new suppression or historical-debt campaign. Existing advisory Gitleaks job policy is unchanged. |
| S95-11 release rehearsal truth | Ordinary CI, local release dry-run and immutable-candidate release.yml dry-run have separate evidence/dispositions. Validate version/Axis E/co-tagging, trusted workflow/runner provenance and the non-publishing route before expensive work. If main ancestry/version preparation needs separate authority, record that prerequisite and the rehearsal as pending; do not bypass checks or claim pre-tag readiness. Publication and registry-installed proofs remain separate. |

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
5. S95-CI4: reproduce `CARGO_TERM_COLOR=always` dependency-tree output. Normalize
   presentation at the existing command/parser boundary; retain rejection of a
   real extra cudarc, wrong version/source or forbidden CUDA feature. The
   observed ANSI `(*)` suffix is not a dependency-policy exception.
6. S95-CI5: isolate ShellCheck fixture lookup/cache paths from whichever correct
   binary the runner image already has. Keep positive installed/cache controls
   and decisive wrong/missing-version controls; do not accept arbitrary failure
   as proof the intended version guard ran.
7. S95-CI6: preserve a real preflight log before any suite creates a spill file,
   and preserve the original failure if collection, upload or cache cleanup also
   fails. Do not weaken the empty-artifact or redaction/symlink protections.
8. S95-CI7: add the successful manual empty-diff classification as RED in the
   existing routing guard. Setting docs_only=false alone does not select the
   scoped jobs when every path category is false. Make validated candidate
   dispatch select the applicable full qualification regardless of diff/lite
   state, preserving existing immutable-candidate and trusted-runner checks.
   Retain cheap automatic docs/platform scoping and cancellation semantics.
9. Examine the rest of the CI inventory for real coverage gaps, duplicate work,
   failure masking, missing artifacts and unsupported claims. Fix only bounded
   defects necessary for this release; record unrelated improvements separately.

### CI regression controls to preserve

Reuse existing machinery, adding a focused case only where a distinct repair
needs it. This is a behavioral audit, not a second CI evaluator or a new gate.

| Boundary | Decisive cases and existing control |
| --- | --- |
| Routing and cost | Markdown-only, Rust/Python/TS, Windows-only, workflow/harness/manifest edits, manual candidate dispatch with empty/no branch diff, classifier failure and cancellation. Use `scripts/tests/test_ci_proportional_routing.py` and the pinned matcher fixture, plus existing paths-filter tests. Preserve the intended red classifier/fast diagnostic behavior; do not force all heavy jobs after classifier failure. Trusted ci-lite cannot suppress release qualification or be selected by an untrusted PR. |
| Tier and process verdict | `scripts/tests/test_agent_test_tiers.sh` and collect-all tests prove complete/disjoint registration and propagate early/middle/late suite failures. Preserve shell pipefail, PowerShell external exit status, zero-selected-test refusal, counts and reviewed skips. No fixed historical suite total becomes an oracle. |
| Scheduling | Existing CI run-hygiene tests preserve superseded-PR cancellation and independent non-PR attempts; avoid always-running job conditions that defeat cancellation. A cancelled or queued platform remains non-evidence. |
| Diagnostics and runner environment | `scripts/tests/test_ci_verify_observability.sh`, `test_ci_long_job_efficiency.sh`, pinned-tool tests and the observed CI4/CI5/CI6 reproducers cover green-run timing, early setup failure, long/truncated diagnostics, safe collection, and separate race/cache outcomes. Exercise the relevant cold/cache-restored paths without sweeping unrelated caches. |
| Installed artifacts and trust | `scripts/tests/test_native_artifact_receipts.py` and native-runtime tests retain missing/duplicate/cross-candidate rejection. `scripts/release/check-ts-private-test-hooks.mjs` and `scripts/release/smoke/slice73-windows-napi-modules.json` keep shipped tests independent of disposable private hooks. Existing release-gate, publish-safety and CUDA provenance tests keep exact-candidate rehearsals on the trusted non-publishing route. |

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
| Local correctness and coverage | Full `scripts/agent-verify.sh`, applicable `scripts/test-feature-complete.sh` and declared feature/test-target routes; all-workspace check/Clippy, release/default/operator/test-hooks routes as applicable. `scripts/test-feature-matrix.toml`, `scripts/test-skip-allowlist.toml` and `scripts/check-test-target-coverage.py` bind permanent feature coverage to the current source; require live prerequisites for non-exempt targets and avoid incompatible all-features builds. Deduplicate commands already positively executed by canonical gates. Use a checkout-owned environment and a ptrace-capable executor for strict checks. |
| Security and documentation | Strict security with actual AC-037 live netns and live catch lines and zero violations/blockers/downgrades per the existing runbook; retain capability/grant/revert evidence only when needed. Scan tracked source and generated retained evidence separately with pinned Gitleaks; preserve advisory policy and report findings. Validate lifecycle/current-owner authority and review maintained claims against code; surface/removal/docs checks remain. Unavailable capabilities are not live passes. |
| Accepted performance and reliability | Reconcile `AGENT_LONG` checks, eu7 and relevant 0.8.28 acceptance against the current release plan, accepted replacements and owner dispositions before timing. Use existing thresholds, inputs and runners; no new competitor or physical-erasure optimization campaign. Existing owner deferrals remain intact. |
| Candidate packages/platforms | Reconcile `dev/platform-capabilities.json` with current delivery contracts: five supported CPU native wheel/addon targets (Linux x64/ARM64, macOS x64/ARM64, Windows x64), collected by `scripts/release/native-artifact-receipts.py`; distinct exact-candidate Windows WAL receipt; shipped vs disposable-hook controls; external Rust source-archive consumer with sibling/patch resolution. Applicable CPU/driverless and warranted GPU/Tegra routes retain separate rows, exact native registration and close/exit. Keep supported runtime pins and existing CUDA policy; no driver/toolkit change or claim for an unavailable host. Version-dependent dependent-crate packaging boundaries must be disclosed instead of substituting older registry code. |
| Non-publishing release rehearsal | `scripts/release/local-dry-run.sh` and separate `release.yml` workflow_dispatch with dry_run=true, release_version and immutable candidate_commit. Keep `scripts/verify-release-gates.sh` ancestry/version/Axis E checks and trusted dispatch/runner restrictions. Record precisely what the leaf/package rehearsal can prove; unpublished sibling registry resolution and post-publish consumption remain separate. If a prerequisite is unauthorized/unmet, record pending and do not infer release-workflow or pre-tag readiness from ordinary CI. |
| Hosted CI and release readiness | Exact-candidate workflow/job evidence through gh, job coverage and raw failure/artifact inspection. Separate product, runner, diagnostic and unavailable results. Report a readiness verdict rather than add mandatory GitHub status checks or automatic release authorization. |

Use the existing [AC-037 live runbook](../../../../release/ac-037-live-netns-hitl-runbook.md)
and [current-owner authority catalog](../../../../design/current-owner-authority.json).
The non-publishing rehearsal is planned here, but neither this amendment nor a
green CI run authorizes the version cut/main integration needed by its release
entry checks. Prepare concrete inputs and retain a named pending prerequisite
until that authority exists. Report CI repair, candidate qualification and
pre-tag rehearsal readiness distinctly; none authorizes publication.

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
