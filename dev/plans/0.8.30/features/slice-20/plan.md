---
title: Slice 20 efficient and diagnosable CI/CD
status: PROPOSED
target_release: 0.8.30
slice: 20
---

# Slice 20 efficient and diagnosable CI/CD

Make a failed CI run actionable after cancellation, make normal npm finalization
use the qualified OIDC path, and reduce measured verification overhead while
preserving test coverage. Depends on approved Slice 10 preparation and separate
Slice 20 commissioning. Follow the [master execution contract](../../plan.md#c-slice-planning-and-execution-contract).

## Entry and scope

- Reconcile current `origin/main`, the release checkout, and changes since draft base `f7fb1da9a`. Review the actual Slice 9 scope decisions, Slice 10 outcomes, and assigned requirements, architecture, test and CI drafts from prework 3–5 and 7. Record what is adopted, revised or rejected against current code and run evidence.
- The .29 record identifies a Rust exit 101 whose diagnostic was lost after cancellation, a 856.43 s Windows workflow source check, about 8 min 50 s of fast bootstrap, a legacy npm E401, and a successful OIDC mutation with an unexplained first readback failure. The original missing npm native binary also has an unknown resolver cause. [0.8.29 retrospective](../../../../notes/0.8.29-retrospective.md)
- Reuse the existing verifier, workflow jobs, test fixtures and release-state reader. Keep current job budgets, platform coverage, security checks, trusted refs, reviewer and worker controls. Slice 80 owns authorized live promotion and GitHub Release completion. This slice qualifies the source and offline behavior.
- Use the [testing findings](../../testing-robustness-findings.md) and [delivery recommendations](../../quality-performance-robustness-recommendations.md) to select improvements to existing tests. Carry forward the .29 approved Pages lifecycle correction within its existing scope.

## Local requirements and acceptance

These are release-local requirements. The fixtures described below are proposed
extensions of existing tests. Reconcile their exact inputs and thresholds before
the design and updated-plan reviews.

| Unit | Proposed behavior | Acceptance evidence |
| --- | --- | --- |
| Failure evidence and cancellation | Extend the existing suite runner, output capture and redacted spill path. Expose each completed outcome and useful full failure evidence to a CI-visible sink before the next child starts. Record the selected registration order before execution so a later cancellation leaves an identifiable active child and unrun tail. Preserve quiet success, standalone `run_capped` exit propagation and the collect-all verdict. | Start with a passing suite, fail a second with exit 101 and a diagnostic beyond line 200, then cancel a subsequent blocking child after a readiness handshake. Before cancellation the original exit, late diagnostic and completed outcomes are available. After cancellation the active child and never-started tail remain distinguishable from PASS, FAIL, SKIP and EXCL. Demonstrate redaction and bounded live capture with disposable inputs and no late upload step. |
| Elapsed time | Reuse the suite runner's monotonic clock for remaining verifier and capped-output elapsed-time calculations. Keep wall time for timestamps and report unavailable elapsed time if the monotonic source fails. | Move a fixture's wall clock backwards between start and completion. Both wrappers retain their outcomes and report nonnegative monotonic durations. A missing or failing timing source preserves the original command exit and emits an explicit unavailable value. |
| Normal and recovery OIDC | Share one promotion implementation using the recovery route's pinned npm 11.21.0, subject to exact-pin and workflow-trust reconciliation. Keep normal smoke and co-tag prerequisites and recovery's exact tag, source and production-run checks. Grant OIDC only to the job that needs it. | Offline tests drive the same implementation from both routes. The successful path issues one `dist-tag add` for the exact `fathomdb@VERSION` to `latest`, with no legacy token or platform package mutation. Failed prerequisites, dry-run, candidate and partial-release routes cannot promote or finalize. GitHub Release remains downstream of verified promotion. |
| Promotion visibility | Give the readback an explicit attempt and elapsed-time bound. Log the expected version and sanitized observed value or lookup error on each mismatch. Keep exact-version visibility before installed smokes. | A fixture returns the old `latest` twice and then the target. Persistent mismatch, missing value or transport failure exhausts the bound and fails with the last observation. The mutation runs once, including on visibility failure, and GitHub Release remains blocked. |
| npm native smoke | Preserve the exact install exit and useful redacted npm output before owned temporary cleanup. Report requested and resolved main version, expected optional package and version, actual resolution or absence, host platform/architecture/libc where relevant, and the expected `.node` path. Apply to the Bash and PowerShell entry points where the failure can occur. | An install that exits zero with a missing optional binary remains a failed smoke and supplies those fields. A failed install retains its exit and usable output. A wrong platform artifact cannot satisfy the expected path or Linux glibc check. Keep real native open/write/read/close and process-exit controls. |
| Closed-ladder validation | Teach `commission-manifest --verify-all` to read the recorded scope when `next_slice` is null. Accept .29 Slice 120's `DEFERRED_BY_OWNER` with its ruling and Slice 150's `NOT_STARTED` with its explicit optional, unselected disposition. Keep the landed and exact release-branch completion checks. | The real .29 state and a mixed terminal fixture pass read-only. Fail for a selected unfinished row, a generic `NOT_STARTED` row, a claimed deferral without authority, malformed rows, nonempty remaining work or an invalid completion ref. Preserve each disposition and the historical JSON bytes. |
| Windows static checker | Parse relevant source and workflow input once per file, evaluate meaningful mutations against the affected checks, and assign the suite by measured cost. Preserve workflow selectors, real Windows cross-process and owned-reader WAL tests, installed controls, and local verbatim-path coverage. | Time matching checker runs before and after with the same source, tools and reported host conditions. Freeze a meaningful reduction criterion before editing, with a proposed target of at least 50% below the comparable baseline. Wrong or zero-test selectors, dropped exit guards, and incorrect artifact or ownership paths still fail. Use actual selected Windows runtime evidence or report the runtime scope outstanding. |
| Tool-only bootstrap and verification DAG | Split pinned tool setup from native construction through the checkout-owned setup path. Keep native prerequisites for checks that import or execute bindings. Remove measured duplicate setup or builds only where consumer requirements and exact producer identity match. | A tool-only fixture installs and checks tools without starting `maturin build` or a native wheel install. Native consumers use their own matching artifact. Changes to source, lock, features, interpreter, target or build inputs, and incomplete identity, require a fresh build. Compare setup/execution and repeated builds on the same routing inputs. Docs/state-only edits avoid native jobs while relevant source, security and platform routes still execute. |

### Publication prerequisites owned here

Slice 20 implements and qualifies the two source gaps identified by
[Slice 80](../slice-80/plan.md), before70 qualification and actual80 delivery:

- Extend the maintained exact-first-tag bootstrap only for an explicitly
  owner-approved `v0.8.30` route or equivalent. Offline positive/negative tests
  preserve denial for other tags, ambiguous/network-failed ref lookup, absent
  trusted provenance and missing reviewers. No runner-group, environment or
  trust-policy change is inferred from this draft. An unavailable authorization
  remains an80 prerequisite, not permission to weaken the guard.
- Extend maintained exact-version public SDK/Python/Node consumer flows to
  write/read the expected body, assert the missing-record outcome, close/exit,
  then reopen/read/close in a separate process. Preserve wrong-version/native
  negatives. Slice20 proves offline consumer behavior and failure sensitivity;
  70 checks candidate bytes; 80 independently verifies actual registry installs.
  Never substitute a source archive for that final public-registry proof.

## Design seams and research

- Trace `verify-fast` and `verify` from bootstrap through the verifier and collect-all runner. Design output persistence around existing invocation-owned capture and redact before publishing. Compare when available log and artifact channels persist output under GitHub Actions cancellation and timeouts using primary GitHub documentation. Preserve the execution distinction between normal completion, handled cancellation and abrupt runner loss. [CI workflow](../../../../../.github/workflows/ci.yml) · [Output capture](../../../../../scripts/lib/agent-output.sh) · [Suite runner](../../../../../scripts/lib/agent-suite-run.sh)
- Trace the normal and recovery permissions and dependency graphs. Check primary npm CLI source or release notes and GitHub/npm trusted-publishing documentation for the pinned CLI's `dist-tag` behavior, the caller identity of any shared implementation, and registry read caching. Use the repository's existing bounded version-visibility helper as a reference. Preserve exact-tag admission and require a check of matching registered workers and local services before Slice 80 enables the exact production job. [Release design](../../../../../dev/design/release.md) · [Normal workflow](../../../../../.github/workflows/release.yml) · [Recovery workflow](../../../../../.github/workflows/promote-npm-release.yml) · [Visibility helper](../../../../../scripts/release/wait-for-registry-version.py)
- Inspect current state fields and branch closure rules before choosing how to validate scope evidence. Check the .29 positive case against an unfinished selected case with the same empty `remaining_ladder`. Keep validation read-only and preserve the active manifest path. [Commission manifest](../../../../../scripts/commission-manifest.sh) · [0.8.29 state](../../../../../dev/plans/release-state-0.8.29.json)
- Measure the current checker and selected setup/DAG legs before optimizing. Trace developer tool imports and native consumers before moving setup. Carry forward checked producer identity for any reuse and preserve the strict security prerequisite and serial Rust oracle where selected. [Bootstrap](../../../../../scripts/bootstrap.sh) · [Heavy bootstrap](../../../../../scripts/bootstrap-heavy.sh) · [Owned Python setup](../../../../../scripts/setup-python-owned.sh)

## Implementation and independent verification

1. Draft the design and any needed requirement, interface or release-contract deltas against the reconciled code. Resolve the difficult decisions above and have a direct Astra-medium subagent review the design additions.
2. Put the proposed cases and existing controls into the updated slice plan. Obtain a direct Astra-medium plan review, allow at most one FIX-1 and re-review, then commit the plan only after PASS. Hold the affected transition if the review limit is reached.
3. After commissioning and prerequisites, stage or commit a genuine test RED per changed behavior, preserve the observed failure, then implement GREEN and refactor in coherent units. Use controlled child processes and offline registry fixtures for harness failures. Preserve human-defined expectations and real databases for WAL and installed runtime controls.
4. Obtain direct Sol 6.2 high code review and direct Terra verification of the affected tests and evidence, including exit codes and timing comparisons. Spawn these reviewers directly under the master contract. Steward, Orchestrator and MCP runners are outside this workflow.
5. Run scoped lint, type checks and affected tests while working. Run one required `./scripts/agent-verify.sh` on a stable coherent source phase from the owned checkout. For narrow corrections, rerun failed, affected and unrun scopes. Record the actual results and any outstanding hosted or platform proof.

### Existing verification to extend or retain

- Run `bash scripts/tests/test_ci_verify_observability.sh` and `bash scripts/tests/test_agent_test_collect_all.sh`. The former already checks exact exit and redacted spill beyond 200 lines, while the latter checks aggregate and per-suite outcomes. Add the cancellation and remaining-wrapper clock cases.
- Run actionlint on affected workflows, `bash scripts/tests/test_release_workflow_scope.sh`, `python3 scripts/tests/test_wait_for_registry_visibility.py` and `bash scripts/tests/test_smoke_scripts.sh`. Add behavioral promotion/readback and missing-native cases with controlled fixtures. Use the [Bash smoke](../../../../../scripts/release/smoke/smoke-npm-package.sh) and [PowerShell smoke](../../../../../scripts/release/smoke/smoke-npm-package.ps1) as the entry points.
- Run `bash scripts/tests/test_commission_manifest.sh`, `bash scripts/commission-manifest.sh --verify-all` and `bash scripts/check-release-state-views.sh`. Preserve active, completed and malformed fixtures.
- Run `bash scripts/tests/test_windows_wal_attribution_ci_job.sh`, `bash scripts/tests/test_windows_wal_diagnosis_ci_job.sh`, the affected modes of `bash scripts/tests/test_setup_python_owned.sh`, `bash scripts/tests/test_bootstrap_heavy.sh` and `python3 scripts/tests/test_ci_proportional_routing.py`. For affected Windows runtime proof, retain the actual `erasure_busy_cross_process_windows_yields_typed_diagnostic` selector and the managed-reader and installed controls from the [CI workflow](../../../../../.github/workflows/ci.yml).

## Closure

Record one concise update with actual test exits, any skips or unknowns,
the matched before/after measurements, and where the first-failure evidence
survived. For selected R30-TEST work, give Slice 70 the owning test, the wrong
behavior it rejects, and any consolidation's coverage and cost evidence.
Update affected release instructions, contracts and canonical state once.
Integrate into the owning release branch and clean only owned inactive scratch
and worktrees after preserving needed evidence.

## Prerequisites and open design choices

- Slice 10 must identify the actual checkout-owned tooling, approved prep and available verification environments. Availability limits belong in the test results.
- Settle the pre-cancellation evidence sink, bounds and failure behavior against the actual runner before design approval.
- Confirm the qualified OIDC pin and trusted caller identity at entry. Any required registry trust change or live qualification requires its own authority.
