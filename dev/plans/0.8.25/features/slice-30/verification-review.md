---
title: 0.8.25 Slice 30 independent verification
status: BLOCKED_P2
verified_commit: a20dd6d3
updated: 2026-09-04
---

# Slice 30 independent verification

Independent verification found no P1 defect and one blocking P2 contract gap.
Slice 30 remains open pending an owner-authorized correction beyond FIX-8.

## Blocking finding

`Engine::read_dependency_closure` validates the selected closure row but does
not validate the durable `_fathomdb_closure_sequence` singleton on the same
point read. Design v10 requires every point read to validate both and return
`Storage` on disagreement.

The verifier reproduced the gap from a fresh installed wheel. While the Engine
remained open, it changed `_fathomdb_open_state.value` to `0` for key
`_fathomdb_closure_sequence`, then read an existing completed closure. The read
returned `complete`; the required outcome is `Storage`. Reopen correctly detects
the corruption, so the gap is limited to same-open-Engine point reads.

## Passing evidence

- Unconfined fast verifier: 103/103 suites passed, including authorized ptrace.
- Full serial Rust workspace: passed.
- Heavy Rust workspace and TypeScript: passed.
- Focused schema migration: 2 passed.
- Focused Slice 30 Engine: 25 passed.
- TC-90 default target: 3 passed; 4 measurement arms intentionally ignored.
- PyO3 library: 11 passed; N-API library: 10 passed.
- Operator/test-hook Slice 30 route: 25 passed.
- Fresh release wheel build, isolated install, and closure lookup/error smoke:
  passed.
- Focused compiled TypeScript binding: 1 passed.
- Markdown, link, release-state-view, and diff checks: passed.

The heavy Python checkout suite did not collect because this worktree's `.venv`
resolves to the main checkout and loaded its stale native extension. The
repository guard correctly refused to rebind that shared environment. The fresh
wheel route above verifies the built artifact without altering the worktree.

## Not run

Windows-native verification was unavailable. CUDA is not applicable to Slice
30. A monolithic `--all-features` build is structurally invalid because CUDA and
Metal features are mutually exclusive; applicable CPU/operator feature routes
passed instead.
