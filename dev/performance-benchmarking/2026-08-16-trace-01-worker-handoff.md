# TRACE-01 worker handoff

**Track:** `TRACE-01`

**Charter:** [TRACE-01 in PROGRAM](PROGRAM.md#trace-01--projection-lifecycle-integrity)

**Base:** `1b404101`
**Outcome:** implementation is ready for follow-up independent read-only review;
the full verification gate passes in the isolated worktree.

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
- Final focused command:
  `python3 -m pytest tests/experiments/test_trace_projection.py -q` —
  `10 passed`.
- Final required command: `./scripts/agent-verify.sh`, run unconfined only for
  AC-036 ptrace support. It passed: `agent-test.sh: 72/73 suites passed
  (skipped=1 excluded=0)`.
- Local-only prerequisite: the worktree has an ignored local `.venv` copied
  from the primary checkout, then `./scripts/earp-build-binding.sh` rebuilt the
  native extension into it. `sys.prefix` is
  `/tmp/fathomdb-trace-01-20260816/.venv`, and the imported extension is the
  local regular file `src/python/fathomdb/_fathomdb.abi3.so`. An ignored
  `node_modules` symlink supplies the Markdown tool. No network, receipt,
  corpus, model, GPU, or external service was used.

## Review follow-up

The first independent read-only review returned **REQUEST-CHANGES**:

1. Validate every sidecar field before writing rather than serializing an
   arbitrary mapping.
2. Restrict identifiers to a safe grammar.
3. Assert deterministic diagnostics, ordering, and serialized bytes.

Red checkpoint `2eefaed4` demonstrates the initial failures for source-text
identifiers and a payload injected into diagnostics. The follow-up implementation
must fail closed for both cases and pass the expanded synthetic test set before
a second independent review.

The second review found a lifecycle P1: `supersede prior → erase prior → reopen
prior` could make the prior searchable. Red checkpoint `759670a0` captures that
sequence and a sidecar with an active supersession prior. The follow-up makes a
superseded source irreversibly non-current and rejects the invalid written
sidecar. Focused evidence after that fix is
`python3 -m pytest tests/experiments/test_trace_projection.py -q` — `10 passed`.

An earlier full-gate attempt lacked a worktree-local binding; its diagnostics are
retained at `/tmp/fathomdb-agent-test-python-2805454.log`. The final passing
gate used the local-only prerequisite above and supersedes that environment
blocker.

## Review focus

1. Confirm no raw `prior_body`, `supersedes_hint`, or source text can reach the
   sidecar.
2. Confirm warning-only supersession is the sole accepted ELPS representation
   and competing edge fields fail loudly.
3. Confirm superseded and erased projections cannot remain searchable, while a
   valid re-open recovers only an erased source.
