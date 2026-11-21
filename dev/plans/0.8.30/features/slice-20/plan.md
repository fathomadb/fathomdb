---
title: Slice 20 efficient and diagnosable CI/CD
status: COMPLETE
target_release: 0.8.30
slice: 20
---

# Slice 20 efficient and diagnosable CI/CD

Make a failed CI run actionable after cancellation, make normal npm finalization
use the qualified OIDC path, and reduce measured verification overhead while
preserving test coverage. Depends on approved Slice 10 preparation and separate
Slice 20 commissioning. That commissioning is recorded at seq-401. Follow the [master execution contract](../../plan.md#c-slice-planning-and-execution-contract).

## Reconciliation at implementation entry

The commissioned worktree begins at `85d9504bb`, on the release lineage
containing `origin/main` at `b86b19847`. Since draft base `f7fb1da9a`:

| Input | Resolution for Slice 20 |
| --- | --- |
| .29 delivered and closed | Retain the recorded earlier cancellation, 856.43 s and 579.34 s Windows-check observations, roughly 8m 50s fast bootstrap, npm E401 and OIDC PUT whose immediate readback failed. These are symptoms and unmatched historical measurements. Keep the authorized Pages lifecycle correction and .29 publication and terminal rows intact. |
| Slice 9 rulings and prework 3–7 | Adopt R30-CI/DIAG, approved process-level discriminators and existing exact platform/registry guards. Current source still has an end-only suite table, a 200-line capped failure, legacy normal promotion, immediate recovery readback, and duplicate Python production-wheel preparation. The existing Rust/Python/Node registry checks have incomplete persisted-read evidence. No receiving allocation in release-followups expands this slice. |
| Slice 10 landed through `9c9b78658` | Keep the Markdown/security and developer Node 26.13.1/npm 11.20.0 pins and the independently held production and Tegra toolchains. Reuse the noneditable owned setup and exact installed consumer shape. Of the five full-gate failures, the preparation cohort is already fixed. This slice owns the pending-publication fixture and .29 closed-ladder tooling. Hints timing belongs to 40 and Python logging to 50. The renderer and public Node support remain inputs to 70. [Slice 10 evidence](../slice-10/status.md) |
| Slice 80 prerequisites | seq-401 authorizes exact .30 first-tag source and offline controls. The existing tag guard lists .28/.29 only. Source work extends the exact branch while retaining its checks. Runtime/registry delivery, environment/trusted-publisher settings and matching registered workers are separate 80 prerequisites. |
| Quality roadmap | Apply real installed consumer regressions, small deterministic fault scenarios and equivalent-platform controls. Existing delivery recommendations own the selected improvements. Skip product expansion and a broader test-harness rewrite. |

## Local requirements and acceptance

The acceptance cases extend the existing suites and use the implementation design
below.

| Unit | Proposed behavior | Acceptance evidence |
| --- | --- | --- |
| Failure evidence and cancellation | Extend the existing suite runner, output capture and redacted spill path. Expose each completed outcome and useful full failure evidence to a CI-visible sink before the next child starts. Record the selected registration order before execution so a later cancellation leaves an identifiable active child and unrun tail. Preserve quiet success, standalone `run_capped` exit propagation and the collect-all verdict. | Start with a passing suite, fail a second with exit 101 and a diagnostic beyond line 200, then cancel a subsequent blocking child after a readiness handshake. Before cancellation the original exit, late diagnostic and completed outcomes are available. After cancellation the active child and never-started tail remain distinguishable from PASS, FAIL, SKIP and EXCL. Demonstrate redaction and bounded live capture with disposable inputs and no late upload step. |
| Elapsed time | Reuse the suite runner's monotonic clock for remaining verifier and capped-output elapsed-time calculations. Keep wall time for timestamps and report unavailable elapsed time if the monotonic source fails. | Move a fixture's wall clock backwards between start and completion. Both wrappers retain their outcomes and report nonnegative monotonic durations. A missing or failing timing source preserves the original command exit and emits an explicit unavailable value. |
| Normal and recovery OIDC | Share one promotion implementation using the recovery route's pinned npm 11.21.0, subject to exact-pin and workflow-trust reconciliation. Keep normal smoke and co-tag prerequisites and recovery's exact tag, source and production-run checks. Grant OIDC only to the job that needs it. | Offline tests drive the same implementation from both routes. The successful path issues one `dist-tag add` for the exact `fathomdb@VERSION` to `latest`, with no legacy token or platform package mutation. Failed prerequisites, dry-run, candidate and partial-release routes cannot promote or finalize. GitHub Release remains downstream of verified promotion. |
| Promotion visibility | Give the readback an explicit attempt and elapsed-time bound. Log the expected version and sanitized observed value or lookup error on each mismatch. Keep exact-version visibility before installed smokes. | A fixture returns the old `latest` twice and then the target. Persistent mismatch, missing value or transport failure exhausts the bound and fails with the last observation. The mutation runs once, including on visibility failure, and GitHub Release remains blocked. |
| npm native smoke | Preserve the exact install exit and useful redacted npm output before owned temporary cleanup. Report requested and resolved main version, expected optional package and version, actual resolution or absence, host platform/architecture/libc where relevant, and the expected `.node` path. Apply to the Bash and PowerShell entry points where the failure can occur. | An install that exits zero with a missing optional binary remains a failed smoke and supplies those fields. A failed install retains its exit and usable output. A wrong platform artifact cannot satisfy the expected path or Linux glibc check. Keep real native open/write/read/close and process-exit controls. |
| Closed-ladder validation | Teach `commission-manifest --verify-all` to read the recorded scope when `next_slice` is null. Accept .29 Slice 120's `DEFERRED_BY_OWNER` with its ruling and Slice 150's `NOT_STARTED` with its explicit optional, unselected disposition. Keep the landed and exact release-branch completion checks. | The real .29 state and a mixed terminal fixture pass read-only. Fail for a selected unfinished row, a generic `NOT_STARTED` row, a claimed deferral without authority, malformed rows, nonempty remaining work or an invalid completion ref. Preserve each disposition and the historical JSON bytes. |
| Windows static checker | Parse relevant source and workflow input once per file, evaluate meaningful mutations against the affected checks, and assign the suite by measured cost. Preserve workflow selectors, real Windows cross-process and owned-reader WAL tests, installed controls, and local verbatim-path coverage. | Time matching checker runs before and after with the same source, tools and reported host conditions. Freeze a meaningful reduction criterion before editing, with a minimum 50% reduction in median wall time across three matched repetitions. Wrong or zero-test selectors, dropped exit guards, and incorrect artifact or ownership paths still fail. Use actual selected Windows runtime evidence or report the runtime scope outstanding. |
| Tool-only bootstrap and verification DAG | Split pinned tool setup from native construction through the checkout-owned setup path. Keep native prerequisites for checks that import or execute bindings. Remove measured duplicate setup or builds only where consumer requirements and exact producer identity match. | A tool-only fixture installs and checks tools without starting `maturin build` or a native wheel install. Native consumers use their own matching artifact. Changes to source, lock, features, interpreter, target or build inputs, and incomplete identity, require a fresh build. Compare setup/execution and repeated builds on the same routing inputs. Docs/state-only edits avoid native jobs while relevant source, security and platform routes still execute. |

### Publication prerequisites owned here

Slice 20 implements and qualifies the two source gaps identified by
[Slice 80](../slice-80/plan.md), before70 qualification and actual80 delivery:

- Extend the maintained exact-first-tag bootstrap for the
  owner-approved `v0.8.30` source route (seq-401). Offline positive/negative tests
  preserve denial for other tags, ambiguous/network-failed ref lookup, absent
  trusted provenance and missing reviewers. No runner-group, environment or
  trust-policy change is inferred from this draft.
- Extend maintained exact-version public SDK/Python/Node consumer flows to
  write/read the expected body, assert the missing-record outcome, close/exit,
  then reopen/read/close in a separate process. Preserve wrong-version/native
  negatives. Slice20 proves offline consumer behavior and failure sensitivity;
  70 checks candidate bytes; 80 independently verifies actual registry installs.
  Never substitute a source archive for that final public-registry proof.

## Implementation design

- **Cancellation and timing.** Collect the selected tier's suite registrations in order before dispatch. Preserve argv, skip reason and exclusion semantics. Write a plan line per suite and a start and outcome line at the appropriate boundaries in the GitHub job log. Absence of an outcome for a recorded start means execution was interrupted or the runner lost contact, while no start or outcome means unrun. GitHub can interrupt the shell or kill its process tree, so the recorder does not depend on a signal trap, final summary or upload. Retain final summary and full redacted artifacts when the job reaches those steps. At failure time flush an immediate redacted diagnostic including lines beyond 200, bounded by bytes and with an explicit truncation marker for oversized logs. Limit live output to the first failed suite plus concise later outcomes. Use the same redactor for artifacts and early output and disable interpretation of log text as workflow commands. Share the monotonic timestamp and duration calculation with verifier and capped wrapper; failure of the timer produces `unavailable` and preserves command status. [GitHub cancellation](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-cancellation) · [Job logs](https://docs.github.com/en/actions/how-tos/monitor-workflows/use-workflow-run-logs) · [Workflow commands](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands)
- **Promotion.** Use one repository script called by the existing hosted OIDC jobs. Add the pinned checkout for `github.workflow_sha` with `persist-credentials: false` to normal promotion, following the existing visibility job’s control-plane checkout. Retain the trusted recovery checkout with credentials disabled. Check that each job invokes the shared script from its own workflow source, including dispatches targeting another release tag. Keep Node 25.9.0 and install/verify exactly npm 11.21.0 for promotion only. Preserve existing job conditions and require the GitHub OIDC request variables. Invoke `dist-tag add` once against public npm for exact main package/version in an isolated working directory and exclude legacy auth configuration. Poll `npm view` with online revalidation, no implicit retries, maximum six read attempts and a 60 s monotonic budget, including subprocess time. Log the expected version and each validated observed version, missing value or safe error classification. Readback remains a prerequisite of finalization. Existing exact-version registry polling and all eight consumers remain upstream. Auditing separate trusted-publisher action permission on both workflow identities belongs to 80. [npm trusted publishing](https://docs.npmjs.com/trusted-publishers/) · [Pinned dist-tag implementation](https://github.com/npm/cli/blob/v11.21.0/lib/commands/dist-tag.js) · [npm online revalidation](https://docs.npmjs.com/cli/v11/using-npm/config/#prefer-online)
- **Consumer flow.** Keep published install paths, exact-version and platform/glibc controls. The Rust smoke resolves and compiles the exact dependency and invokes its binary twice. Share the Python and Node persistence fixtures between Unix and Windows entry points. Keep the existing installed trace/refusal controls. First process uses an explicit known record ID and exact UTF-8 body, verifies the missing ID, then closes. Second process reopens the same path and checks the body and missing ID without writes, then closes. Each process must exit successfully. For Node, check exact main and optional manifest versions, the OS/arch/libc-specific file resolved from the fresh consumer, and that Node actually loaded that file. Preserve npm install failures and report redacted npm output before temporary cleanup. Use native offline build/package inputs and discriminating wrong-version/missing-native/missing-store cases. [Native consumer scripts](../../../../../scripts/release/smoke/)
- **Closed state and public docs.** Use one terminal-ladder validator from commission-manifest and the no-active preflight regression. Drive the real release selector against a tracked all-published fixture while .30 is active in the worktree. Exercise the production verifier’s no-live-release branch, accepting valid published closure for the regression and refusing a selected unfinished row. A slice/dependency preflight without a selected live release continues to refuse commission and worktree entry. Require an explicit null next slice, empty remaining ladder, well-formed unique slice IDs and consistent landed membership. Validate any completion ref against its exact release branch and handle pending main integration explicitly. Treat .29 Slice 120 as deferred with the recorded `artifact-selection` ruling and Slice 150 as optional with the recorded unselected disposition. Generic unfinished rows still fail. Construct the pending docs fixture from controlled publication-facing prose and explicit negative mutations. The real checker continues selecting the published snapshot when the active state lacks `public_docs_target_release`. Do not modify historical state bytes or set the live release pointer.
- **Windows static verification.** During baseline, track wall/user/system time and load with the complete existing mutation set. Freeze a minimum 50% median wall-time reduction, with three before/after runs on this host and no missing outcome. The 1,762-line attribution checker recursively invokes the entire checker for each mutant. Evaluate each mutant with the same relevant workflow or ownership rules, reuse parsed source/module data for unaffected inputs, and preserve existing workflow job/selector, artifact, opener ownership and native control assertions. Keep attribution in the fast tier if it meets the frozen criterion, with diagnosis and Windows runtime jobs preserved. Check independent wrong/zero selector and wrong artifact/owner mutations. Windows runtime claims still need a runnable Windows venue.
- **Bootstrap and routing.** Introduce tool-only provisioning in the checkout-owned setup script, including the existing ownership checks and pin validation. Use it in fast CI instead of building a production wheel. Move the real installed Slice 135 Python tests to heavy with their production wheel; audit the other fast consumers and keep tool-only fixtures in fast. Keep production and test-hook Python artifacts distinct. Share the heavy production wheel among consumers in that job and retain fresh builds for separate jobs, source/lock/input changes or incompatible feature sets. Apply the existing docs-only path to narrowly matched release-state and board metadata, with the independent state, docs and current-tree checks still covering those paths. Retain fail-safe routing, explicit release qualification, Rust serial oracle and strict ptrace. Measure the owned provisioning legs and compare job selection/build count on the same fixtures. [CI workflow](../../../../../.github/workflows/ci.yml) · [Owned setup](../../../../../scripts/setup-python-owned.sh)

## Implementation and independent verification

1. Submit this updated plan and design together for a direct Astra-medium review. Allow at most one FIX-1 and re-review. Commit the reviewed plan after PASS and before implementation.
2. After commissioning and prerequisites, stage or commit a genuine test RED per changed behavior, preserve the observed failure, then implement GREEN and refactor in coherent units. Use controlled child processes and offline registry fixtures for harness failures. Preserve human-defined expectations and real databases for WAL and installed runtime controls.
3. Obtain direct Sol 6.2 high code review and direct Terra verification of the affected tests and evidence, including exit codes and timing comparisons. Spawn these reviewers directly under the master contract. Steward, Orchestrator and MCP runners are outside this workflow.
4. Run scoped lint, type checks and affected tests while working. Run one required `./scripts/agent-verify.sh` on a stable coherent source phase from the owned checkout. For narrow corrections, rerun failed, affected and unrun scopes. Record the actual results and any outstanding hosted or platform proof.

### Existing verification to extend or retain

- Run `bash scripts/tests/test_ci_verify_observability.sh` and `bash scripts/tests/test_agent_test_collect_all.sh`. The former already checks exact exit and redacted spill beyond 200 lines, while the latter checks aggregate and per-suite outcomes. Add the cancellation and remaining-wrapper clock cases.
- Run actionlint on affected workflows, `bash scripts/tests/test_release_workflow_scope.sh`, `python3 scripts/tests/test_wait_for_registry_visibility.py` and `bash scripts/tests/test_smoke_scripts.sh`. Add behavioral promotion/readback and missing-native cases with controlled fixtures. Use the [Bash smoke](../../../../../scripts/release/smoke/smoke-npm-package.sh) and [PowerShell smoke](../../../../../scripts/release/smoke/smoke-npm-package.ps1) as the entry points.
- Run `bash scripts/tests/test_commission_manifest.sh`, `bash scripts/commission-manifest.sh --verify-all`, `python3 scripts/tests/test_preflight_release_state.py`, and `bash scripts/check-release-state-views.sh`. Preserve active, completed and malformed fixtures.
- Run `bash scripts/tests/test_windows_wal_attribution_ci_job.sh`, `bash scripts/tests/test_windows_wal_diagnosis_ci_job.sh`, the affected modes of `bash scripts/tests/test_setup_python_owned.sh`, `bash scripts/tests/test_bootstrap_heavy.sh` and `python3 scripts/tests/test_ci_proportional_routing.py`. For affected Windows runtime proof, retain the actual `erasure_busy_cross_process_windows_yields_typed_diagnostic` selector and the managed-reader and installed controls from the [CI workflow](../../../../../.github/workflows/ci.yml).

## Closure

Record one concise update with actual test exits, any skips or unknowns,
the matched before/after measurements, and where the first-failure evidence
survived. For selected R30-TEST work, give Slice 70 the owning test, the wrong
behavior it rejects, and any consolidation's coverage and cost evidence.
Update affected release instructions, contracts and canonical state once.
Integrate into the owning release branch and clean only owned inactive scratch
and worktrees after preserving needed evidence.

## Public smoke control repair during Slice 80

The immutable public `v0.8.30` tag peels to
`e3d31f5cdc0df3e32dab9bad151c5ab5c321222c`; production run `44929252460`,
attempt 1, published all registries. Its Linux PyPI and macOS ARM Python
installs failed during propagation. Intel macOS Python instead skipped every
runtime call: Bash 3's empty-array expansion raised an unbound-variable error,
and its successful EXIT cleanup masked that error as exit 0. Intel npm passed
in a separate step. Keep these original outcomes and package bytes unchanged.

Use portable positional arguments for the optional Linux native path. Preserve
cleanup's incoming failure status, and default completion to failure until all
Python write, reopen and installed trace consumers finish. A Bash 3 regression
must first demonstrate absent calls and false exit 0, then require both calls
and exact install/runtime failure exits. Shell fixtures prove control flow;
actual recovered registry processes provide product evidence.

Extend only the maintained npm recovery workflow with an opt-in, exact `.30`
mode. Bind version, annotated tag object, source, production run and attempt 1.
Require completed original publishers, co-tagging and unaffected consumer jobs;
inherit Intel npm only from its unique completed successful step 7 with the
exact original command name. Never count its Python false green. Refuse wrong
identity, attempt, missing, duplicate, skipped or failed inherited proof.
Paginate the bound attempt's jobs. Run fresh corrected Python consumers on
Linux x64 and both macOS architectures, plus the unrun macOS ARM npm consumer,
from the trusted workflow source with read-only permissions. Require every
recovery matrix job to succeed before the existing pinned OIDC promotion and
original-run GitHub Release attachment. The default all-original-green recovery
route retains its all-original-success requirements for unaffected versions;
`.30` must use corrected consumers even if a tag rerun appears all green. The
normal release workflow retains its existing requirements. No
build, publisher, GPU campaign, retag or version change belongs to this repair.

Obtain direct Astra-medium design approval after at most one consolidated FIX,
then stage behavioral workflow refusal RED before implementation. Direct
Sol6.2-high reviews final source; delivery's Terra verifies actual recovery.
Use affected script tests, actionlint and ShellCheck, retaining the accepted
full source/native qualification without repeating it for this control repair.
The parent integrates and dispatches recovery after delivery's source checks.
