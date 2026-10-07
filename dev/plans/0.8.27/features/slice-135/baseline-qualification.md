---
title: FathomDB 0.8.27 Slice 135 — 0.8.26 baseline qualification
status: SOURCE_IDENTIFIED_NOT_MEASURED
target_release: 0.8.27
---

# 0.8.26 baseline qualification

Snapshot taken on 2026-10-06 for the [Phase 1 protocol](phase1-protocol-draft.md).
This identifies the source baseline; it is not a timing receipt or an
installed-artifact qualification.

| Item | Observed value |
| --- | --- |
| Baseline tag | `v0.8.26` |
| Peeled source commit | `f99e002f0d2e4002f3694c9f8d4986b56089edaa` |
| `Cargo.lock` SHA-256 | `ffebda90ad6bae977a63cdb7d4c8edbce88632c2d353d5251247410040a16022` |
| Isolated checkout | `/home/coreyt/projects/fathomdb-worktrees/release-0.8.26-slice-135-baseline`, detached at the tag commit |
| Source tree status | Clean at the snapshot |
| Local release artifacts | No wheel, npm tarball or crate package in that checkout |
| Python environment | No checkout-owned `.venv` in that checkout |
| Host readiness | Rust 1.95.0, Python 3.12.3, Node 26.8.2; `perf_event_paranoid=4`; approximately 61 GB free on the worktree filesystem |

The 0.8.26 tree has Python and TypeScript SDKs and the lower-level Rust
`fathomdb` facade, but no `fathomdb-sdk` crate. Therefore, pair equivalent
Rust **engine** operations across releases and label 0.8.27 Rust SDK calls as
candidate-only. Pair Python and TypeScript installed-SDK calls only where the
capability and timing boundary match. Do not treat the current 0.8.27 source
tree's pre-release `0.8.26` package version strings as candidate identity;
record exact source and built-artifact hashes.

Next: qualify built or published artifacts and the baseline-only noise pilot
under the frozen protocol. No sample count, percentile or performance
comparison is established by this snapshot.
