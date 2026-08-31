---
title: 0.8.24 Slice 5 — verification-adequacy review
status: COMPLETE
target_release: 0.8.24
---

# Slice 5 — verification-adequacy review

**Observed:** 2026-08-23. This is a source-inspection review of the existing
test and release controls. It adds no test, workflow, product, contract,
registry, runner, or release-state change. “Existing” below means that the
named file asserts the stated property; it does not turn a structural check
into target-hardware or installed-package evidence.

## Verdict

The established CPU/release controls are substantial: version consistency,
publisher idempotency, native CPU artifact topology, CUDA rehearsal integrity,
and Windows WAL attribution controls all have named local contract tests. They
are insufficient to release the proposed new Tegra and Windows CUDA routes.
Those routes require decisions, real target executors, artifact provenance,
and fresh installed-package smokes. The remaining material documentation gap
is the contradiction between atomic-publish wording and existing retry-safe
mechanics.

No proposed 0.8.24 requirement lacks a falsifiable acceptance signal. Several
signals are deliberately **proposed**, rather than claimed as passed, because
the target package identities, executor contracts, and external evidence are
not yet available.

## Traceability matrix

| Planning requirement(s) | Falsifiable acceptance signal | Existing evidence checked | Gap / required proof | Environment and primary owner |
| --- | --- | --- | --- | --- |
| R24-1, R24-8; proposed `REQ-TARGET-TEGRA` / `AC-TARGET-TEGRA` | A resolver-visible separate public Tegra identity installs only when explicitly selected; generic Linux ARM64 CPU installation remains distinct; installed lifecycle and GPU-selection evidence identify the candidate. | `scripts/release/build-python-cuda-tegra.sh` and `scripts/tests/test_cuda_release_contract.sh` prove only the intentionally local `+tegra` build/rehearsal contract. `scripts/tests/test_jetson_tegra_cuda_evidence_ci_job.sh` checks the retained evidence-workflow shape. | Select/register the public identity and trusted publisher; prove resolver selection and generic-CPU separation; retain a Jetson artifact SHA/version/toolchain/GPU witness and a clean installed open/write/search/close/exit smoke. Local `+tegra` and workflow-shape tests cannot prove a public project or Jetson run. | Slice 30 owns identity/contract; Slice 60 owns target-installed smoke. Jetson plus registry. |
| R24-2, R24-9; proposed `REQ-TARGET-WINDOWS-CUDA` / `AC-TARGET-WINDOWS-CUDA` | An owner-declared Python/npm support matrix and remote Windows CUDA executor yield a provenance-bound supported artifact; unsupported routes are clear; installed lifecycle smoke passes without local compilation. | `.github/workflows/release.yml`, `src/ts/src/platform.ts`, and `scripts/tests/test_native_artifact_runtime_validation.sh` establish existing Windows **CPU** native-artifact behavior. | Name and observe the remote GPU executor, toolkit/driver/compiler/GPU and transfer boundary; decide Python, npm, or both; prove artifact/loader selection, unsupported behavior, and an installed Windows CUDA smoke. Hosted Windows CPU evidence is not CUDA evidence. | Slice 40 owns contract/artifact; Slice 60 owns installed smoke. Remote Windows CUDA. |
| R24-3, R24-10 | Every existing CPU package remains name/version/installable; optional-dependency wiring and ABI floors remain valid. | `scripts/tests/test_linux_aarch64_release_artifacts.sh` asserts the stable platform package set and ARM64 route. `scripts/tests/test_native_artifact_runtime_validation.sh` asserts the five native triples and locally consumes both artifacts. `scripts/tests/test_cuda_release_contract.sh` includes isolated CPU and CUDA rehearsal checks. | Extend the exact package/version/install matrix to the release candidate and include every affected CPU Python/npm/CLI lane when a target route changes. Existing source/rehearsal checks are not registry-installed CPU evidence. | Slice 60; target registries and relevant native hosts. |
| R24-4, R24-10; proposed `AC-PUBLISH-RETRY` and `REQ-050`/`AC-054` update | An existing valid artifact is skipped; a registry query error fails closed; a missing artifact remains publishable on retry; release completion is refused until required artifacts are present and verified. | `scripts/release/{pypi,npm,cargo}-publish-if-new.sh`; `scripts/tests/test_idempotent_republish.sh`, `test_publish_registry_safety.sh`, and `test_pypi_publish_roundtrip.sh` exercise the existing-version and fail-closed controls. | Reconcile canonical atomic wording with these mechanics; add/retain per-publisher injected existing/query-error/missing-target coverage and release-completion evidence for the selected artifact set. A successful upload alone cannot prove safe retry. | Slice 60; local fixtures plus registry query/publish path when authorized. |
| R24-5; existing `REQ-010` / `AC-076` | The approved SCALE-02 decision rule is satisfied by retained evidence and the integrated engine change preserves exact-result/correctness constraints. | `dev/plans/0.8.24/prework/benchmark-evidence-index.md` records the branch, 60 repetitions, result constraints, and owner-selected streamed-boundary result. `dev/acceptance.md` assigns existing text-latency coverage to `perf_gates::ac_012_text_query_latency_on_fts5_path`. | Inspect the complete unmerged delta, retain the decision-rule provenance, add RED-to-GREEN targeted correctness/regression proof for the integrated behavior, and run no speculative confirming benchmark. The retained result is not a test of a later integration. | Slice 20; local targeted tests, retained benchmark evidence. |
| R24-6, R24-12; `NEED-020a` | The linked external client outcome is classified reproduce, not reproduced, or insufficient evidence after comparison with the installed FathomDB Python path; no code change is proposed before that disposition. | `scripts/tests/test_windows_wal_attribution_ci_job.sh`, `src/python/tests/test_slice65_wal_attribution_installed.py`, `src/python/tests/test_slice65_wal_attribution_typing.py`, and the Windows WAL controls in `.github/workflows/ci.yml` establish FathomDB-side attribution controls. | Obtain the actual completed Memex job logs/artifacts, preserve its client/package/environment identity, and compare it with the FathomDB installed-wheel control. Existing controls cannot attribute the external client result. | Slice 50; linked Memex evidence and Windows comparison environment. |
| R24-7 | Current main CI preserves proportional routing and fast/heavy job ownership; any added target route has an explicit classifier, transfer, and proof need. | `scripts/tests/test_ci_proportional_routing.py`, `test_ci_long_job_efficiency.sh`, and `test_bootstrap_heavy.sh` check the routing and heavy-bootstrap contract. `5e2a05e2` is the current main-owned fast/heavy landing. | Slice 10 begins from current `origin/main`, compares release-interface needs, and adds a route only for a concrete uncovered target proof. Local structural checks and `actionlint` cannot establish hosted executor availability. | Slice 10; current main plus targeted local workflow checks. |
| R24-11 | A named dependency/tooling remediation has current vulnerability/version evidence, bounded manifests/lockfiles, and a targeted green check without manufacturing a hosted release gate. | Slice 1 recorded the root `markdownlint-cli2` → `js-yaml` advisory path, exact candidates, and pinned-package rationales. The existing guarded Markdown check protects the supported Markdown path. | If accepted, capture the failing audit/baseline, update only the identified root dependency/lockfile, and prove the named advisory path plus guarded Markdown tooling. Pyright/Ruff/Dependabot changes remain separate owner choices. | Slice 7; local tooling. |
| R24-13 | Maintained public docs/site configuration/shipped READMEs contain neither a former-owner link nor a stale current-release assertion, except intentionally historical records. | Slice 2's bounded scan identified maintained `docs/**`, `mkdocs.yml`, `src/python/README.md`, `src/ts/README.md`, and package READMEs as affected. | Define the maintained-path allowlist and historical exclusions, demonstrate the stale baseline, then use text/link/build checks appropriate to changed files. A broad repository URL replacement would damage historical evidence. | Slice 7; local docs tooling. |
| R24-14 | Active engineering navigation resolves the current program and release state consistently through the release-state lookup contract. | Slice 2 checked the conflicting current wording in `dev/README.md`, `dev/plans/README.md`, and `dev/DOC-INDEX.md`. | Define the active-navigation set, assert the lookup contract and index consistency after a bounded correction. Do not create a live release-state file merely to satisfy a documentation check. | Slice 7; local docs/index checks. |
| R24-15 | Any proposed archive/delete names retention or replacement and passes tracked-code-read, inbound-reference, and targeted verification checks. | Slice 2 found no qualifying archive/delete candidate and identified retained roles for every reviewed path. | No change is proposed. If a later concrete candidate appears, it needs all stated evidence before it can enter a later Slice 7 decision. | Later only; no current owner. |

