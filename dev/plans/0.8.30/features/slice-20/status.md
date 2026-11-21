---
title: Slice 20 implementation and verification
status: COMPLETE
target_release: 0.8.30
slice: 20
date: 2026-11-18
---

# Slice 20 implementation and verification

Commissioned by seq-401 from `85d9504bb73761041a5b695dc4f8fae29b61bd3d`.
Astra-medium passed the updated [plan and design](plan.md) after FIX-1.
Sol-high passed the source phase at
`5093cf0fd1e798c9b87d6806d2704fd223c0d096`. Review fixes cover the live and
retained credential forms, capped npm log fragments, inherited npm identity
overrides and all owned inputs to the moved Slice 135 suite. The first routing
fix covered Python and JSON. The final fix also covers the Node driver and
Cargo snapshots, with a passing Markdown-only control.

| Scope | Result |
| --- | --- |
| Cancellation and time | `test_ci_verify_observability.sh`, `test_agent_test_collect_all.sh`, `test_agent_verify_markdown_scope.sh` and tier totality exited 0. Before a later child's cancellation, the CI-style log retained exit 101, a diagnostic beyond line 200, the completed pass and failure, the started child and the unrun tail. No final summary or later upload ran in that fixture. Late secrets were redacted. Clock corrections and an unavailable timer preserved verdicts. |
| Promotion and first tag | `test_npm_promotion.py`, the loopback registry visibility fixture, release workflow scope/contract, 55 tag-route cases and actionlint exited 0. Both workflow commands used one pinned OIDC mutation in the fixture, then observed two stale values and the exact target. Pin/identity, timeout, invalid/absent data and persistent failure controls passed. The exact npm 11.21.0 CLI accepted the isolated config. |
| Native smokes | `test_smoke_scripts.sh` exited 0, including npm's missing matching binary with a wrong-platform decoy, exact install exit 43 and secret-boundary tests. Real offline Linux x64 consumers exercised two process lifetimes per binding, reading the exact UTF-8 body and missing result. Rust used an exact-version local SDK dependency. Python installed the ABI3 wheel in its own fresh venv. Node 26.13.1/npm 11.20.0 used paired .29 tarballs with publish-time optional metadata injected and confirmed the loaded addon. Wrong version and missing store controls failed as intended. |
| Release-state truth | The public-doc fixture, real public checker, commission fixture/sweep, preflight state and landing, and generated views exited 0. The live doc checker selects published .29. The retained deferred 120 and optional unselected 150 rows pass closure in the recorded scope. Selected unfinished rows, missing rulings, malformed/duplicate rows and wrong completion refs fail. |
| Owned setup and routing | The ten owned-setup fixture modes, bootstrap-heavy and exact pinned-path routing exited 0. The new owned environment had no installed binding, and all 165 selected fast experiment tests passed. Core setup took 12.35 s versus 92.28 s for the original provision-and-build path with its cold Cargo work. The paired route now builds one production wheel instead of two. The native test-hooks artifact serves its distinct purpose. State-only edits use the scoped route and mixed source edits retain required jobs. |
| Windows static controls | All 345 attribution assertions and 25 diagnosis assertions passed, including wrong and zero-selected selectors, artifact/owner locations, and native exit checks. Baseline spans from log creation to verdict were 628.91, 629.49 and 628.47 s. Candidate elapsed times were 149.60, 150.97 and 149.57 s on the same host and frozen workflow/Rust/Python inputs. The median fell 76.2% against a 50% threshold. The host has 24 logical CPUs. Candidate starts recorded one-minute load of 1.29–2.33. An exploratory checker run ended with exit 127 when the script changed mid-run. The comparison uses the three completed runs per side. |

The staged npm main tarball had SHA-256 `2b3b97a353886d7e0b3b87f41c14a8780a0e3e09b75fe2cc75a1671ce91765be`.
The loaded Linux x64 addon had SHA-256 `d61de587c2bd1779c85a1b67227be688371d4e8f2ab06f1688c7edea1bd2a68f`.
The final source-bound Python wheel had SHA-256
`6248cf3be4d63cefe0f59b2bbe5b17707434041f4997ed786522d2f614c7dd88`.

## Full gate and focused recovery

Terra ran `bash scripts/agent-verify.sh` once from clean
`5093cf0fd1e798c9b87d6806d2704fd223c0d096` in a ptrace-capable executor.
Preflight, lint, typecheck and strict security passed. AC-036 captured no
successful listen syscall, and security reported zero violations, blockers or
downgrades. The default serial Rust workspace, Python, Python native receipt,
TypeScript and executable N-API checks passed.

