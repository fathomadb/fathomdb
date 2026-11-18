---
title: Slice 7 — Build, verification and delivery delays
status: COMPLETE
slice: 7
target_release: 0.8.30
date: 2026-11-18
---

# Slice 7 — Build, verification and delivery delays

Reviewed `18427b7a8`, retained .27/.28/.29 evidence and read-only GitHub job
summaries and focused logs on 2026-11-18. No build, verifier, scanner, runner
change, dispatch, publication or registry transaction ran in this prework.
[Master](../plan.md), [.27 delay review](../../0.8.27/prework/slice-7.md),
[.28 delay follow-up](../../0.8.28/release-ci-followup.txt),
[.29 retrospective](../../../notes/0.8.29-retrospective.md).

## From checkout through qualification

- **Preflight and build environment.** The planning checkout lacks its own venv. Ambient Node/npm
  differ from pins, and a version-matched Node binary may bundle a different npm. Historical heavy
  builds exhausted storage. The preflight disk floor is a generic guard, with no measured .30 build or
  cache-post peak. 10 selects exact checkout-owned tool/runtime paths and allocates real heavy-build
  space when commissioned. Keep missing setup an explicit preflight failure and keep owned
  storage/cleanup. Use the [Slice 0 inventory](slice-0.md) and
  [preflight](../../../../scripts/preflight.sh).

- **Fast bootstrap and static check.** .29 hosted fast bootstrap took about 8m 50s. Windows WAL
  workflow source checking took 856.43 s and later 579.34 s. .28 had already recorded roughly 30 min
  fast verification on a slow runner and substantial static harness/secret-guard costs. These
  observations have different runner conditions. 20 separates pinned tool setup from native
  construction where actual consumers permit. Profile the selected DAG and remove repeated
  parsing/walks from the [Windows source
  checker](../../../../scripts/tests/test_windows_wal_attribution_ci_job.sh). Measure old/new under
  comparable conditions and keep invalid-workflow mutations and real Windows WAL exercise.