## Critical-path coverage

| Critical path | Success evidence | Failure / negative evidence | Verdict and allocation |
| --- | --- | --- | --- |
| Existing CPU Python/npm/CLI artifacts survive target work | Native CPU topology and local runtime-consumption checks exist in `test_linux_aarch64_release_artifacts.sh` and `test_native_artifact_runtime_validation.sh`. | CUDA rehearsal tests assert driverless CPU behavior and reject malformed route evidence. | **Partially adequate.** Candidate-version registry installs for every affected CPU lane must be added to Slice 60. |
| Jetson explicit public CUDA install | Local-only Tegra package/rehearsal and evidence-workflow contract are checked. | Local `+tegra` is deliberately non-public; no public project/runner witness exists. | **Absent target proof.** Blocked on identity and observed Jetson route; Slices 30 then 60. |
| Windows CUDA installed artifact | Windows CPU package/runtime controls exist. | Existing workflow matrix has no CUDA executor or loader identity. | **Absent target proof.** Blocked on owner-selected SDK matrix and remote executor; Slices 40 then 60. |
| Retry-safe multi-registry publication | Publisher helper and fixture tests establish skip-if-existing and fail-closed behavior. | Fixture tests cover registry-safety/error paths. | **Partially adequate.** Canonical release completion wording and full selected-target evidence belong to Slice 60. |
| Engine performance integration | Retained SCALE-02 result and decision constraints exist. | No integration-specific regression proof yet; no confirmatory run is required. | **Partially adequate.** Slice 20 must test the integrated delta against the declared correctness constraints. |
| Windows Python-SDK WAL behavior | FathomDB source/installed attribution controls exist. | Actual external client result is unavailable; current controls must not be overread. | **Unknown by design.** Slice 50 obtains evidence and records a durable disposition. |
| Main CI without ceremony | Proportional-routing and long-job contract tests exist on main. | Structural tests cannot prove a newly required target executor or hosted behavior. | **Adequate for existing CI; conditional for new routes.** Slice 10 owns any demonstrated narrow change. |
| Slice 7 maintenance | Each candidate has an identified bounded local proof. | No candidate is authorized or may become a feature substitute. | **Contingent.** Slice 6 owner decisions determine the accepted packages. |

