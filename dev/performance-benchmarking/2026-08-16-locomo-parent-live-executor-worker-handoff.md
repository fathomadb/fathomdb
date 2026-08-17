# LOCOMO-01/PARENT-01 live-executor worker handoff

**Outcome:** ready for independent read-only review; no live execution occurred.  
**Implementation commit:** `b9e681ff7f510899af953044718b05e7354ae241`

## Independent-review remediation

Red-first remediation checkpoint `747034ffc6d9cf9e6a2a7c435c3b9ee923925100`
added six human-intended regressions before the fixes existed. All six failed:
full TRACE sidecar semantics, canonical provenance binding for parent relation
proof, same-release 52-cell closure, actual selected CUDA device plus adapter
attestation, duplicate-key adapter JSON, and accepted review-evidence binding.

The remediation implements each control without changing shared TRACE or
provenance helpers. The full `trace-projection.v1` validator is invoked before
active sources are accepted; a minimal sources-only object is rejected. Parent
relation entries now pin and match canonical turn/session manifest digests and
fingerprints. CPU and GPU projections require one identical paired full-grid
release/configuration before `finalize-full-grid` may emit the 52-cell closure.
GPU execution checks the selected local device and exact adapter attestation.
The release now requires a hash-pinned `accepted` review-evidence record bound
to the review SHA, release binding, and every integration/config/runner digest.

## Scope and isolation

- Worktree: `/tmp/fathomdb-perf-locomo-parent-live-executor-20260816`
- Branch: `experiments/performance-locomo-parent-live-executor-20260816`
- Deliberate campaign base verified before editing:
  `4c9e321698ec500d27a47649579a96fe172e7ed5`
- Tracks: `LOCOMO-01` plus the HITL-approved
  `PARENT-01` `parent_child_turn_session_v1` treatment.
- Owned changes: the new local runner, its separate typed config, one
  human-intended lifecycle-test module, and the dated contract/handoff only.

`PROGRAM.md`, Track Runner control/status, shared experiment helpers,
historical configs/receipts, `experiments/index.jsonl`, and external campaign
artifacts were not changed.

## Delivered boundary

- `experiments.locomo_live_executor` is an actual future subprocess runner,
  separate from the Phase-B injected adapter. It invokes only a released,
  hash-pinned absolute external cell adapter and accepts one strict safe JSON
  result per frozen cell.
- `live-executor.v1.json` pins the canonical Phase-B config and current runner
  digest. It has three distinct gates: five-cell listed-order dry run, 26-cell
  CPU grid, and 26-cell GPU/CE grid. The two full-grid actions are disjoint and
  cover all 52 frozen cells.
- A `locomo-live-executor.release.v1` record is self-hashed and binds the
  current integrated SHA, both configuration digests, runner and adapter
  digests, accepted review-evidence record, `seq-249`/`seq-250`, the exact dry
  gate or paired CPU/GPU gates, GPU policy/selected device, and all
  external roots. It fails closed on stale, tampered, repository, historical,
  missing, or digest-drifted inputs.
- Parent results require complete TRACE-01 sidecar semantics plus an external,
  hashed `locomo-parent-relation-proof.v1` inventory whose relation entries are
  cryptographically and structurally bound to the canonical turn/session
  provenance manifests before bounded context is retained.
- Only completed action projections are emitted. They contain no paths or
  payload and are compatible with the Phase-B receipt contract. A dry proof is
  eligible for later receipt/index closure; `finalize-full-grid` accepts only
  the paired same-release/config CPU/GPU projections with exactly 52 unique
  cells before declaring full-grid receipt/index eligibility.

## Red-first and verification evidence

The red checkpoint is `3f6e4fe3`, which added lifecycle tests before the
module existed. It failed during collection as intended:

```text
ImportError: cannot import name 'locomo_live_executor' from 'experiments'
```

After implementation, the following passed:

```bash
python -m ruff check experiments/locomo_live_executor.py tests/experiments/test_locomo_live_executor.py
python -m pytest tests/experiments/test_locomo_live_executor.py \
  tests/experiments/test_locomo_live_executor_review_remediation.py \
  tests/experiments/test_locomo_phase_b.py \
  tests/experiments/test_locomo_phase_b_review_remediation.py -q
# 43 passed
python -m experiments.locomo_live_executor validate experiments/configs/locomo-01/live-executor.v1.json
python -m experiments.locomo_live_executor preview experiments/configs/locomo-01/live-executor.v1.json
./scripts/agent-lint-md.sh
git diff --check
./scripts/agent-verify.sh
```

The synthetic lifecycle test injects both a fake external adapter and fake
hash-pinned external files. It proves listed dry-run dispatch order, no adapter
start after token tampering, parent TRACE/membership/ordinal enforcement,
complete-only projection behavior, and absence of external paths or fixture
payload from the written projection. It is not a corpus or benchmark run.

## Review focus

1. Confirm that the release self-hash plus accepted review-evidence binding and
   current integrated SHA makes an
   isolated-worker token unusable after coordinator integration drift.
2. Check the CPU/GPU action membership against the Phase-B grid and the exact
   user authorization; especially confirm that `gpu_ce_grid` deliberately
   retains non-CE GPU controls to cover the full grid.
3. Adversarially inspect full TRACE semantics, canonical parent relation
   provenance binding, selected CUDA device/adapter attestation, duplicate JSON
   parsing, external-root, duplicate/partial projection, and receipt/index
   eligibility failure paths.
4. Confirm the same-release CPU/GPU combiner proves 52 unique cells and the
   parent relation proof adds evidence rather than inventing a
   second lifecycle/supersession representation.

## Future command and prerequisites

Only after independent acceptance, coordinator integration, and a matching new
release record may the coordinator use:

```bash
python -m experiments.locomo_live_executor execute \
  experiments/configs/locomo-01/live-executor.v1.json \
  /absolute/external/locomo-parent-release.json \
  --action fixed_subset_dry_run
```

Required facts are the newly integrated SHA, release self-hash, accepted review
SHA, exact runner/adapter digests, existing external root, byte-matching corpus
and provenance/subset inputs, accepted TRACE sidecar, full parent relation
proof, and matching action device policy. No corpus was acquired, no GPU/model
or paid service was invoked, no external campaign artifact/index row was
written, and no push occurred.
