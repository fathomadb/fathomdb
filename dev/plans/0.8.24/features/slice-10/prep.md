---
title: 0.8.24 Slice 10 — current-main CI preparation and evidence
status: COMPLETE
target_release: 0.8.24
---

# Slice 10 — current-main CI preparation and evidence

**Observed:** 2026-08-24 in the isolated `release/0.8.24-slice10`
worktree.

## Goal and execution baseline

Slice 10 closes the existing interface between informational main CI and the
explicit release workflow. It does not design Tegra or Windows CUDA routes and
does not wait for Slices 30 or 40 to do so.

| Fact | Evidence |
| --- | --- |
| Current remote main | `5e2a05e281571024a3e7bb305373915597a54078`, committed 2026-08-23T10:54:52-05:00 |
| Release-planning tip at start | `cf7fd1fe819f1155997863e468d5d07eaef25cdb` |
| Ancestry | `5e2a05e2` is the merge base and an ancestor of the Slice 10 branch. |
| Later relevant main work | None: `origin/main` is exactly `5e2a05e2`; the log after it is empty. |
| Release-branch workflow delta | None in `ci.yml`, `release.yml`, `bootstrap-heavy.sh`, or the three core CI contract tests. Slice 7 changed only general `bootstrap.sh` messaging/tooling among the inspected CI/script paths. |

## Assigned inputs and dispositions

| Input | Review result | Slice 10 disposition |
| --- | --- | --- |
| P24-12 | Owner accepted current-main ownership and the no-change presumption. | **Accepted.** No release-branch recreation and no ceremony run. |
| R24-7 | CI must be compatible with release topology. | **Accepted locally.** Existing CI and release triggers remain separate and structurally compatible. No canonical product requirement is created. |
| A24-5 | No architecture change absent a demonstrated route gap. | **Accepted.** Current implementation is aligned; no architecture CRUD. |
| Slice 4 CI alignment | Proportional routing already exists on main. | **Confirmed from code and executable contracts.** |
| Slice 5 CI proof | Structural tests cover the existing interface but not target hardware. | **Confirmed.** Target proof remains with the target slices. |

## Relevant current-main evolution

| Commit | Change assessed | Current disposition |
| --- | --- | --- |
| `23258cb2` | Added proportional path classification, trusted `[ci-lite]`, manual history scanning, and release-side advisory history scanning without rewriting heavy job bodies. | Retain. The executable routing fixture covers exact matcher behavior and job conditions. |
| `0985cfcd` | Recorded the first routing landing outcome. | Documentation evidence only; no implementation delta to integrate. |
| `ee9bb753` | Added a pull-request warning before GitHub suppresses a workflow through a skip marker. | Retain; it does not alter release triggers. |
| `f0712fa5` | Preserved typed unattributed Windows WAL outcomes while requiring managed roles to be inactive. | Retain; Slice 50 owns external-client attribution. |
| `5e2a05e2` | Added dependency caches, removed one-off Ripgrep setup, split heavy dependencies into `bootstrap-heavy.sh`, made BGE skip evidence visible, and registered mutation-sensitive contract tests. | Retain. Local tests prove the intended fast/heavy ownership. |

No related main change landed after the Slice 10 plan was written.

## Function and ownership inventory

| Function | Current owner | What exists now | Net-new work here |
| --- | --- | --- | --- |
| Ordinary PR/main feedback | `.github/workflows/ci.yml` | PR and main-push triggers; current-tree security and low-cost independent checks; proportional heavy jobs. | None. |
| Change selection | `ci.yml` `changes` | Rust, Python, TypeScript, Windows-WAL, CI, verifier, Rust-test, security, and native-artifact categories plus trusted exact-marker lite mode. | None. |
| Fast/heavy verification | `verify-fast`, `verify`, `scripts/bootstrap-heavy.sh` | Fast developer-tooling ownership and heavy-only Python/TypeScript dependency setup. | None. |
| Existing platform/WAL evidence | Windows WAL and native-artifact CI jobs | Hosted Windows attribution plus five CPU-native artifact matrix. | None; this is not Windows CUDA evidence. |
| Release rehearsal/publication | `.github/workflows/release.yml` | `v*` and explicit dispatch triggers, candidate/tag selection, artifact build/publish/smoke graph. | None. |
| Existing self-hosted CUDA route | Release preflight/rehearsal jobs | Main-owned control plane, main-ancestor candidate, read-only candidate checkout, environment-bound labeled runner, same-run receipts. | None; this proves only the existing Linux x64 CUDA route structure. |

## Assessment result

There is no missing selector, dependency edge, or test in the **existing**
main-CI/release interface. Adding a placeholder Tegra or Windows CUDA route
would pre-decide package identity, executor, artifact topology, and smoke
ownership that belong to later slices. The implementation outcome is therefore
documentation-only and no-code.
