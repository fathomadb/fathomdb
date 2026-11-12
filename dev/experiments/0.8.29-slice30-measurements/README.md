# Slice30 current direct-text measurement tools

Initial P only: provider-free direct text, unchanged format37 and contracts.
[Design](../../plans/0.8.29/features/slice-30/design.md) and
[preregistration](../../plans/0.8.29/features/slice-30/preregistration.md) own scope.
G/D need later concrete review; M/F and whole30 remain held.

## Tools and origins

- `converter.py`: current bounded LongMemEval input conversion. Origin:
  07095a16dad020798bc5125cbde0c909e8c38a78,
  dev/experiments/ac013-scale-search/lme_corpus.py. Preserves first-seen unique
  nonempty turn/session order and all questions; source answers are never read.
  Prefix selection/reporting and array validation are separate new behaviors.
- `test_tools.py`: literal order/count oracles; planned statistics/runner tests
  are added in their own runtime RED/GREEN units. No tamper/drift/path-hardening.

## Commands

Use this checkout's Python and an owned/mnt/data output directory:

```bash
.venv/bin/python -m unittest discover -s dev/experiments/0.8.29-slice30-measurements -v
.venv/bin/python dev/experiments/0.8.29-slice30-measurements/converter.py \
  --lme /mnt/data/OWNED/longmemeval_s_cleaned.json --limit 100000 \
  --out /mnt/data/OWNED/converted
```

Outputs are corpus.jsonl, queries.jsonl and inputs.json; raw text remains outside
Git. CLI limits are configurable. New measurement results may be used only after
the corresponding tools/tests and frozen cells are committed. No timing has run.
