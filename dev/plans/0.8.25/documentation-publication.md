---
title: FathomDB 0.8.25 documentation publication
status: IN_PROGRESS
target_release: 0.8.25
---

# FathomDB 0.8.25 documentation publication

## Goal

Make the completed 0.8.25 public documentation available at the repository's
GitHub Pages root without replacing the live `0.8.24+tegra` PEP 503 package
index already hosted under `/tegra/`.

This publishes documentation about the existing 0.8.25 release. It does not
move `v0.8.25`, alter package artifacts, or require 0.8.26.

## Existing evidence

- The post-release documentation update is complete at `9ef87489` on `main`.
- Strict MkDocs, documentation lint, Markdown lint, and surface scans passed for
  that update.
- GitHub Pages is already configured for workflow deployment.
- The live retained wheel is
  `fathomdb-0.8.24+tegra-cp310-abi3-linux_aarch64.whl` with accepted SHA-256
  `652ad6926b17c9580365b012ec9cb925fa1aabc6fe83047874c718dc5c5e5897`.

## Work

1. Build MkDocs and the retained Tegra index into one Pages artifact.
2. Add a main-triggered and manually dispatchable documentation deployment.
3. Make the existing opt-in Tegra publisher use the same combined-site path so
   a later Tegra deployment cannot remove the documentation.
4. Run only focused builder, workflow-contract, actionlint, shellcheck, strict
   MkDocs, and Git checks.
5. Push to `main`, observe the Pages deployment, and smoke the documentation
   root plus retained Tegra index.

## Completion criteria

- `https://fathomadb.github.io/fathomdb/` serves the 0.8.25 documentation.
- The Rust, Python, TypeScript, CLI, and 0.8.25 release-note pages are reachable.
- `https://fathomadb.github.io/fathomdb/tegra/simple/` still exposes the exact
  retained wheel and digest.
- No package registry or tag is changed.
