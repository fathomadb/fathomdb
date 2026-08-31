# LOCOMO-01 and PARENT-01 Phase-B worker handoff

**Date:** 2026-08-16

**Outcome:** ready for independent read-only review; no live benchmark was run.

## Scope and base

- Tracks: `LOCOMO-01` and the approved `PARENT-01` treatment.
- Worktree: `/tmp/fathomdb-perf-locomo-parent-exec-20260816`.
- Branch: `experiments/performance-locomo-parent-exec-20260816`.
- Verified base before changes: `378a8214041033891d6a1e66fae8dcf9f8cc9681`.
- Red checkpoint: `f2b652c20e5a888780951211f3f9f7f507c2a5b0`.
- Implementation commits: `e6ad61bd97a54ea58675973ae2d374c1bf2166d5` and
  `57c19876419564668983d703bfadfca9141d9fa5`.
- Independent-review remediation red checkpoint:
  `170c6d0d8394547fd36babae4ccd5665c1dba833`.
- Independent-review remediation implementation:
  `c872b6aad5fd96ac51395942db8023fade70d463`.

The worker owned only the new Phase-B adapter, its typed configuration, its
human-intended tests, and these dated contract/handoff documents. It did not
modify `PROGRAM.md`, Track Runner control/status, shared experiment helpers,
historical configurations or receipts, the index, or external artifacts.

## Delivered contract

- `experiments.locomo_phase_b` resolves the frozen Phase-A catalog by SHA-256,
  expands 48 LOCOMO cells plus four `PARENT-01` cells, and supports safe CLI
  `validate` and `preview` only.
- `phase-b-execution.v1.json` assigns `program_track` per resolved cell,
  hash-pins external corpus/provenance/subset inputs, freezes M1/M2/M4-proxy/
  M6/M7 plus class metrics, and fixes the five-cell 32-question dry run.
- `parent_child_turn_session_v1` has exact session-parent selection, top-10
  child rank preservation, no second fusion, five bundles, one neighbor per
  side, bounded context, TRACE-compatible attribution, and fail-closed unsafe
  input rejection.
- `execute` remains unavailable until a matching coordinator release record
  names the independent review SHA and both `seq-249`/`seq-250`; it requires a
  pre-existing root outside the repository and dispatches only an injected
  later-reviewed cell executor.
- `write_safe_receipt` composes the unchanged common receipt/index helper with
  logical metrics references and digests only. It rejects repository and
  historical-output destinations, and an executed dry-run proof is recorded
  honestly as a complete receipt rather than as a planned run.

## Independent-review remediation

The first independent review requested four corrections. The red checkpoint
added a separate regression suite, which failed before implementation on all
four findings. The remediation now enforces:

1. The five fixed dry-run cells dispatch in their declared list order.
2. Results bind a cell ID and mode; receipts require complete, unique selected
   coverage and all LOCOMO metric sections, plus PARENT metrics for each
   PARENT cell, before a complete receipt is written.
3. Parent-child context requires child provenance with exactly one parent,
   ordinal, and TRACE source ID. Neighbors carry same-session ordinal and TRACE
   evidence proving immediate `-1` or `+1`; compact strings are rejected.
4. Receipt verdict and `n` are coherent: dry proof is complete with 32
   questions, and the full grid is complete with 1,536 evidence-backed
   questions.

## TDD and verification evidence

The red test commit intentionally failed before implementation with:

```text
ImportError: cannot import name 'locomo_phase_b' from 'experiments'
```

After implementation, these commands passed:

```text
python -m ruff check experiments/locomo_phase_b.py tests/experiments/test_locomo_phase_b.py
python -m pytest tests/experiments/test_locomo_phase_a.py tests/experiments/test_locomo_phase_b.py tests/experiments/test_locomo_phase_b_review_remediation.py tests/experiments/test_locomo_provenance.py tests/experiments/test_locomo_metrics.py -q
# 30 passed
python -m experiments.locomo_phase_b validate experiments/configs/locomo-01/phase-b-execution.v1.json
python -m experiments.locomo_phase_b preview experiments/configs/locomo-01/phase-b-execution.v1.json
./scripts/agent-lint-md.sh
git diff --check
./scripts/agent-verify.sh
```

`agent-verify.sh` exited zero. No corpus was acquired; no model, GPU, paid or
external service was invoked; no external artifact or experiment receipt/index
was written by this worker.

## Review focus

1. Confirm the 52-cell and five-cell dry-run interpretations faithfully retain
   the Phase-A grid and the approved PARENT amendment.
2. Check that release metadata cannot be omitted, mismatched, or used with an
   in-repository/historical destination before injected execution begins.
3. Check safe receipt/index composition for path and corpus-content leakage.
4. Check parent deduplication, rank tie ordering, neighbor/session bounds, and
   TRACE attribution against the frozen treatment rather than against a new
   alternate representation.

## Integration and execution boundary

This commit is preparation evidence, not a LOCOMO/PARENT measurement. The
coordinator must obtain an accepted independent review, integrate the reviewed
commit, then issue the matching release record before a later executor receives
the externally pinned corpus/provenance roots. A completed run still requires
per-cell external metrics, safe receipt/index evidence, and the track-charter
selection rules.
