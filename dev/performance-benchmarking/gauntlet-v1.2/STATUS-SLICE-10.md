# Performance Gauntlet v1.2 — Slice 10 status

**State:** COMPLETE  
**Slice:** 10 — user-facing entry point

## Reconciliation since the project plan

- The repository has no existing `run-gauntlet` entry point or gauntlet Python
  driver.
- Performance scripts use fail-closed Bash entry points, while experiment
  contract tests load Python scripts directly with `runpy`; the new entry point
  should follow both conventions.
- The user has added an explicit project outcome: after implementation, run the
  full default gauntlet on `v0.8.26` and remove only task-created temporary
  runs, worktrees, and branches.
- Slice 10 must not anticipate Slice 20 configuration validation, Slice 30
  command resolution, or Slice 60 execution. It owns the stable CLI surface,
  cell vocabulary, selection semantics, and a content-free dry-run projection.
- The project-plan example configuration belongs to Slice 20 because Slice 10
  deliberately treats `--config` as an opaque path.
- `scripts/agent-test.sh` enumerates experiment tests explicitly, so the new
  focused test must be added there or ordinary agent verification will omit it.
- The existing performance README is 0.7.0-specific and cites two parser files
  that no longer exist; the focused usage update should remove those stale
  layout entries without rewriting the rest of the historical guidance.

## Requirements and acceptance

1. Add a stable Bash entry point at
   `scripts/perf-experiments/run-gauntlet.sh` that delegates all logic to
   Python with the driver resolved from the script location.
2. Add `scripts/perf-experiments/run_gauntlet.py` with the default ten-cell
   ordered vocabulary: `ac076`, `ac072`, `ac081`, `ac073`, `ac075`, `scale02`,
   `protected-writes`, `ce-profile`, `search01`, and `locomo`.
3. Expose `--release`, `--source-root`, `--config`, `--output-root`,
   `--preflight-only`, `--dry-run`, `--cells`, `--resume`, and `--fail-fast`.
4. `--help` must not inspect the filesystem or external data.
5. `--dry-run` must emit a metadata-only JSON plan with the selected cells in
   stable order and must not create the output root.
6. `--cells` must split comma-separated input, trim whitespace, reject empty,
   unknown, and duplicate identifiers before canonicalization, and return
   selections in default order regardless of caller order.
7. `--dry-run` succeeds. `--preflight-only` alone, neither mode, or both modes
   must fail explicitly with exit code 2 until their owning slices land; none
   may appear to run a gauntlet.
8. No measurement, fixture, scoring, parsing, or threshold logic belongs in
   either entry-point file.
9. Update `scripts/perf-experiments/README.md` with the new CLI and initial
   mode behavior, deleting only the stale references encountered in its touched
   layout section.
10. Add `tests/experiments/test_perf_gauntlet.py` to the focused experiment
    test list in `scripts/agent-test.sh`; the new shell entry point must also
    pass the repository-wide shell lint.

## Design

- Keep the Bash file to `set -euo pipefail`, driver-directory resolution, and
  `exec python3 ... "$@"`. Resolve the driver from `BASH_SOURCE` without
  changing the caller's working directory, so relative arguments retain their
  caller-defined meaning.
- Keep the Python file import-safe and standard-library-only. Define the ten
  default cell IDs as one immutable tuple, parse selections into that canonical
  order, and serialize the dry-run projection with sorted JSON keys.
- Do not use a permanently required mutually exclusive mode group: final
  default execution intentionally uses neither mode. In Slice 10, manually
  reject both modes together, accept dry-run alone, and reject preflight-only
  or ordinary execution as not implemented.
- Require `--release`, `--source-root`, `--config`, and `--output-root` for
  operational invocations, but treat every value as an opaque string.
- Define the metadata-only dry-run schema as
  `fathomdb.performance-gauntlet.plan/v1`. It includes release, the three
  caller-provided path strings, ordered cells, `resume`, and `fail_fast`.
  Echo paths verbatim; do not hash, resolve, stat, create, or validate them.
- Return exit code 2 for invalid CLI or unsupported execution, matching common
  command-line usage errors.

## TDD plan

1. RED: tests prove default order, subset canonicalization, invalid selection
   rejection, all four mode combinations, help behavior, dry-run no-write
   behavior, and thin Bash delegation from a non-repository working directory.
2. GREEN: implement only the parser, projection, and wrapper needed by those
   tests.
3. REFACTOR: keep subprocess execution and configuration validation absent;
   those belong to later slices.

## Review and verification plan

- Obtain an independent design review before implementation.
- Obtain an independent read-only code review after GREEN.
- Have a separate verification subagent run the focused tests and direct CLI
  smoke checks.
- Record exact commands and results below, then mark the slice complete only
  when its acceptance criteria are proven.

## Design review

Independent read-only review returned **CHANGES REQUIRED**. The changes above
resolve its six findings: mode semantics, exact cell/argument contract,
metadata-only dry-run schema, wrapper working-directory behavior, README and
Slice 20 ownership, and ordinary test-runner integration. The resulting design
is approved for RED implementation.

## Evidence

- RED was captured before implementation: the focused test suite reported 13
  failures because neither entry-point file existed.
- GREEN: `PYTHONPATH=src/python:. .venv/bin/python -m pytest
  tests/experiments/test_perf_gauntlet.py -q` reports 13 passed.
- The ordinary experiment-harness command, including the new test file,
  reports 62 passed.
- Python compilation, Ruff, Bash syntax, and ShellCheck all pass for the new
  entry-point files.
- An outside-repository dry run preserved caller-relative path strings,
  restored a requested subset to canonical order, and did not create its
  output root. Direct checks also proved all ten default cells and exit code 2
  for neither mode, preflight-only, conflicting modes, and an unknown cell.
- Independent code review initially required three plan/status wording fixes.
  After those fixes, re-review returned **APPROVE** with no remaining findings.
- Independent verification returned **PASS** with no findings. The verifier
  created no branch or worktree and removed its temporary trace and bytecode
  state.
- `./scripts/agent-verify.sh` was attempted after the slice edit. It stopped in
  the lint tier at the pre-existing release-surface truth mismatch:
  `README.md lacks a current published 0.8.26 statement`. Slice 10 does not
  alter that unrelated root release documentation.
