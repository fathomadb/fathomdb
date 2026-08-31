# SCALE-01 TC-5 execution-preparation worker handoff

**Date:** 2026-08-16

**Worker branch:** `experiments/performance-scale-exec-20260816`

**Base:** `378a8214041033891d6a1e66fae8dcf9f8cc9681`

**Final implementation commit:** `728166dc867a39ae706f44d1fd83b232d9481a9c`

## Scope delivered

- `experiments.tc5_characterization`: a preparation-only release boundary that
  composes, but does not modify, `experiments.tc5_manifest`.
- `experiments/configs/scale-01/tc5-execution.v1.json`: frozen, disabled CPU
  configuration for the 7,667-document bridge plus 18,472-document primary
  arm.
- Human-intended tests for release-state refusal, frozen CPU pins, qualified
  external manifest/root requirements, historical EU7/repository rejection,
  and safe external receipt projection.
- [Execution and remediation contract](2026-08-16-scale-01-execution-remediation-contract.md):
  the separately authorized current-configuration diagnosis protocol.

## Red-first evidence

`eb1ab252c70c0a2175e252a04f442cf57bd4eaaa`
(`test(perf): specify tc5 execution release boundary`) was committed before the
module existed. Its focused test command stopped at the intended red failure:
`ModuleNotFoundError: No module named 'experiments.tc5_characterization'`.

`728166dc867a39ae706f44d1fd83b232d9481a9c`
implements the contract and turns that suite green.

## Verification

- `python3 -m pytest tests/experiments/test_tc5_characterization.py tests/experiments/test_tc5_manifest.py -q`
  — 20 passed.
- `python3 -m ruff check experiments/tc5_characterization.py tests/experiments/test_tc5_characterization.py`
  — passed.
- `./scripts/agent-lint-md.sh` — passed after the documented linked-worktree
  `node_modules` setup.
- `git diff --check` — passed.
- `./scripts/agent-verify.sh` — passed.

## No-live assertion

This worker did not acquire a corpus, read corpus payloads, invoke an
embedder/model/EU7, run smoke or benchmark work, write a campaign artifact,
make an external request, use a paid service, or push. The only receipt write
coverage uses test-managed temporary directories. The committed configuration
is explicitly disabled, and `run_characterization` refuses execution pending
independent review and coordinator release.

## Reviewer focus

1. Confirm this worker leaves `tc5_manifest.py`, historical EU7 code/output,
   `experiments/index.jsonl`, shared helpers, PROGRAM, and Track Runner state
   unchanged.
2. Confirm both arms and all CPU/model/`K=192`/query/bootstrap pins fail closed
   on drift, and receipt projection contains no raw path, document ID, corpus
   text, metric, or SCALE-02 claim.
3. Confirm the remediation distinguishes a ground-truth diagnosis from a 0.90
   pass assertion and labels every incomplete or changed-variable result
   inconclusive.

## Next gate

Independent read-only review of `728166dc` (with the red checkpoint
`eb1ab252`), followed by coordinator-only integration. No execution release is
requested by this handoff.
