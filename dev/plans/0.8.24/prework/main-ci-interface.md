---
title: 0.8.24 Slice 0 — main CI interface finding
status: COMPLETE
target_release: 0.8.24
---

# Main CI interface finding

**Observed:** 2026-08-23 from `origin/main` at
`f0712fa5c262505b7a849c17cb5d594acd9920cc`.

## Finding

The requested new CI work is already on `main`; it is not missing release
branch work. The relevant landing sequence is:

| Commit | Result |
| --- | --- |
| `23258cb2` | Proportional CI routing, a manual full-history Gitleaks workflow, and release-side advisory history scanning. |
| `0985cfcd` | Documents the first routing landing outcome. |
| `ee9bb753` | Warns before GitHub suppresses a workflow run from a skip marker. |
| `f0712fa5` | Preserves typed unattributed Windows WAL outcomes while checking that managed reader roles are inactive. |

`ci.yml` retains always-on baseline jobs and uses a `changes` classifier to
avoid unnecessary heavier jobs. Its scoped categories include Rust, Python,
TypeScript, the Windows WAL installed-attribution test path, CI workflow,
verification harness, security harness, Rust test harness, and native artifact
harness. An exact `[ci-lite]` marker is accepted only from a trusted source:
an owner/member/collaborator same-repository pull request, a direct push's
candidate commit, or the second parent of a merge push. It never applies to
`release.yml`, and incidental prose containing the marker does not activate
lite mode.

## Release interface

- `release.yml` remains a distinct, tag/dispatch release path. The CI routing
  change does not make a release dry run or publication conditional on an
  unrelated PR/full-tree ceremony.
- Release work must not edit `ci.yml` merely to reproduce the main redesign.
  Any later Slice 10 change needs a concrete uncovered release requirement.
- Shared risk surfaces are `.github/workflows/{ci,release}.yml`, release smoke
  scripts, package metadata, and compatibility documentation. Writers touching
  these surfaces must be serialized and worktree-isolated.
- The release branch is based on this current main commit. Before later code
  work is integrated, it must compare again with `origin/main`; it may not
  overwrite CI work that lands meanwhile.

## Slice 0 disposition

No CI change is proposed. Slice 10 begins by confirming the above contract
against the exact changed files; its default outcome is **no change** unless a
new Tegra/Windows route exposes a genuinely missing classification or proof.

## Evidence

- `git log 8ec9c60a..origin/main` on 2026-08-23.
- `.github/workflows/ci.yml` at `ee9bb753`.
- `git show 23258cb2 -- .github/workflows/ci.yml .github/workflows/release.yml`.
