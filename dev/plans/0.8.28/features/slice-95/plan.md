---
title: FathomDB 0.8.28 Slice 95 — CI completeness and final release readiness
status: IN_PROGRESS
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
strengthens that scope. Execution reconciliation and the concrete repair design
are recorded in [design](design.md); implementation and qualification now proceed
under that bounded design.

## Dependency and documentation handoff

Owner placement of [Slice 92](../slice-92/plan.md) on 2026-11-04 (ledger
seq-365) inserts documentation audit/convergence after 90 and before this slice.
Consume its reviewed user/contributor/project documentation baseline and example
checks. S95-5 remains final semantic/structural consistency verification and any
correction caused by subsequent CI/package repairs; do not repeat the full audit.
The CI repair and qualification scope below is retained.

Slice92 is complete at `7b24e4c0913ec6585781b1148b81da0853f9dd67`; its
[status](../slice-92/status.md) records the reviewed baseline, real consumer
examples and assembled verification (196/205 initially passed; nine affected
suites passed scoped reruns). Current documentation targets pending .28, not a
publication receipt. Preserve the required generated plan/board pointers and
refresh only documentation invalidated by later repairs. Final CI/platform/
feature/performance/security/rehearsal obligations below remain assigned here.

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

The execution [qualification matrix](qualification.md) binds each applicable
route to current acceptance, resources and disposition. It preserves accepted
TC5 recall and separately assessed stress rather than reviving the superseded
EU7 recall oracle. Astra approved this applicability reconciliation.

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
| S95-6 final candidate qualification | One clean reviewed candidate has passing applicable canonical local, feature, security, performance and platform/installed-artifact evidence under the matrix below, or an existing explicit owner-authorized non-pass disposition. Record actual counts, skips, commands, exits, inputs and artifact identity. Permanent feature-complete and target-coverage checks account for every non-exempt target with live prerequisites and reviewed skip/ignored allowlists; unavailable assets/platforms are not implicit exemptions. Long cursor evidence must witness successful writes during reads, propagated writer errors, positive matching rows and advancing hit cursors bounded by the response cursor in an explicit provider-free synchronous lexical fixture. Missing required evidence remains a named readiness blocker; earlier-slice receipts are reused only with an explicit relevant-input comparison. |
| S95-7 hosted CI result | Use gh to dispatch/inspect the exact reviewed candidate. Every applicable job is accounted for: success, actual failure, intentional diagnostic/non-applicable, or explicit unavailable/queued blocker. Inspect failed logs/artifacts and false-green paths. Do not treat cancelled/queued/skipped or zero-test results as passes, or silently invent a waiver. No branch protection, merge queue or soak-period policy is added. |
| S95-8 closure and hand-over | Independent Astra review and separate verification find no unresolved material defect in changes/evidence. Status records readiness, residual risks, source/artifact identities, exact broad/resumed provenance, and remaining owner decisions. Update release JSON and regenerate its views; land and clean up task-only worktrees/branches. Keep publication authorization separate. Preserve the existing 0.8.29 hand-over rule at final verification and forward later verification fixes as already directed. |
| S95-9 CI routing and harness robustness | Existing executable routing/tier/hygiene controls pass the cases below, with real pinned matcher behavior. Fast/heavy registrations are total and disjoint, and a failing registered suite keeps its nonzero verdict. No retries, quarantine, broadened exclusions or changed diagnostic scheduling disguise a failure. |
| S95-10 diagnosable and safe evidence | A forced preflight failure before the first suite retains useful real diagnostics; a truncated later failure retains full redacted spill evidence and its actual exit/count/time data. Empty/symlinked/foreign logs are not uploadable proof. Tracked-tree and generated retained-evidence scans have separate pinned-tool outcomes; generated secrets are corrected/redacted before retention, with no broad new suppression or historical-debt campaign. Existing advisory Gitleaks job policy is unchanged. |
| S95-11 release rehearsal truth | Ordinary CI, local release dry-run and immutable-candidate release.yml dry-run have separate evidence/dispositions. Validate version/Axis E/co-tagging, trusted workflow/runner provenance and the non-publishing route before expensive work. If version/Axis E or trusted workflow preparation needs separate authority, record that prerequisite and the rehearsal as pending; do not bypass checks or claim pre-tag readiness. Publication and registry-installed proofs remain separate. |

