# LOCOMO/PARENT normalized-corpus qualification worker handoff

**Date:** 2026-08-16  
**Track Runner worker:** `LOCOMO-01` / `PARENT-01` factual qualification correction  
**Worktree:** `experiments/performance-locomo-qualification-normalization-20260816`

## Scope and commits

- Red checkpoint: `e14906d3` — `test(perf): define normalized locomo corpus qualification`.
- Implementation: `375439282d3f179ccbd033900394cdbea304d6fd` —
  `fix(perf): derive normalized locomo corpus pin`.
- Owned changes:
  - `experiments/locomo_input_qualification.py`
  - `tests/experiments/test_locomo_input_qualification.py`
  - `2026-08-16-locomo-parent-input-qualification-contract.md`

The implementation derives the Phase-B corpus identity with
`eval.locomo_loader.corpus_hash(eval.locomo_loader.load_locomo(raw)[0])`. It
records the raw SHA-256 as content-free provenance, validates the derived
normalized SHA-256 against the unchanged Phase-B pin, and leaves raw-corpus
structural and question-ID checks intact. It neither changes a frozen pin nor
rewrites the PARENT child identity representation.

## Red-first evidence

Before implementation:

```console
PYTHONPATH=src/python python3 -m pytest tests/experiments/test_locomo_input_qualification.py -q
```

Result: `1 failed, 8 passed, 1 skipped`. The new synthetic test expected a
qualified report when the raw and derived normalized digests differed; the old
qualifier instead reported `blocked`, proving it compared raw bytes directly
with the normalized Phase-B pin.

## Verification

```console
FATHOMDB_LOCOMO_RAW_PATH=<approved-external-locomo10.json> \
  PYTHONPATH=src/python python3 -m pytest \
  tests/experiments/test_locomo_input_qualification.py -q
python3 -m ruff check experiments/locomo_input_qualification.py \
  tests/experiments/test_locomo_input_qualification.py
./scripts/agent-lint-md.sh
git diff --check
./scripts/agent-verify.sh
```

Results: focused tests `10 passed`; Ruff, Markdown lint, diff check, and the
full agent verifier all exited `0`. The historical raw-corpus test invokes only
the pure LOCOMO loader and corpus-hash function; it does not call `qualify` or
write a report, TRACE sidecar, parent proof, or other external artifact. Its
raw and derived normalized digests are distinct, and the derived digest equals
the unchanged frozen Phase-B corpus pin.

## Boundaries and review focus

No corpus payload, question, answer, external path, measurement, release
record, index row, model, adapter, FathomDB load, CUDA selection, GPU action,
or benchmark action was created or invoked.

Review the raw-versus-normalized identity boundary: a raw byte digest must
remain report provenance only, while the Phase-B corpus comparison must use the
loader-derived normalized digest. Confirm malformed external corpus input still
produces a content-free blocked report and that this correction does not relax
the fixed-subset or parent-relation blockers.
