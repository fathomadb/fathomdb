---
title: 0.8.25 Slice 10 — executable measurement classification design
status: READY
design_version: 6
review_fix: 3
depends_on: 7
---

# Slice 10 design

## Authority, scope, and readiness

Implements R25/AC25-10, N25-04, and Memex need 24 under architecture v2's
measurement boundary. EARP run identity, blockers, metric eligibility, and
output ordering are reused. Historical receipts are immutable references; this
design adds classification sidecars only. Slice 7 is complete. This is a
development/evaluation contract: it changes no Engine, SDK, wire, persisted
database, or public-documentation surface. READY requires an independent
review with no unresolved P1/P2.

## Requirements-to-design comparison

| Boundary | Decision |
| --- | --- |
| Metric ownership | Every metric leaf has one layer, source pointer, and contributing components. |
| Exact search execution | Every arm/call path references an execution witness. |
| Negative execution claim | `not_executed` requires coverage evidence, never absence. |
| Comparability | Arm manifests derive shared/differing components. |
| Mixed-layer claims | Validator rejects claims narrower than contributors. |
| Historical integrity | Sidecars hash exact source artifacts and never re-score or rewrite an index row. |
| Future enforcement | A frozen index-prefix policy requires valid sidecars for every post-cutover row. |
| Persistence | Atomic, byte-idempotent writes reject collisions and partial state. |

## Versioned classification contract

The closed JSON schema is `measurement.classification.v2`. Every object rejects
unknown keys, unknown enum values, duplicate IDs, and dangling references.
Implementation review found that the provisional v1 contract let a sidecar
select its own measurement roots and did not require an Engine-contributed
metric or comparison arm to name an execution witness. Version 2 binds roots
and exclusions in the plan and enforces those witnesses. The one provisional
v1 native run remains preserved but is explicitly superseded and cannot support
a successful claim.

```text
MeasurementLayer = data_plane | semantic_control_plane | end_to_end
ExecutionState = executed | not_executed | unknown_historical
EvidenceKind = instrumented_call | source_result | coverage_trace |
               static_path_audit | immutable_receipt
ClassificationOutcome = complete | blocked
BlockedReasonV2 {
  code, stage, message, detail
}

SourceArtifactV2 {
  id, locator_kind, locator, role, sha256, measurement_root_json_pointers
}
ComponentV2 {
  id, name, kind
}
CallPathV2 {
  id, operation, source_artifact_ids
}
ExecutionWitnessV2 {
  id, arm_id?, call_path_id, component_id, engine_search_state,
  call_count, count_semantics, evidence_kind, source_artifact_ids
}
MetricRefV2 {
  id, source_artifact_id, json_pointer, value_type, allowed_values,
  layer,
  contributing_component_ids, execution_witness_ids
}
MetricExclusionV2 {
  source_artifact_id, json_pointer, reason
}
ComparisonArmV2 {
  id, component_ids, execution_witness_ids
}
ComparisonV2 {
  id, arms, shared_component_ids, differing_component_ids
}
ClaimV2 {
  id, layer, metric_ids
}
MeasurementClassificationV2 {
  schema_version, classifier_version, classification_id, run_id, outcome,
  blocked_reason, source_artifacts, components, call_paths,
  execution_witnesses, metrics, metric_exclusions, comparisons, claims,
  migration
}
```

`locator_kind` is `repository_path`, `external_path`, or `git_blob`.
Repository paths cannot escape the repository; external paths are allowed only
for historical source artifacts already named and hashed by the immutable
record; git blobs use `<commit>:<path>` and must resolve locally. Artifact roles
are exactly `record`, `metrics_payload`, `checkpoint`, `configuration`,
`implementation`, or `derivation_spec`. In every case, the resolved bytes must
match `sha256`.

Only `metrics_payload` artifacts may carry measurement roots and must carry at
least one. Evidence-only `record`, `checkpoint`, `configuration`,
`implementation`, and `derivation_spec` artifacts require an empty roots list.

