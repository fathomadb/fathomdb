# LOCOMO/PARENT external cell-adapter worker handoff

**Worker branch:** `experiments/performance-locomo-external-adapter-20260816`  
**Deliberate campaign base:** `efc215585839bfad05d43986960c765959457785`  
**Scope:** synthetic-only external cell-adapter implementation

## Owned paths

- `experiments/locomo_external_adapter.py`
- `experiments/configs/locomo-01/external-adapter.v1.json`
- `tests/experiments/test_locomo_external_adapter.py`
- `dev/performance-benchmarking/2026-08-16-locomo-parent-external-cell-adapter-contract.md`
- this handoff

No coordinator control, PROGRAM, Track Runner status, shared helper, index,
historical receipt, frozen existing configuration, or existing live executor
file was changed.

## Red-first evidence

`18334e51` is the test-only checkpoint. Before the adapter existed,
`PYTHONPATH=src/python python -m pytest
tests/experiments/test_locomo_external_adapter.py -q` failed during collection
because `experiments.locomo_external_adapter` did not exist. The implemented
adapter now satisfies the same human-intended synthetic tests.

## Focused verification

```bash
python -m ruff check experiments/locomo_external_adapter.py \
  tests/experiments/test_locomo_external_adapter.py
PYTHONPATH=src/python python -m pytest \
  tests/experiments/test_locomo_external_adapter.py -q
```

The focused tests prove strict ABI parsing, duplicate-key rejection, exact safe
result shape and required metric families, FTS versus hybrid FathomDB call
selection through an injected engine, no raw corpus/question leakage to stdout
or metrics, GPU no-fallback behavior, and PARENT relation-proof rejection and
safe hit shape. They use synthetic temporary files only.

## Reviewer focus

1. Confirm the module is standalone enough to copy externally and that it
   cannot write repository receipts or the index.
2. Inspect the byte-bound canonical provenance/relation checks and the exact
   frozen `parent_child_turn_session_v1` comparison.
3. Confirm that a GPU request fails before the engine opens unless a single
   visible `cuda:0` is attested; this is intentionally narrower than the live
   executor's generic CUDA policy because its request lacks an ordinal.
4. Verify that all real corpus/model/GPU/external-write behavior is deferred to
   a coordinator release and factual preflight.

## Factual runtime prerequisites

The external executable still needs an access-controlled deployment, qualified
LOCOMO/provenance/TRACE/parent/subset inputs, an empty output root, and a pinned
FathomDB runtime with local model assets. GPU cells also need a functioning
driver and the single visible `cuda:0` envelope described in the contract. No
live corpus, benchmark, model, GPU, external artifact, receipt, or index write
was performed by this worker.