### Canonical CI finding allocated during execution

The first manual candidate reached real Rust execution and exposed CI8: an
Autonomous worker reserves physical service on primary contention before
confirming committed debt. This is a bounded correctness defect under the
existing contract, not a reopened erasure feature. The [design amendment](design.md#ci8-amendment-confirmed-debt-worker-admission)
records the observed failures, lock-order constraints and deterministic RED/GREEN.

| ID | Additional decisive acceptance |
| --- | --- |
| S95-12 confirmed-debt admission | A debt-free contended primary cannot make an idle Autonomous poller occupy physical service and refuse fresh foreground erasure. Deterministic real-DB admission/completion coverage fails before the fix and passes after; true debt retains priority/progress and a genuine competing service retains typed refusal. The three original hosted failures pass unchanged. No retries or serialization of these three fixtures/workspace, weaker success assertions, new policy or deadline. |

### Python subscriber finding allocated during final qualification

Exact candidatebef17bfab completed fast198/198 and all other applicable jobs,
but its heavy Python suite failed the unchanged real-write phase-pairing test.
The accepted subscriber ADR guarantees delivery below capacity; producer
try_lock contention incorrectly loses records before capacity exhaustion. The
[CI12 design](design.md#ci12-preserve-subscriber-admission-below-capacity)
preserves the public contract and existing assertions through a bounded standard
channel, deterministic native RED/GREEN and fresh installed-wheel verification.

| ID | Additional decisive acceptance |
| --- | --- |
| S95-13 subscriber admission | With spare capacity and a live functioning logger, consumer bookkeeping cannot discard Started/Finished or reorder their FIFO delivery. Only true capacity exhaustion counts loss. Capacity4096, warning fairness, nonblocking producers, weak ownership, reentry/handler containment, prompt disable and bounded replacement/close remain correct. Deterministic contention RED/GREEN and unchanged real-DB Python tests pass on a fresh native wheel; affected artifacts/features and final exact-candidate CI are refreshed. No retries, weaker phase assertions or new queue framework. |

### Fixture findings allocated during final qualification

Run41964943428 and the completed local205-suite collection distinguish two
fixture defects from product failures. CI9 isolates the foreground rollback
property from valid Autonomous progress; CI10 gives six disposable-repository
landing fixture helpers an explicit1GiB budget while preserving the real10GiB
default. The reviewed [CI9](design.md#ci9-foreground-rollback-property-owns-its-actor)
and [CI10](design.md#ci10-disposable-landing-fixtures-declare-their-resource-budget)
amendments retain observed RED, unchanged assertions and scoped GREEN, plus
fresh final exact-candidate CI. S95-2/3/8 govern these corrections; no new product
requirement or numerical SLA is added. Production-default rejection at8GiB and
separate genuine Autonomous progress/refusal remain decisive negative controls.

### Feature findings allocated during final qualification

The first feature round at5f668f330 supplied164 passing live tests and nine
findings: seven configurations hit one obsolete report-field assertion in an
ignored cost experiment; one real UTF8 extent test remained ignored; one
feature-only crash-child disposition was stale. The compiler logger also
omitted stdout diagnostics. The [CI11 amendment](design.md#ci11-feature-inventory-and-complete-compiler-diagnostics)
allocates minimal contract/inventory corrections and observed logging RED/GREEN.
S95-1/5/6/10 require complete relisting of affected configurations, unchanged
substantive assertions, actual cached-model UTF8 execution, explicit disposition
of the existing deferred timing experiment and full compiler streams. Original
failures remain attributed; reuse of unaffected passes requires exact input
comparison. Refresh final exact-candidate CI and .29 forward integration. No new
product requirement, performance campaign or numerical SLA is added.

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

### Dated qualification findings and bounded follow-up

SourceE86fb2924 fixed CI12 and passes the independent installed-wheel subscriber
suite10/10, fresh Python examples5/5 and affected PyO3 feature coverage. Its
exact CI41991174061 passed all applicable jobs except one close-fixture wall-time
assertion; [CI13](design.md#ci13-observe-the-provider-deadline-rather-than-teardown-wall-time)
replaces that proxy with actual deadline decisions, preserving production code.
[CI14](design.md#ci14-witness-concurrent-progress-in-the-long-cursor-fixture)
adds the missing positive concurrent-write witness to the existing long fixture.
Observed failures/mutation sensitivity precede GREEN; Astra reviews design/code
and a separate verifier checks the focused results. Final exact-candidate CI
remains required; neither finding justifies a full cold local/feature restart.

The qualified AC011a run is a required non-pass:59986 commits in60.006908962s,
999.651557/s <1000/s. AC011b, AC012/013, all seven hard AC081 observations,
AC021 and the accepted real-corpus AC073 stress assessment pass. AC081 retains
its existing sequential warning. The retired EU7 recall assertion exits101;
accepted TC5 fidelity remains separately attributed. No threshold, protocol or
baseline is changed. Bounded diagnostic tracing is nonqualifying and does not
establish a preparation-cost cause. Further performance direction is requested
under the saved characterize-underperformance-then-HITL policy; no waiver or
optimization is assumed. S95-6 cannot be closed on the current throughput result.

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
green CI run authorizes a version cut or main integration. Immutable
nonpublishing dispatch already has the documented ancestry exemption; its
version/Axis E and trusted workflow entry checks still apply. Prepare concrete inputs and retain a named pending prerequisite
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

## CI15: retain classified fidelity evidence

Final-source CI exposed missing post-cutover classification for the newly
registered TC5 receipt; fast lint stopped before its 198 suites. Astra approved
the bounded correction under S95-5/6/10: preserve the immutable record, resolved
configuration, metrics and index; add the existing classification sidecar, a
truthfully retrospective measurement plan and two exact-record-hash amendments
using the already supported missing-plan and missing-kind locator reasons.

The [retrospective derivation](../../../../../experiments/configs/scale-01/slice95-release-fidelity-postrun.md)
binds the actual measured source and private vector-stage execution. It makes no
preregistration, public-search, latency or byte-immutability claim. Observed
validate-tree RED precedes GREEN; all 38 existing classification tests pass.
Astra code review and separate verification check semantic metric coverage and
unchanged immutable evidence. No product AC, validator bypass or new exception
class is introduced. Recheck affected metadata/fast verification; records-only
changes retain explicit source-attributed reuse of unchanged executable evidence.

## CI16: focused AC011 diagnosis and repair

On 2026-11-05 the owner directed: continue focused diagnosis and repair, retaining
the threshold. AC011a's valid 999.651557/s non-pass remains unwaived. S95-6 governs
this work; its >=1,000/s and AC011b's >=100/s, 5-second warmup, 60-second window,
sequential commits, workload and canonical host/storage remain unchanged.

Astra reviewed a bounded two-stage diagnosis. First, preserve the exact workload
and original threshold assertion while temporarily adding aggregate phase timers
in an isolated worktree: admission waits, guards/validation, transaction begin,
trigger check, body, visibility advance and actual commit. Enable existing
metrics only after warmup and capture original elapsed/rate before printing.
Diagnostic instrumentation perturbs execution by an unknown amount; neither its
throughput nor overlapping stage totals qualify acceptance or prove the tiny
observed deficit. Interpret worker/shared metrics separately from per-write
counts. Preserve every fence, error path, lock lifetime and worker activity.

If needed, the second diagnostic counts actual SQL compilation/repreparation and
execution in the dominant phase. At most two diagnostic runs precede a new design
decision. No kernel/infrastructure knob, checkpoint, durability or trigger-policy
change is allocated. Finding a preparation call alone does not justify caching:
trigger configuration changes can expire prepared statements.

An evidenced avoidable mechanism receives Astra design review, a deterministic
correctness RED, focused GREEN/controls and Astra code review. Restore all
throwaway instrumentation before that production TDD phase. The separate verifier
checks source identity, unchanged acceptance and measured evidence. Run the
unchanged canonical AC011a/b helper once after a reviewed repair on an isolated
ready host; do not retry into green. Recheck the repair's blast radius, update
source-attributed qualification, forward to .29 and clean up. Hosted fast remains
pending until the acceptance candidate is settled. Publication remains held.

Phase 2 is complete: the scoped probe confirmed 1,000 endpoint compilations and
zero executions for the 1,000 Node writes. Astra approved only lazy preparation
after effective-edge filtering, with a real-SQLite no-read Node regression, Edge
error/count controls and unchanged receipt/search behavior. The historical miss
is not causally explained by this evidence. Keep the retained probe rerunnable
from its historical source, remove active instrumentation, then complete focused
RED/GREEN, code review and the unchanged acceptance gates.

The first canonical repaired-candidate qualification failed at 827.268/s and
stopped before AC011b; no threshold or policy was changed. Astra reassessed the
two-diagnostic budget and approved one foreground release-mode wall/thread-CPU
attribution run under the design's concentration/reconciliation decision rule.
It is diagnostic-only; no automatic extra run or production repair follows.

The wall/thread-CPU result retained a negative accounting residual and therefore
its original inconclusive verdict. Following source/clock review, Astra approved
one narrowly filtered, bounded whole-test sync trace under the design's explicit
limits. It may justify only read-only investigation, not a checkpoint policy
change, acceptance claim or retry.

The sync-only observation meets its material-span criterion, but no removable
duplicate sync was established. Before changing checkpoint/acknowledgement policy,
resolve the accepted NORMAL/100ms/zero-OS-crash contract mismatch explicitly with
the owner. Do not turn process-kill tests into machine-crash evidence or enlarge
the checkpoint interval to make throughput green. Keep existing numerical targets
unmodified until an explicit successor contract is approved.

### CI16 follow-up: reuse compiled metadata reads

Astra design review approved one further policy-neutral repair on 2026-11-07.
Existing attribution shows substantial pretransaction/body CPU work, and source
review identifies repeated compilation of the identical open-state query in
`physical_erasure::load`. Material sync spans do not establish that other safe
optimizations are exhausted. No additional diagnostic run precedes this TDD work.

Requirements and acceptance for this change:

- Reuse only compiled SQL within stable SQLite configuration; execute every
  existing read with its current key and current database contents. Preserve all
  guard calls, validation, defaults, errors, trigger/transaction lifetimes and
  durability behavior. Do not cache decoded metadata or alter cache capacity.
- A real-SQLite authorizer regression must first fail on repeated compilation of
  the actual loader, then pass unchanged with statement reuse across both keys.
  UPDATE, DELETE, malformed JSON, invalid typed state and recovery controls must
  continue to observe current data and fail closed where required.
- Real trigger enable/disable and schema invalidation controls must allow safe
  SQLite reprepare and fresh reads. Existing admission, erasure, maintenance,
  rollback and write-visibility controls cover the affected call paths.
- Separate verification checks the frozen RED/GREEN and unchanged qualifiers.
  Astra reviews code before integration. Then run unchanged canonical AC011a/b
  once on the clean reviewed candidate; retain any non-pass without retrying it
  into green. Final local/hosted qualification remains required before closure.

The pending owner durability choice governs policy changes only; this statement
preparation repair preserves the current policy and can proceed independently.

The metadata-read repair is correctness-complete at `10ca01503`, with frozen
RED/GREEN, Astra code approval and independent 14-control verification. The next
single unchanged canonical qualifier nevertheless failed at 932.474344/s.
Two focused repairs have now completed without satisfying AC011a; stop further
implementation/measurement for a reviewed source/evidence/design reassessment.
No automatic next diagnostic, speculative caching campaign, acceptance retry or
policy change follows. Performance acceptance and final qualification remain open.

Astra's next proposed investigation is one 99Hz `cpu-clock:u` profile of the exact
failed release binary, with DWARF call chains, unchanged 5s/60s workload and
preserved session/autogroup. Workers remain separate from foreground user-CPU
samples. Bound it to 120s and 256MiB; preflight permissions and symbolization.
Require >=95% symbolized samples and a foreground family with >=20% exclusive
user-CPU samples and >=200 samples before proposing a focused design; avoid
inclusive-stack double counting. Unwind loss, incomplete output or permission
failure is inconclusive and permits no second profile, policy change or repair.

The unprivileged permission preflight failed with host policy
`perf_event_paranoid=4`; no workload profile or kernel change occurred. Execution
is not authorized by this proposal: the owner has been asked to authorize a
narrow privileged profile or select a separately reviewed instrumentation plan.
Preserve the latest valid throughput failure and all existing thresholds.

The owner authorized elevated CPU profiling on 2026-11-08. Desktop authentication
preflight succeeded; the approved attach design keeps the benchmark unprivileged
and unchanged. Implement/review the retained bounded runner with failing focused
measurement/control checks first, then collect the single profile and obtain
independent attribution verification. Durability clarification is not a request
to change WAL/NORMAL or choose a new contract before this investigation.
