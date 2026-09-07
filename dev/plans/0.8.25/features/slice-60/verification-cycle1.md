---
title: 0.8.25 Slice 60 independent verification — cycle 1
status: FAIL
candidate: a9be46edfd64533599ff8d6b8c8e3bf15f179a0b
---

# Slice 60 independent verification — cycle 1

## Verdict

**FAIL.** The exact clean candidate fails mandatory fast-gate security check
AC-050a because the public `test-hooks` constructor
`legacy_unverified_degraded` begins with the forbidden `legacy_` compatibility
prefix. Restricted, unchanged unconfined, and direct AST-scan executions all
report the same single finding in `graph_expand.rs`. AC-036 ptrace and AC-037
network-namespace checks pass unconfined, so this is not sandbox noise.

The minimum owner is Slice 60 test-hook naming and its one integration-test
caller. Heavy, all, complete-check, parallel-report, and Windows verification
were stopped fail-closed pending correction. No Windows archive or transfer was
created during this failed cycle.

## Passing evidence before the blocker

- All 15 explicit Slice 60 engine binaries pass 50/50 under
  `test-hooks,operator` and under the applicable default-embedder/default-
  reranker feature combination.
- Slice 20 graph, Slice 35 frozen/frontier, and Slice 55 trace/integrity
  compatibility pass 110 tests; one documented Slice 55 release-performance
  test remains intentionally ignored in the focused route.
- Slice 55/60 facade and governed-surface tests pass 4/4.
- The exact-source Linux wheel has SHA-256
  `3428dbe7aa6c0d49a9841c0447246ce7c181ff1a33b80d6536ac1e7bfef4a10f`;
  a fresh isolated import passes 24/24 Python tests.
- The Linux N-API module has SHA-256
  `3c82600055806fc9a4c75904c8965f1f7d6481ae4fe283d89e3bb7549eac00d6`;
  four explicit Node modules pass 15/15 with no skip, cancel, or todo.
- Workspace fmt, strict workspace all-target Clippy/check, strict applicable-
  feature engine Clippy/check, release-state views, Markdown/design/findings/
  reference checks, and `git diff --check` pass.
- Query seeding does not enter embedding, KNN, or dense-device dispatch; GPU
  remains correctly inapplicable to Slice 60.

## Environment and cleanup

Restricted N-API process spawning required an unchanged unconfined retry. The
N-API command's external output-directory copy and the first isolated Node
harness omitted required layout files; using the already compiled exact module
and copying the governed allowlist corrected only the disposable harness.

The verifier removed 67 MiB of owned temporary artifacts. The durable Cargo
target grew from 16 GiB to 22 GiB and remains as a mixed reusable cache. Final
free disk was about 151 GiB. The worktree remained clean, no verifier process
remained, and no ref, push, merge, release package, registry, tag, or
publication action occurred.
