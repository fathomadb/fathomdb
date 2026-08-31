---
title: 0.8.24 Slice 60 — preparation and change reconciliation
status: COMPLETE
target_release: 0.8.24
---

# Slice 60 preparation and reconciliation

## Goal

Prove the public, exact-version Tegra distribution as an installed package
without weakening existing CPU artifact or publisher preservation. This slice
adds no Windows feature: all Windows CUDA, VM, runner, smoke, and WAL work is
deferred to 0.8.26 by `seq-258`; existing Windows CPU release lanes remain
preserved inherited behavior.

## Upstream changes reviewed

| Source | Observed change | Slice 60 effect |
| --- | --- | --- |
| Slice 20 | FTS/WAL implementation and targeted independent verification are now closed on the release branch. | No change to package topology; consume through normal installed lifecycle. |
| Slice 30 | A Jetson Orin built `fathomdb==0.8.24+tegra` and deployed it to the authorized first-party Pages index in run `32878233246`. | Add the missing clean public-index install, using the retained artifact digest as a separate trust input. |
| Slice 40/50 | Explicitly deferred by the owner. | Remove new Windows CUDA/WAL proof from this slice; preserve legacy CPU paths only. |
| Current source | Python docs, CLI help, and the generic-build warning still claim Tegra has no published artifact. | Correct all three public-facing surfaces in the same change as the installed proof. |

## Approved target inventory

| Target | Identity/source | Slice 60 responsibility |
| --- | --- | --- |
| Python CPU | `fathomdb==0.8.24` / PyPI at owner publication time | Preserve existing publisher and post-publish smoke route; do not publish here. |
| Python Tegra | `fathomdb==0.8.24+tegra` / `https://fathomadb.github.io/fathomdb/tegra/simple/` | Fresh Jetson install from exactly this index, wheel-digest verification, lifecycle, and CUDA witness. |
| Cargo/npm/CLI CPU | Existing exact package/version routes | Preserve existing idempotency and smoke contracts; no net-new publisher semantics needed. |
| Windows CPU artifacts | Existing release workflow lanes | Preserve as inherited release behavior; no new Windows CUDA or SDK change. |

## Disposition

The previous draft is approved with one material adjustment: a public Pages
index is mutable interim hosting, so its link hash alone is not sufficient.
The smoke must compare the downloaded wheel to the SHA-256 recorded in the
retained Slice 30 Jetson evidence artifact before executing it. The artifact
archive digest and Pages deployment metadata are retained provenance, not a
substitute for the wheel digest or target-native execution.

## Prerequisites handed forward

Slice 70 receives the completed installed-index evidence plus the remaining
owner-only actions: branch integration, final gates, tag, publish, registry
queries, and normal post-publish smokes. It must not create a new Windows
support condition for 0.8.24.
