# LOCOMO/PARENT input qualification worker handoff

**Worker branch:** `experiments/performance-locomo-input-qualification-20260816`  
**Deliberate base:** `febd1155e61b2f8da3ed17870a8966d1034e16a7`  
**Scope:** LOCOMO/PARENT factual input qualification only

## Owned paths

- `experiments/locomo_input_qualification.py`
- `tests/experiments/test_locomo_input_qualification.py`
- this contract and handoff

No coordinator board, PROGRAM, shared executor, adapter, receipt, or index
file was changed.

## Red-first evidence

`ec6863f6` is the test-only red checkpoint. It records hash-pin mismatch,
missing-input, and ambiguous-parent-membership behavior before the qualifier
implementation was committed. The module was absent from that commit.

`831d5624` is the separate red checkpoint for malformed corpus JSON, malformed
or invalid fixed-subset JSON, and invalid corpus shape. It also preserves the
hard-rejection control-plane boundary for malformed Phase-B configuration.

## Green verification

Run from this worktree:

```bash
PYTHONPATH=. python -m pytest \
  tests/experiments/test_locomo_input_qualification.py \
  tests/experiments/test_locomo_live_executor.py \
  tests/experiments/test_locomo_external_adapter.py \
  tests/experiments/test_locomo_phase_b.py -q
python -m ruff check \
  experiments/locomo_input_qualification.py \
  tests/experiments/test_locomo_input_qualification.py
```

Expected focused result after implementation: 37 passing tests. The broader
agent verifier remains a coordinator/reviewer gate.

## External artifact evidence

The content-free blocked report is at
`/tmp/fathomdb-performance-artifacts-20260816/locomo-preflight/locomo-input-qualification-report.v1.json`.
Its file SHA-256 is
`2f08d6ffaf6b9e9d8f96c0dc16d3cc535702ec63a48ab84630f84ffaa5025954`;
its canonical self-hash is
`af003df7f2a1c1f37b1372be7b3f8b3a6deaf1fde08dad9bfb601f6b6c216b07`.

The separately valid content-free TRACE sidecar at the same root has file
SHA-256 `8797b79975b71fa21894377837d15151d21bc49e3f1604ff69649d857abaf6fa`.
No parent-relation proof was emitted.

The report records these factual blockers:

1. available LOCOMO payload SHA-256 differs from the frozen Phase-B pin;
2. the frozen 32-ID subset is unavailable; and
3. the frozen turn provenance uses ambiguous child IDs, so the current PARENT
   proof/adapter ABI cannot construct a globally unambiguous relation.

Turn and session manifest pins did match. The CORPUS-01 matrix confirms the
external-only, non-commercial LOCOMO posture and limits eligibility to its
named retrieval/temporal/multi-session claims.

## No-live statement

This lane did not acquire a corpus, run the adapter or live executor, load
FathomDB or a model, select a device, use CUDA, create a result receipt, append
an index, conduct human review, or make any external service call. It created
only the declared content-free preflight artifacts.

## Review focus

- Ensure duplicate-key parsing and artifact-root containment fail closed.
- Verify canonical report self-hashing and that no path/payload fields enter
  external outputs.
- Confirm a mismatched pin never becomes a new pin or release input.
- Confirm the parent-proof ambiguity is reported rather than repaired by an
  undocumented ID rewrite.