- **Strict verifier and cancellation.** Hosted
  [44215258383](https://github.com/fathomadb/fathomdb/actions/runs/44215258383) cancelled both verify
  jobs. Rust returned 101 and the original diagnostic remains unavailable. A later scoped Rust pass did
  not reproduce the issue. Current CI gathers redacted spills on failed steps, while `run_capped` caps
  console text and uses wall-clock timing. 20 saves bounded actionable first failure before any later
  suite or cancellation, preserves fail-fast callers and actual passed/failed/skipped/unrun outcomes,
  and uses monotonic elapsed time. Exercise >200 lines then cancellation using harmless fixtures.
  Preserve scoped preflight and strict AC-036 ptrace. [CI](../../../../.github/workflows/ci.yml),
  [wrapper](../../../../scripts/lib/agent-output.sh).

- **End-of-ladder validation after publication.** In [44244789028
  T3b](https://github.com/fathomadb/fathomdb/actions/runs/44244789028/job/134601018880), the checker
  rejected explicit deferred 120 and unselected 150 because it required all rows LANDED. The [later
  fast job](https://github.com/fathomadb/fathomdb/actions/runs/44244789028/job/134601116052) completed
  with 216/219 passed in 1,587 s and uploaded nine redacted logs. `test-commission-manifest` rejected
  the same pair. `test-preflight-release-state` asserted all rows LANDED in the no-active-release
  regression. `test-slice10-preparation` found the capability manifest stale, still .28 despite .29
  publication. 20 uses a completed-state fixture with explicit authorized terminal dispositions, a
  negative unfinished selected row, and the real current closure shape. Correct the regression
  assumption as part of that scope. The published platform capability data should match verified
  delivery when reconciled by its release metadata owner. Preserve historical statuses and test
  positive/negative publish-state transitions before the .30 tag.
  [Checker](../../../../scripts/commission-manifest.sh), [capability
  manifest](../../../platform-capabilities.json).

- **Gitleaks and generated metadata.** .27 tracked a known tokenizer integrity digest and narrow
  registration. .28 separately recorded tracked source clean and generated metadata false positives for
  independently corroborated integrity digests. Its advisory history count drift had incomplete
  cross-run tuple revalidation. The earlier .29 hosted Gitleaks job succeeded. Evidence for a real new
  credential was not established. Retain current/staged scanner enforcement, redaction, exact known
  metadata treatment and advisory history scope. 20 investigates only demonstrated present
  failures/cost; 70 uses existing required security coverage. Keep raw sensitive material out of
  reports and preserve findings versus scanner/tool error. [Policy
  evidence](../../0.8.28/features/slice-95/qualification.md), [.29 delay
  register](../../0.8.29/prework/slice-7/delay-register.md).

## After product bytes exist

- **Route, runner and packaging.** .27 had exact-tag GPU authorization issues, and .28 delivered
  through scoped recovery. .29 worker 168 claimed real production work before the prepared launcher. A
  distinct Tegra wheel build failed when staging omitted the root Cargo source config. The checked-in
  copy repair `86d5f376d` is present. 10 verifies effective route/tool identity and available venue. 20
  owns least-scope route and consumer fixes. 70/80 bind actual producer and artifact bytes and verify
  matching live workers before an authorized dispatch. Retain the packaging fix rather than rebuild a
  closed .29 case. [Tegra builder](../../../../scripts/release/build-python-cuda-tegra.sh).

- **Exact registry visibility and npm install.** Core
  [44229162869](https://github.com/fathomadb/fathomdb/actions/runs/44229162869) reached successful
  publishers and co-tagging. Original npm install returned success without its optional native binary.
  Targeted retry succeeded. The missing binary is established, while propagation and resolver behavior
  remain hypotheses. Exact Cargo visibility polling already exists. 20 preserves existing tier/version
  polling and adds useful expected/observed visibility, npm resolution, optional versions and expected
  binary diagnostics. Bound retries to observed visibility. Keep runtime/data failures red and keep
  original attempts. [Exact Cargo polling](../../../../scripts/release/wait-for-crate-version.sh).

- **Finalization.** Normal `release.yml` still uses a legacy token for latest and .29 retained E401
  after publishers succeeded. OIDC attempt one received PUT 204 and npm exit zero, then failed an
  immediate read whose returned value was omitted. The later
  [44233820520](https://github.com/fathomadb/fathomdb/actions/runs/44233820520) job list confirms
  successful OIDC promotion and GitHub Release. Latest-attempt job metadata does not expose the
  original failed read. 20 shares the already qualified pinned OIDC implementation between normal and
  recovery, with exact trusted routing, target verification and bounded visibility polling that logs
  expected/observed values. 80 checks actual normal success. Separate a known mutation from its
  visibility and avoid reinvoking successful publishers. [Normal
  workflow](../../../../.github/workflows/release.yml), [recovery
  workflow](../../../../.github/workflows/promote-npm-release.yml).

- **Consumer strength.** Public Rust SDK smoke currently installs the exact crate and opens/closes. The
  public Python/Node flows include some real writes, native loading and selected interface checks, with
  no shared persisted-hit plus missing-record and separate-process reopen oracle. Their successes
  qualify what the actual jobs exercised. 20 makes exact-version public SDK/Python/Node flows assert
  persisted write/read, missing record, clean independent reopen and exit, with local negatives. Keep
  platform-specific native resolution. 70 tests exact candidate bytes and 80 requires the authorized
  real registry job results. [SDK](../../../../scripts/release/smoke/smoke-crates-sdk.sh),
  [Python](../../../../scripts/release/smoke/smoke-pypi-wheel.sh),
  [Node](../../../../scripts/release/smoke/smoke-npm-package.sh).

- **Supplemental Tegra.** Read-only query of
  [44246092920](https://github.com/fathomadb/fathomdb/actions/runs/44246092920) on 2026-11-18 showed
  seven successful jobs. [Public URL
  verification](https://github.com/fathomadb/fathomdb/actions/runs/44246092920/job/134607953884) and
  [installed Orin
  consumers](https://github.com/fathomadb/fathomdb/actions/runs/44246092920/job/134608027048) are
  positive job evidence. The docs/manifest fix `42629c2a1` includes both artifacts. The .29 state still
  says preflight. .29 owner reconciles state to verified delivery without repeating the run. 70/80
  reuse the correct previous delivery disposition and still require the applicable .30 artifacts.
  [Slice 6 URLs and limits](slice-6.md#verified-delivery-and-remaining-record-drift).

## Decision input

Prioritize bounded failure retention, truthful closure and post-build normal
publication/consumer fixes in 20. Select owned tools and artifact setup in 10.
Measure the static-check and pinned-tool savings before broader rerouting.
Retain existing registry tiering, security gates, supported native matrix,
and exact source/build/artifact linkage. Keep verification order lint,
typecheck, security and affected tests as implemented by the strict gate.
Use one coherent final developer gate with separate feature, long,
platform, and registry execution evidence. [Feature 20](../features/slice-20/plan.md),
[qualification 70](../features/slice-70/plan.md),
[delivery 80](../features/slice-80/plan.md).
