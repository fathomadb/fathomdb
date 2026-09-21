# Performance Gauntlet v1.2 — Slice 30A status

**State:** COMPLETE  
**Slice:** 30A — exact AC-076/AC-072/AC-081/AC-073 invocation map

## Current-state reconciliation

- Aggregate Slice 30 design is independently approved after two correction
  rounds. This sub-slice implements only its 30A runner-map boundary.
- The target checkout supplies product source/cwd. The gauntlet checkout
  supplies orchestration scripts, scanner, and validator identities.
- AC-072's dispatcher owns three repetitions and accepts 10 positional
  arguments after its path; AC-081's cell runner owns one process and is planned
  seven times.
- AC-073 can return a green Cargo test after a corpus SKIP, so planning binds the
  actual target `data/corpus-data`, all 13 corpus files and canonical identity,
  the frozen command, and the inherited embedder-cache file hashes.

## Acceptance

- Plans have the exact closed shape and remain JSON serializable.
- AC-076 delegates once with its historical environment and unset control.
- AC-072 delegates once to the campaign with six hashes and typed generated
  `perf_gates` placeholders; the legacy config is provenance-only.
- AC-081 has seven distinct raw logs/fresh invocations.
- AC-073 parses the frozen command, permits only output-root substitution,
  binds corpus/cache identities, and refuses missing/disconnected/drifted data.
- The planner imports no subprocess, numerical, retrieval, scoring, percentile,
  or bootstrap implementation.

## Evidence

- RED: the initial execution-map suite failed before the planner existed. A
  later adversarial `cargo --version` mutation was accepted before exact EU7
  argv validation and then failed as intended.
- GREEN: the focused Slice 30A suite passes with 19 tests.
- Independent code review: **APPROVE** after two correction rounds. Corrections
  pinned the corpus/model contracts, exact EU7 argv and manifest identity,
  effective timeout, and gauntlet-owned AC-081 scanner path/hash.
- Independent verification: **PASS**. The registered experiment boundary
  passes with 111 tests; Ruff, Python compilation, shell syntax, ShellCheck,
  and diff whitespace checks pass.
- Production-input probe: the actual 13-file corpus and three embedder-cache
  files match their frozen hashes, and the AC-073 plan serializes with the
  exact command, environment, identities, timeout, and completion controls.
- No branch or worktree was created for this slice; cleanup was unnecessary.
- The full repository gate was not rerun at closure because its known
  release-state/public-document and sandbox-only failures are outside this
  slice. The slice-owned and registered experiment boundaries are green.
