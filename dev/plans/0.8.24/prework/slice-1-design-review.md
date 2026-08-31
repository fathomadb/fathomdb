---
title: 0.8.24 Slice 1 — feature and dependency design review
status: PROPOSED
target_release: 0.8.24
---

# Slice 1 — feature and dependency design review

## Review basis and authority

This review applies the owner’s Slice 1 direction to the existing 0.8.24 plan
and Slice 0 findings. It approves or adjusts **planning allocations**, not the
five unresolved owner decisions in the Slice 0 decision brief. Slices 2–5 have
not run, so there are no later-slice draft CRUD items to approve or reject yet.

## Feature/function allocation review

| Proposed item | Functions/surfaces reviewed | Disposition | Allocation |
| --- | --- | --- | --- |
| New CI workflow | `ci.yml` proportional classifier, `release.yml`, local release smoke scripts | **Approve, adjusted.** The CI work is already on main; do not recreate it as a release feature. | Slice 10 is assessment-only by default; no change unless a new target route has a demonstrated gap. |
| Engine performance | FTS query path in `fathomdb-engine`; retained SCALE-02 evidence branch | **Approve, conditional.** The streamed boundary-tie result is a real, owner-approved input but is unmerged. | Slice 20 as one reviewed integration decision with targeted correctness tests; no confirming benchmark. |
| Tegra public CUDA | `build-python-cuda-tegra.sh`, PyPI metadata/publisher route, compatibility docs | **Approve, adjusted.** Reject a same-name ARM64 CUDA wheel or PyPI `+tegra` upload. | Slice 30, only after an owner selects a separate public distribution identity and publisher. |
| Windows x64 CUDA | PyO3 wheel route, N-API platform package/loader, remote artifact route | **Approve, conditional.** Reject a local-Windows-build prerequisite and hosted-Windows-as-CUDA assumption. | Slice 40 after the owner approves a remote Windows CUDA executor and Python/npm surface. |
| Windows WAL review | Current Windows WAL jobs and the external Memex finding | **Approve, evidence-only.** Existing FathomDB diagnostics do not settle the Memex outcome. | Slice 50; no product change absent attributed evidence. |
| Target smokes and CPU preservation | Existing registry smokes, package matrix, idempotent publish guards | **Approve, strengthened.** These are release properties, not optional cleanup. | Slice 60; mandatory before Slice 70 evidence. |
| Broad dependency upgrades | Cargo, Python, TypeScript, root tooling, Actions | **Reject as a bundled release feature.** Upgrades must have individual compatibility/security evidence. | Slice 7 only for accepted contained maintenance; otherwise postpone. |

## Additional draft user needs

| ID | Need | Allocation |
| --- | --- | --- |
| N24-1 | A Jetson user must be able to select the CUDA package explicitly, without pip confusing it with the generic ARM64 CPU artifact. | 30 |
| N24-2 | A Windows user must not need to compile CUDA locally to use a supported CUDA artifact. | 40 |
| N24-3 | A maintainer must be able to retry a partial registry publication without replacing immutable valid artifacts. | 60 |
| N24-4 | A user installing a target-specific artifact must receive an installed-package proof on that target, not only a source-tree build result. | 60 |
| N24-5 | A maintainer needs security/tooling updates to be evidence-led, narrow, and reversible rather than silently accumulated while Dependabot PR creation is paused. | 7 |

## Additional draft requirements and acceptance criteria

| ID | Draft requirement | Draft acceptance criterion | Allocation |
| --- | --- | --- | --- |
| R24-8 | A public CUDA distribution identity must encode target selection outside ambiguous same-name wheel tags. | AC24-8: a clean target install uses the documented explicit distribution name; generic ARM64 CPU installation is tested separately. | 30, 60 |
| R24-9 | Windows CUDA support must have a declared remote executor and SDK matrix before any release artifact is claimed. | AC24-9: the evidence names runner labels, GPU/toolchain, artifact provenance, and the exact Python/npm support/unsupported matrix. | 40 |
| R24-10 | Existing CPU publish routes and retry behavior are invariants during CUDA work. | AC24-10: each affected CPU package is queried/installed and every publisher uses an existing-version no-op/fail-closed guard. | 60 |
| R24-11 | A dependency remediation may enter this release only when its current vulnerability/update evidence and blast radius are recorded. | AC24-11: the change is constrained to named manifests/locks, has a targeted check, and does not add a required full-CI or release gate. | 7 |
| R24-12 | Any Windows WAL behavior change must follow a comparison of the completed external client evidence with FathomDB’s installed Python path. | AC24-12: Slice 50 records reproduce / not reproduced / insufficient evidence before proposing a code change. | 50 |

These are approved design inputs for the plan, not edits to `dev/needs.md`,
`dev/requirements.md`, or `dev/acceptance.md`. Slice 3 owns any formal CRUD
proposal for those documents.

## Design rules

1. CPU and CUDA artifacts remain distinct and explicit; CUDA is never an
   implicit resolver preference.
2. CI remains informational, fast, and diff-scoped. Do not introduce required
   checks, merge queues, soak periods, or a release-wide full-CI ceremony for a
   narrow maintenance change.
3. A new distribution/executor is not real until its registry identity,
   trusted-publisher route, target provenance, and installed-package smoke are
   all recorded.
4. Dependency work is separated by blast radius. Security remediation of root
   Markdown tooling is contained; CUDA/runtime, database, binding, and model
   stack migrations are not bundled with it.
5. External evidence unavailable because of a service limit stays unknown. It
   must not be converted into an affirmative or negative product claim.

## Slice 1 implementation decision

The only currently recommended Slice 7 dependency item is a contained root
tooling remediation: update `markdownlint-cli2` from 0.23.0 to 0.23.2 and its
lockfile after the owner accepts it. Its declared dependency moves `js-yaml`
from 5.2.0 to 5.2.2, outside the two audit ranges observed in this slice. The
required proof is the root npm audit plus the existing AST-guarded Markdown
lint; no full release or hosted CI cycle is justified.

All other items remain postponed or need owner decision/evidence.

## Evidence

- [Slice 0 decision brief](slice-0-decision-brief.md).
- [Slice 1 library sweep](slice-1-library-sweep.md).
- `.github/dependabot.yml`, `Cargo.toml`, `src/python/pyproject.toml`, root
  and TypeScript package manifests/locks, and 2026-08-23 public metadata.