A metric JSON Pointer resolves to one scalar leaf in the named source
artifact. `value_type` is `number`, `boolean`, or `enum`; enum leaves are strings
and require a nonempty closed `allowed_values` list containing the resolved
value. Number/boolean leaves require an empty allowed-values list. Every leaf
beneath the artifact's declared measurement roots is referenced exactly once
by `metrics` or by an exclusion. Exclusion reasons are exactly
`identifier_or_hash`, `run_control`, or `cost_budget`.
Overlap, missing leaves, containers, free-form strings, and source-free paths
reject. Empty metric-payload roots and overlapping roots reject. The migration
manifest fixes roots for historical payloads, so a classifier cannot omit an
inconvenient result.

Component kinds have a minimum layer:

| Minimum layer | Kinds |
| --- | --- |
| `data_plane` | `corpus_gold`, `fathomdb_storage`, `fathomdb_engine_search`, `deterministic_metric`, `external_retrieval` |
| `semantic_control_plane` | `query_planner`, `candidate_planner`, `context_packer` |
| `end_to_end` | `answer_generator`, `semantic_judge` |

Layers form an ordered lattice. A metric's effective layer is the maximum of
its contributing component minima. A claim's effective layer is the maximum
of its referenced metrics. Declared values must equal, not merely exceed, those
derived values. Thus a semantic or end-to-end metric cannot be advertised as a
data-plane claim.

Component completeness is not trusted to the classification sidecar. Every
post-cutover run's resolved config contains a closed `measurement_plan` object
with `path`, `sha256`, and `plan_id`. The referenced tracked
`measurement.plan.v2` document fixes component/call-path/arm manifests,
measurement roots, each expected metric JSON Pointer and its required
components, and expected claims before execution. The record's existing config
hash binds that plan. Validation requires the sidecar to equal the plan's
components and ownership; missing or additional components, metric bindings,
arms, or claims reject. A blocked outcome may omit realized metrics/claims but
must retain the plan identity. Historical manifest entries carry the same
closed plan fields for pre-cutover runs. This source-bound plan is the
authority from which layer, shared/differing, and completeness checks derive;
the sidecar is the observed application of that plan.

Every `call_path_id` resolves to one `CallPathV2`, whose `operation` is exactly
`Engine.search` or `external_retrieval`. An optional `arm_id` must resolve to a
comparison arm; it is null for a standalone characterization such as the native
fixture. `count_semantics` is `exact`, `lower_bound`, or `unknown`. `executed`
requires `Engine.search`, a positive count with exact/lower-bound semantics, and
an instrumented-call or source-result witness. `not_executed` requires zero
exact calls plus path-complete coverage or static-path evidence. Missing or
non-immutable proof is `unknown_historical`, with null count and unknown
semantics; it supports no positive or negative execution claim. There is no
run-global flag.

For each comparison, the validator derives shared components as the exact arm
intersection and differing components as the exact union minus intersection;
the stored values must match. This makes component claims executable rather
than narrative.

## Persistence and future enforcement

The sidecar path is
`experiments/runs/<run_id>/measurement-classification.v2.json`. Its
`classification_id` is the SHA-256 of the complete canonical classification
body with only `classification_id` omitted. Writers serialize UTF-8 canonical
JSON with one trailing newline to an exclusive same-directory temporary file,
fsync it, and use a same-filesystem hard link as the atomic no-replace publish
primitive. They then unlink the temporary file and fsync the directory. If the
target exists, identical bytes are success and differing bytes raise
`measurement_classification_conflict`; no ordinary replace/rename may overwrite
it. No classification index is added: the existing append-only experiment
index remains authoritative for run discovery.

`experiments/measurement-classification-policy.v2.json` is closed to
`schema_version`, `classifier_version`, `index`,
`historical_manifest_path`, and the exact superseded post-cutover run set.
`index` is closed to `path`, `prefix_bytes`,
`prefix_lines`, and `prefix_sha256`.

