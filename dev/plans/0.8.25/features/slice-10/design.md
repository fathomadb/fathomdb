---
title: 0.8.25 Slice 10 — executable measurement classification design
status: DRAFT_SCOPE_RECONCILED_BLOCKED_ON_SLICE_7
design_version: 2
review_fix: 1
depends_on: 7
---

# Slice 10 design

## Authority and readiness

Implements R25/AC25-10, N25-04, and Memex need 24 under architecture v2's
measurement boundary. EARP run identity, blockers, metric eligibility, and
output ordering are reused. Historical receipts are immutable references; this
design adds classification sidecars only. READY remains blocked on Slice 7 and
independent review with no unresolved P1/P2.

## Requirements-to-design comparison

| Boundary | Decision |
| --- | --- |
| Metric ownership | Every metric has one layer and contributing components. |
| Exact search execution | Every arm/call path references an execution witness. |
| Negative execution claim | `not_executed` requires coverage evidence, never absence. |
| Comparability | Arm manifests derive shared/differing components. |
| Mixed-layer claims | Validator rejects claims narrower than contributors. |
| Historical integrity | Sidecars hash exact source artifacts and never re-score. |

## Versioned classification contract

```text
MeasurementLayer = data_plane | semantic_control_plane | end_to_end
ExecutionState = executed | not_executed | unknown_historical
EvidenceKind = instrumented_call | coverage_trace | source_receipt |
               source_result | static_path_audit
ExecutionWitnessV1 {
  schema_version: 1, id, comparison_id?, arm_id, call_path_id,
  component_id, engine_search_state, call_count,
  evidence_kind, source_artifact_sha256, instrumentation_version?
}
MetricClassificationV1 {
  metric_path, layer, contributing_component_ids, execution_witness_ids
}
ComparisonArmV1 {
  arm_id, component_ids, execution_witness_ids
}
MeasurementClassificationV1 {
  schema_version: 1, run_id, source_artifacts: [{path_role, sha256}],
  components, execution_witnesses, metrics,
  comparisons: [{id, arms, shared_component_ids, differing_component_ids}],
  claims, migration?
}
```

IDs are unique and every reference resolves. `executed` requires an
instrumented governed `Engine.search` call/result or typed failure and a
positive call count. `not_executed` requires a coverage/static-path witness
that covers the named arm/call path and has call count zero. Missing proof is
`unknown_historical`, which supports no positive or negative search claim.
Metrics and arms reference all relevant witnesses; no run-global execution flag
exists. Unknown fields/versions/enums, duplicate IDs, dangling references, and
unowned metrics fail closed.

Retrieval-only metrics may use corpus/gold, Engine config/calls, and
deterministic metric code only. Caller planning makes a metric at least
semantic-control-plane; answer generation or judging makes it end-to-end. The
validator derives shared/differing components from normalized arm manifests
and refuses a narrower claim.

## Historical classification and migration

Migration hashes each source receipt/result before classification, validates
the sidecar, then appends its index row. Identity is `(run_id, sorted source
hashes, classifier version)`; changed artifacts fail
`historical_receipt_changed`. Values, prompts, responses, and verdicts remain
byte-identical.

GLOBAL-01 is pinned as four distinct paths:

1. The first FathomDB storage-backed arm bypassed `Engine.search`; it carries a
   `not_executed` coverage witness.
2. Its native GraphRAG arm is an external-system path, not FathomDB search; it
   carries its own `not_executed` coverage witness.
3. The 39-question held-out control and treatment each carry separate positive
   `Engine.search` witnesses, but answer metrics are end-to-end because caller
   planning and answer generation contributed.
4. Slice 10 adds a small deterministic direct-native fixture that calls
   `Engine.search` without answerer/judge and emits data-plane retrieval
   metrics only. It validates classification machinery; Slice 75 repeats the
   witness against the locally packaged release candidate and owns the final
   release evidence.

Each path references exact receipt and result hashes. Missing inputs or a typed
Engine refusal produce a blocked receipt, never an empty result.

## Failures, performance, and tests

Failures: `unsupported_classification_version`,
`measurement_classification_invalid`, `component_reference_invalid`,
`execution_witness_invalid`, `execution_coverage_missing`,
`claim_layer_mismatch`, and `historical_receipt_changed`. Classification is
append-only, local, no-network/model/GPU, and excluded from retrieval latency.

RED/GREEN fixtures cover positive/negative/unknown witnesses, per-arm
asymmetry, all four GLOBAL paths, dangling/hash-changed sources, mixed claims,
idempotent migration, and the direct fixture. Run fast schema/unit, heavy copied
index migration, all, and the small native fixture. Product/platform/model
routes remain N/A unless implementation adds a product surface. A formal
independent READY review remains required after Slice 7 completes.
