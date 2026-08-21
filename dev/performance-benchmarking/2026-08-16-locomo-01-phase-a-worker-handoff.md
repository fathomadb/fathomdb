# LOCOMO-01 Phase-A worker handoff

**Outcome:** ready for independent read-only review; no live work performed.

## Scope and history

- Track and charter: `LOCOMO-01`,
  [locomo-01-self-characterization.md](tracks/locomo-01-self-characterization.md).
- Worktree and base: `/tmp/fathomdb-locomo-01-20260816`, branch
  `experiments/performance-locomo-01-20260816`, base `d599d4a5`.
- Red-first checkpoint: `6e935daf`
  (`test(perf): specify locomo phase-a planning contract`).
- Green implementation: `ec1150a0`
  (`feat(perf): prepare locomo phase-a grid catalog`).

## Owned paths

- `experiments/locomo_phase_a.py`
- `experiments/configs/locomo-01/phase-a-grid.v1.json`
- `tests/experiments/test_locomo_phase_a.py`
- `dev/performance-benchmarking/2026-08-16-locomo-01-phase-a-contract.md`
- `dev/performance-benchmarking/2026-08-14-locomo-fathomdb-capability-campaign-plan.md`
- this handoff

No shared helper, PROGRAM, Track Runner control/status file, SCALE path,
historical configuration, receipt, or experiment index row changed.

## Delivered preparation

The new `locomo-phase-a.v1` catalog pins canonical A0, LOCOMO identity, both
external provenance manifests, the M1 bootstrap/margin rule, M2/M4-proxy/M6/M7
dimensions, Fast-local rule, and GPU report-only semantics. It deterministically
expands 48 cells: two ingest units × six treatments × CPU/GPU × cold/steady.

The new runner surface supports only `validate` and `preview`. Its configuration
requires `plan_only` plus `live_execution = forbidden`; the direct `run()` entry
always raises. It cannot acquire a corpus, launch a harness/façade, select a
GPU, invoke a model, write external artifacts, or append a receipt/index row.

## Test and verification evidence

Red command, before implementation:

```bash
python3 -m pytest tests/experiments/test_locomo_phase_a.py -q
```

Result: expected collection failure because `experiments.locomo_phase_a` did not
exist.

Green focused commands:

```bash
python3 -m pytest tests/experiments/test_locomo_phase_a.py \
  tests/experiments/test_locomo_metrics.py \
  tests/experiments/test_locomo_provenance.py \
  tests/experiments/test_fathomdb_locomo.py -q
python3 -m experiments.locomo_phase_a validate \
  experiments/configs/locomo-01/phase-a-grid.v1.json
python3 -m experiments.locomo_phase_a preview \
  experiments/configs/locomo-01/phase-a-grid.v1.json
python3 -m ruff check experiments/locomo_phase_a.py \
  tests/experiments/test_locomo_phase_a.py
./scripts/agent-lint-md.sh
git diff --check
```

Results: 18 focused tests passed; validation passed; preview reported 48 cells;
Ruff, Markdown lint, and diff check passed.

Full `./scripts/agent-verify.sh` ran to completion but is not green solely
because this isolated worktree has no local native Python extension. Its Rust
suite passed (`test-rust rc=0`, 569158ms). `test-python` then failed collection
with seven imports of `fathomdb._fathomdb` missing from this worktree. The exact
summary was `registered=73 ran=72 passed=71 failed=1 skipped=1 excluded=0` and
`FAIL verify at step=test tier=all (865s elapsed)`. The security stage recorded
zero violations and its standard AC-037 environmental downgrade. No native
binding was rebuilt or editable-installed from this worktree, in accordance with
the worktree isolation rule.

## No-live assurance and requested review

No corpus acquisition, extractor, harness/dry run/live grid, GPU/model or paid
service invocation, external write, push, receipt, or index append occurred.

Please independently review the two commits against the charter and dated
contract, with particular attention to: (1) historical hashes and A0 pins;
(2) whether every treatment/runtime semantic cell is distinct and stable under
axis ordering; (3) the plan-only refusal boundary; and (4) that no future caller
could interpret this catalog as live-execution authorization.

## Independent-review remediation

An independent review found that the first resolver checked treatment IDs and
the neighbor-expansion flag, but did not freeze the remaining semantics for
each named treatment. This is integration-blocking because a fixed ID could
silently acquire a different retrieval mode, cross-encoder setting, or candidate
breadth.

Red remediation checkpoint: `6b36f1f5`
(`test(perf): pin locomo treatment semantics`). Its six human-intended,
parameterized cases mutated every semantic field—retrieval mode, cross-encoder
alpha, candidate pool, candidate depth, and neighbor expansion—for every named
treatment ID. Before the fix, all six cases found an accepted mutation.

The remediation adds exact, value-and-type checked semantic mappings for all
six IDs. Any change now raises a treatment-specific frozen-tuple error. The
same focused no-live suite now has 24 passing tests, and catalog validation,
48-cell preview, Ruff, Markdown lint, `git diff --check`, and Track Runner check
all pass. The full-gate native-extension boundary above is unchanged and was not
re-run.