`experiments/measurement-classification-global-01.v2.json` is closed to
`schema_version`, `included`, and `excluded`. Each included row fixes run ID,
record/metrics hashes, measurement roots, required artifact locators/hashes,
component/call-path/arm manifests, metric ownership, and claims. Each excluded
row fixes run ID and the sole reason
`not_decision_bearing_complete_comparison`. Included/excluded sets must be
disjoint and must exactly cover all indexed `GLOBAL-01` runs at cutover.

`experiments/audits/global-01-measurement-classification-source.v1.json` is the
portable, closed audit receipt. It records the externally verified
checkpoint/result paths and hashes, commit-addressed implementation blob
hashes, retrieval-cell counts, queries-per-cell derivation, and resulting
lower-bound witnesses. The historical manifest pins this audit receipt. The
portable lint validates the committed records, metrics, git blobs, audit
receipt, manifest, policy, and sidecars; it never requires `data/`.

The repository validator:

1. proves that prefix is unchanged;
2. validates the two explicitly included historical decision runs;
3. verifies the closed exclusion inventory for non-decision GLOBAL-01 runs;
4. requires a structurally valid sidecar for every index row appended after
   the prefix.

The validator is a mandatory `agent-lint` leg. A runner crash between index
append and sidecar write therefore leaves a typed, actionable lint failure; it
cannot silently create an unclassified claim. A blocked experiment writes the
normal sidecar with `outcome: blocked`, a closed nonempty `blocked_reason`, and
empty metrics/claims. This satisfies classification presence but is ineligible
as successful evidence. `outcome: complete` requires nonempty metrics and no
blocked reason. Because the index is append-only, no blocked row creates an
irreparable gate failure.

Historical external bytes are verified separately by the deep command
`audit-historical --external-root data`. That command recomputes the portable
audit receipt from raw result/checkpoint bytes and fails on any difference. It
runs during Slice 10 verification when the preserved data tree is present, but
is deliberately not part of clean-clone lint.

## Historical classification and migration

Migration is closed to `kind` (`historical` or `native`), `manifest_path`,
`manifest_entry_sha256`, `measurement_plan_id`, and
`measurement_plan_sha256`; historical classifications use manifest fields and
native classifications use plan fields.
Migration hashes each source receipt/result, validates the sidecar, and writes
only that sidecar. It never appends or rewrites a historical index row. Changed
artifacts fail `historical_receipt_changed`; values, prompts, responses, and
verdicts remain byte-identical.

GLOBAL-01 is pinned as three classified run groups:

1. The first comparison's code receipt is dirty and does not bind the executed
   runner. Both arms therefore carry `unknown_historical`. Later source review
   indicates that the storage-backed arm bypassed `Engine.search`, but this
   narrative is not promoted into an immutable negative witness.
2. The held-out run has a clean source receipt and checkpoint. Its
   `f5c5715:experiments/global_01_lazy.py` and
   `f5c5715:experiments/global_01_lazy_live.py` git blobs are hashed source
   artifacts. The portable audit receipt binds those blobs, the config's
   four-subquery rule, and checkpoint retrieval-cell counts. The 42
   completed control cells prove a lower bound of 42 search calls; the 42
   treatment cells prove a lower bound of 210 calls because each path uses the
   original query plus four subqueries. Lower-bound semantics account for a
   crash after search but before checkpoint persistence. The 39-question
   result excludes the three witness questions; all answer metrics remain
   end-to-end because planning and answer generation contributed. Lifecycle
   canary calls are classified by a separate call path and witness.
3. Slice 10 adds a small deterministic direct-native fixture that calls
   `Engine.search` without answerer/judge and emits data-plane retrieval
   metrics only. It validates classification machinery; Slice 75 repeats the
   witness against the locally packaged release candidate and owns final
   release evidence.