The command exited **1** after **3,791 s**. The runner registered 241 suites,
ran 238, passed 236, failed 2, skipped the three private live controls and
excluded none. Lint explicitly skipped Tegra Rust and unconfigured ESLint.
The runner counts describe suite outcomes. Optional test bodies and other
platforms retain their own execution and skip boundaries.

- `test-runtime-release-alignment` exited 1 with
  `CHECKOUTS False total=35 control=5`. Its fixture still expected the five
  previous workflow-SHA checkout jobs. The sixth is the reviewed normal npm
  promotion job with credential persistence disabled.
- `test-create-venv-with-selected-python` exited 1 at arm C because the fixture
  expected a literal `--provision` call. Bootstrap now selects the explicit
  owned provisioning or tools-only mode.

Follow-up `7299d8be7a0348c809eeb2fb32c42c40649908a6` changed only those
two fixtures. Both commands exited 0 in the focused rerun and Terra's
independent recheck. The release workflow scope, heavy bootstrap, tools-only
fixture and scoped ShellCheck also exited 0. The original full-gate outcome
remains recorded as exit 1. The earlier hints timing and logging evidence
remains with Slices 40 and 50 respectively.

Hosted cancellation, Windows runtime, broader native candidate qualification
and live registry delivery remain open in their owning venues. Slice 80 owns
production runner, environment and npm trust checks and actual publication.
Raw logs, selected artifact inputs and the timing comparison are under
`/tmp/fathomdb-0830-slice20-evidence/` for review and worktree cleanup.

## Release-branch integration

Reviewed implementation `5e7906b6d7a871611ccdddd74a9ed1410c32cc47` is integrated
into local `release/0.8.30` after landing preflight. Both post-gate corrections
were inspected against the full-gate subject; they retain checkout trust,
default native provisioning and explicit tools-only controls. No second full
run was performed. Source/offline implementation is complete; the original
full-run exit and later qualification limits remain as recorded above.
Slice30 is next and remains uncommissioned. Raw evidence is preserved outside
the implementation worktree before cleanup.

## Slice 80 public-smoke control repair

Baseline `e3d31f5cdc0df3e32dab9bad151c5ab5c321222c` and public annotated
`v0.8.30` object `fbae0d3319fb31bb62360977fded676f2dea8557` are unchanged.
Production `44929252460`, attempt 1, failed Linux PyPI and macOS ARM install
resolution during propagation. Intel Python step 6 falsely reported success:
its empty native-argument array aborted under Bash 3 before write/reopen, then
successful EXIT cleanup returned 0. Its separate npm step 7 genuinely passed.
The original failed and false-green outcomes remain evidence.

RED `45be3abc854381b5bfdd5a03b5faeea33d572865` ran the actual smoke under
Bash 3.2.57 and observed no consumer calls and exit 0 where 43/44 were required.
The registry image digest was
`sha256:2556c2da19c32c1614d318645c73a7bf64d38be21cf69e98e3ca51f053983719`,
with network disabled and source mounted read-only. Portable positional native
arguments and a completion-aware cleanup trap now pass that same regression.
It covers write/reopen/trace, install failure, fatal nounset, and preservation
of the original failure when cleanup also fails. Bash 5 also passes.

Recovery RED `f2b8eee8b8a5086fcdbb6942a19b79d15560b4a6` demonstrated the
missing bounded recovery gate. Review REDs `737008cfbfe9ab4a6872e1ce1da661c513e0d2e5` and `9b06e2d066b6ec5b8b6df918aaf2cc48e1ea25a3`
additionally refused apparent green `.30` reruns, cleanup exit masking and
promotion after cancellation. All six recovery fixture tests now pass. The
actual read-only prerequisite verifier accepted the immutable tag and bound
original production attempt. It does not dispatch or mutate registry state.

Direct Astra-medium approved the design after one consolidated FIX binding
attempt 1 and Intel npm step 7. Direct Sol6.2-high approved final source after
the consolidated corrections. Existing npm promotion (4 tests), smoke-script,
release workflow scope, release contract and runtime alignment checks pass.
Affected Ruff, ShellCheck, actionlint, Markdown lint and diff checks pass;
structural shell literals use the existing SC2016 exclusion. New regression
suites are registered in the maintained fast runner. No duplicate full source,
native or GPU qualification was run for this narrow control repair.

The existing recovery workflow's selected `.30` mode requires fresh Linux and
both macOS Python consumers plus macOS ARM npm before pinned OIDC promotion.
The ordinary recovery path still requires all original successes for other
versions; `.30` cannot use its known-broken tag consumers as proof. Parent
integration and dispatch, actual recovered registry runtimes, final promotion
and all five GitHub Release wheel assets remain Slice 80/Terra delivery gates.
