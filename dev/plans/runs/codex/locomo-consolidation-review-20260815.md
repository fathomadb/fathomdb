# LOCOMO campaign consolidation review

Target: `integrate/locomo-capability-20260815` at `0966a6f4`.
Baseline: `43aa7b09`.
Reviewer: independent adversarial fallback after two Codex review invocations
ended without a terminal verdict.

## Verdict: BLOCK

### 1. [P1] Completed A0 is absent from the generated scorecard

`experiments/scorecard.py` skips receipts without a
`metrics.capability_scorecard` object, while the committed A0 receipt contains
only aggregate and margin metrics. The generated scorecard is consequently
header-only, so it does not expose the baseline that the campaign is meant to
preserve. Add a real-A0 fixture assertion and emit the normalized safe metric
object before regenerating the scorecard.

Refs: `experiments/scorecard.py:61`,
`experiments/runs/locomo-capability-a0-baseline-20260814T2311Z-d4a71071/record.json:41`,
`tests/experiments/test_scorecard.py:55`.

### 2. [P1] Raw artifact and receipt IDs diverge

The runner allocates its external raw directory from the full executable
configuration but writes the durable receipt under an ID derived from the
redacted public configuration. The receipt cannot therefore identify its raw
artifact directory. Derive the public receipt ID before allocation and use it
for both locations.

Refs: `experiments/fathomdb_locomo.py:128`,
`experiments/fathomdb_locomo.py:152`, `experiments/fathomdb_locomo.py:193`.

### 3. [P2] A committed safe receipt exposes absolute external paths

The preflight receipt records absolute paths below `/home/coreyt/.local/share`,
contrary to the external-artifact boundary. Replace them with logical artifact
names plus hashes.

Refs: `experiments/runs/locomo-capability-preflight-20260814T2247Z-1ff8b7bc/record.json:69`,
`dev/performance-benchmarking/2026-08-14-experiment-campaign-execution-plan.md:8`.

## What passed on inspection

Receipt schemas are strict and versioned; provenance sidecars retain hashes and
stable identifiers rather than corpus or query text; paired-comparison
validation checks workload pins; and `git diff --check` passed. The focused
experiment suite passed 55 tests and the full agent verification passed before
this review.

## Process notes

The mandatory Codex reviewer was invoked twice through
`dev/agent-tools/codex-nostdin.sh`; each invocation stopped without a terminal
verdict. This independent read-only review is the required fallback.