The migration manifest names exactly these two complete decision-bearing runs:
`global-01-native-comparison-20260829T1613Z-40685e82` and
`global-01-lazy-coverage-20260829T2159Z-60b3642c`. It enumerates every other
GLOBAL-01 preflight or invalid-witness run by ID with the reason
`not_decision_bearing_complete_comparison`. Historical external artifacts may
resolve through the repository's `data` symlink during the deep audit only when
their canonical path and SHA-256 exactly match the immutable record. Missing
inputs or a typed Engine refusal produce a blocked classification, never an
empty or guessed sidecar.

## Native fixture

`experiments/configs/measurement-classification/native-search.v2.json` pins:

- `program_track: GLOBAL-01`, schema/config version, seed, CPU device,
  `embedder: none`, cross-encoder disabled, query, limit, and expected source;
- three literal tiny records and their stable caller IDs;
- a tracked `native-search.measurement-plan.v2.json` whose hash is bound by the
  resolved config;
- `prepare_test_database` with a fresh external database root and captured safe
  `fathomdb doctor`/resolved-config evidence.

The fixture wraps the concrete `Engine.search` call with a local counter. The
counter, reciprocal rank, and recall are written to the metrics payload;
returned hit IDs and expected rank are retained in a separate hashed result
detail artifact. A hashed runtime attestation binds the Python executable,
Python package, native extension, and CLI used by the call. Exactly one call is
required. A missing expected hit, non-one call
count, setup refusal, or record/classification failure emits a standard blocked
receipt and fails nonzero. The blocked record still receives a valid blocked
classification before exit. It uses no network, model, GPU, generated oracle,
or repository-local database.

`blocked_reason` reuses the EARP blocker shape `{code, stage, message, detail}`.
The Slice 10 code subset is closed to `config_invalid_value`, `fixture_invalid`,
`run_id_collision`, `database_setup_failed`, `engine_open_failed`,
`engine_search_failed`, `search_call_count_mismatch`, `expected_hit_missing`,
`record_write_failed`, and `source_artifact_unavailable`. `stage` is a nonempty
identifier, `message` is nonempty, and `detail` is a JSON object. Unknown codes
or a blocked reason on a complete outcome reject.

## Failures, performance, and tests

Failures: `unsupported_classification_version`,
`measurement_classification_invalid`, `component_reference_invalid`,
`execution_witness_invalid`, `execution_coverage_missing`,
`claim_layer_mismatch`, `measurement_classification_conflict`,
`historical_prefix_changed`, `classification_missing`, and
`historical_receipt_changed`. Classification is local, no-network/model/GPU,
and excluded from retrieval latency.

RED/GREEN fixtures cover closed schemas; positive/negative/unknown witnesses;
source-bound exhaustive metric leaves; layer derivation; per-arm asymmetry;
derived comparison sets; the two included and all excluded GLOBAL runs;
dangling/hash-changed sources; atomic/idempotent/conflicting writes; prefix and
post-cutover enforcement; blocked sidecars; and the direct fixture.

Exact verification routes are:

```bash
python -m pytest tests/experiments/test_measurement_classification.py -q
python -m experiments.measurement_classification validate-tree --repository-root .
python -m experiments.measurement_classification audit-historical \
  --repository-root . --external-root data
python -m experiments.measurement_classification run-native \
  --config experiments/configs/measurement-classification/native-search.v2.json
./scripts/agent-verify.sh --tier=fast
./scripts/agent-verify.sh --tier=heavy
./scripts/agent-verify.sh --tier=all
AGENT_LONG=1 ./scripts/check.sh
```

The generated native record, sidecar, setup receipt, and status record are
inspected independently. Windows, binding, wire, live-model, registry, and CUDA
routes are N/A because this slice adds no product surface and performs a
CPU-only development fixture. The full workspace clippy/check gates remain
mandatory. No unresolved implementation-shaping decision may remain at READY.
