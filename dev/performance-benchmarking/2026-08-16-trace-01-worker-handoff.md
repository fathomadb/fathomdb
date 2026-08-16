# TRACE-01 worker handoff

**Track:** `TRACE-01`

**Charter:** [TRACE-01 in PROGRAM](PROGRAM.md#trace-01--projection-lifecycle-integrity)

**Base:** `1b404101`
**Outcome:** implementation is ready for independent read-only review; the
full verification gate is blocked by the worktree's absent Python environment.

## Scope and ownership

- Owned paths: this handoff, the dated projection contract,
  `experiments/trace_projection.py`, and `tests/experiments/test_trace_projection.py`.
- Dependency consumed: accepted ELPS result-envelope warning `kind: supersedes`
  with `source_doc_id`, `prior_body`, and `supersedes_hint`.
- Produced contract: `trace-projection.v1`, a warning-only, payload-free
  lifecycle sidecar for the fixed synthetic fixture.
- No receipt, index, shared configuration, corpus, artifact, or external
  service was changed or used.

## TDD and verification evidence

- Red checkpoint: `ebb30ff0`.
- Exact red command: `python3 -m pytest tests/experiments/test_trace_projection.py -q`.
- Exact red result before implementation: collection stopped with
  `ModuleNotFoundError: No module named 'experiments.trace_projection'`.
- Passing focused command after implementation:
  `python3 -m pytest tests/experiments/test_trace_projection.py -q` —
  `5 passed`.
- Exact final command:
  `PATH=/home/coreyt/projects/fathomdb/.venv/bin:$PATH ./scripts/agent-verify.sh`.
- Final result: failed at `typecheck-python` (exit 1). Pyright reported
  `venv .venv subdirectory not found in venv path
  /tmp/fathomdb-trace-01-20260816`, then existing repository-wide unresolved
  `numpy` and `pytest` imports. The unaltered capped log is
  `/tmp/fathomdb-agent-typecheck-python-3978.log`.
- A documented, untracked `node_modules` symlink to the primary checkout was
  created only to clear the preceding markdown-linter prerequisite; no package
  was installed and no tracked receipt, configuration, or artifact changed.

## Review focus

1. Confirm no raw `prior_body`, `supersedes_hint`, or source text can reach the
   sidecar.
2. Confirm warning-only supersession is the sole accepted ELPS representation
   and competing edge fields fail loudly.
3. Confirm superseded and erased projections cannot remain searchable, while a
   valid re-open recovers only an erased source.