## Test-quality and provenance requirements

1. Code behavior introduced by Slices 20, 30, 40, 50, or 60 requires a
   meaningful failing test first and targeted GREEN proof. Packaging/loader or
   unsupported-route behavior is public behavior, not merely a workflow-text
   condition.
2. Target evidence must bind candidate SHA, package identity and version,
   artifact digest, target, OS/architecture, executor label, GPU/toolkit/driver
   facts where relevant, and the installed-package smoke output. Artifact
   upload, a runner label, or source-tree compilation is insufficient.
3. An installed smoke is a fresh target-native install followed by the
   documented open/write/search/close/exit lifecycle. It is not a wheel build,
   `pip install -e`, or an import-only test.
4. Publisher tests must cover the no-op existing-version path and uncertain
   registry state separately. The latter must fail closed, and a valid existing
   artifact must not be replaced.
5. The external WAL finding needs raw/job-derived evidence. A FathomDB control
   test establishes the comparison method, not the external client conclusion.

## Explicit limits of this review

This review did not and cannot prove trusted-publisher configuration, public
project availability, registry authorization, self-hosted runner online state,
Jetson or remote Windows CUDA capability, GitHub environment approval, or the
Memex job outcome. Those are external facts to be recorded by their owning
feature slices, not weakened into local substitutes.

## Allocation summary

- **Slice 7:** only owner-accepted, bounded root-tooling and documentation
  maintenance (R24-11, R24-13, R24-14); R24-15 has no candidate.
- **Slice 10:** current-main CI/release-interface assessment; retain a
  no-change presumption absent a demonstrated target-routing gap.
- **Slice 20:** retained performance decision plus integration correctness
  proof, without a speculative benchmark rerun.
- **Slice 30 / Slice 40:** target identity/executor/artifact design and its
  public-contract consequences.
- **Slice 50:** external WAL evidence attribution and disposition before any
  product change.
- **Slice 60:** target-installed smokes, CPU preservation, publisher retry,
  and release-semantics contract alignment.

## Evidence inspected

- `dev/{needs,requirements,acceptance,test-plan}.md` and
  `dev/design/release.md` for the authoritative current contract.
- Slice 0–4 records in `dev/plans/0.8.24/prework/`, including the benchmark,
  executor, publication, and main-CI findings.
- `.github/workflows/{ci,release,jetson-tegra-cuda-evidence}.yml`.
- `scripts/tests/test_{ci_proportional_routing,ci_long_job_efficiency,bootstrap_heavy,cuda_release_contract,cuda_package_rehearsal,idempotent_republish,publish_registry_safety,pypi_publish_roundtrip,jetson_tegra_cuda_evidence_ci_job,windows_wal_attribution_ci_job,linux_aarch64_release_artifacts,native_artifact_runtime_validation}.sh`.
- `src/python/tests/test_slice65_wal_attribution_{installed,typing}.py` and
  the release helpers cited in the traceability matrix.
