---
title: 0.8.24 Slice 0 — CI and release controls finding
status: COMPLETE
target_release: 0.8.24
---

# CI and release controls finding

## Current controls

- `release.yml` has repository-level `id-token: write`; individual npm and
  PyPI publisher jobs use OIDC. PyPI publication is bound in source to the
  `pypi` GitHub environment.
- CUDA x64 preflight/rehearsal is intentionally a self-hosted Linux route.
  It produces fail-closed evidence bundles before canonical CUDA artifacts.
- PyPI/npm/crates publishing includes idempotency guards. Post-publish smokes
  are real registry installs and are deliberately skipped for a dry run.
- Existing Windows CPU packaging publishes a Windows platform npm package on
  `windows-latest`; it is not a CUDA build route.
- Main CI now uses proportional routing, not an assumed full workflow for every
  administrative change. This is compatible with the owner’s stated
  low-ceremony policy.

## Windows WAL relationship

`ci.yml` already has two hosted-Windows jobs:

| Job | What it establishes | What it does not establish |
| --- | --- | --- |
| `windows-wal-checkpoint-diagnosis` | Cross-process engine WAL checkpoint diagnosis. | A release-package smoke or the external Memex client finding. |
| `windows-wal-attribution` | Source and installed-wheel attribution controls, including a released 0.8.22 wheel and current disposable test-hooks wheel. | The conclusion of Memex job `32587291032/97065598178`. |

The newest main commit, `f0712fa5`, additionally retains typed unattributed
BUSY outcomes in the retained-result control while proving no managed reader
role remains active. That narrows FathomDB’s own diagnosis, but still does not
establish the external Memex client outcome.

The Memex job could not be queried because the authenticated GitHub API is
rate-limited. Slice 50 must retrieve its completed logs/artifacts after the
limit is available and compare its actual client path with these FathomDB
controls. It must not import an outcome from the job title or from FathomDB’s
existing tests.

## Minimal CI-change rule

Slice 10 starts with no presumed change. A new workflow/route is justified only
when the separately identified Tegra or Windows executor needs a selector,
artifact transfer, smoke, or path-classification that the existing workflow
does not provide. Any such change belongs on current `main`, not as a rewrite
of release-branch CI work.

## Evidence

- `.github/workflows/ci.yml:117-250, 612-931`.
- `.github/workflows/release.yml:1-35, 336-925, 1185-1525`.
- `scripts/release/cuda-preflight.sh` and `cuda-package-rehearsal*.sh`.
