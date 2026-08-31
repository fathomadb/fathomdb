# REASON-01 compact-ledger offshoot result

## Outcome

Close the offshoot as incomplete. It produced no comparative quality decision
and does not make an untouched confirmation, native HippoRAG-2 comparison, or
MEMORY-01 refresh eligible. The original rejection of
`protected_multiquery_v1` stands.

## Execution

- Cohort: the consumed 109-case LongMemEval-S REASON-01 cohort.
- Retrieval: unchanged frozen A0 and protected candidate sets.
- Arms: regenerated A0 raw, protected raw, and protected exact-strip ledger.
- Complete cases: 47 of 109.
- Completed cells: 48 ledgers and 141 answer, correctness, and evidence cells.
- Failure: the compact answer cell for one abstention case exhausted all five
  frozen semantic attempts.
- Spend: $1.357430 authoritative incremental OpenRouter spend for the final
  run; $2.115847 total including prior probes and abandoned diagnostics. The
  $10 Airlock cap was not raised.

Partial quality rates are intentionally not reported. The cohort did not
complete, and its completed prefix is not a registered decision sample.

## Stop diagnosis

The failing ledger contained zero evidence and correctly identified missing
support. On every frozen answer attempt, the reader nevertheless returned the
same non-empty answer with two citation IDs that did not exist. The validator
rejected all five responses. This demonstrates that the citation guardrail
worked, while also demonstrating that the treatment's evidence-empty answer
path is not robust enough for a complete run.

The failure was neither provider throttling nor budget exhaustion. The frozen
contract prohibited prompt, route, limit, or validation changes after the full
run began, so execution stopped rather than tuning on the consumed outcome.

## Interpretation

The offshoot changed only query-time evidence packing and answer constraints;
it did not change FathomDB retrieval, storage, Engine, or SDK behavior. It
therefore supplies no new retrieval claim and no evidence that the compact arm
restores correctness, grounding, or attribution.

The [v2 plan](2026-08-30-reason-01-compact-ledger-v2-plan.md) now preregisters
deterministic all-missing abstention and fail-closed continuation. V2 is a new
protocol, not a repair or continuation of this run, and remains
non-confirmatory on the consumed cohort.

## Evidence

- [Content-free experiment receipt](../../experiments/runs/reason-01-compact-ledger-20260830T2002Z-ab15e1aa/record.json)
- External checkpoint:
  `data/performance-benchmarking/locomo-multihop/runs/reason01-compact-ledger-20260830-01/reason01-compact-ledger-checkpoint.v1.json`
  (`sha256:cd344c7ce7afbe47ac09789e2260eeb15181b76d01a1f3aeee121ad4f632841d`)
- External receipt:
  `data/performance-benchmarking/locomo-multihop/runs/reason01-compact-ledger-20260830-01/reason01-compact-ledger-receipt.v1.json`
  (`sha256:bbfb625cd5ffdf20b3438f4d53c3994a408326af756208418f558a792f0159fe`)
- External summary:
  `data/performance-benchmarking/locomo-multihop/runs/reason01-compact-ledger-20260830-01/reason01-compact-ledger-summary.v1.json`
  (`sha256:ecb844490e8129880c4be5454c021759c2ff5cc8d7ebea1910be2e773f3d37e2`)
