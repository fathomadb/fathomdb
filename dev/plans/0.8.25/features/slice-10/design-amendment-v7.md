---
title: 0.8.25 Slice 10 — immutable post-cutover correction policy amendment
status: READY
design_version: 7
amends_sections:
  - versioned-classification-contract
  - execution-proof
  - persistence-and-future-enforcement
triggered_by: slice-35-implementation-review-fix-6
---

# Slice 10 design amendment v7

## Scope

This amendment changes only the evaluation receipt policy. It does not change
FathomDB Engine, SDK, wire, or persisted-database behavior. The accepted
classification schema remains version 2. Measurement-plan v3 supersedes plan
v2 for new exact-count source-result witnesses; plan v2 remains valid for
existing receipts.

Slice 35 exposed three immutable post-cutover receipt defects that the closed
version-2 policy could not represent: a missing embedded plan, an absolute
external-safe-summary locator, and a source witness that named
`Engine.search` although the measured operation was `Engine.search_text_only`.
The original records, index rows, and sidecars must remain byte-identical.

## Policy successor

`experiments/measurement-classification-policy.v3.json` supersedes policy v2.
Policy v2 remains tracked historical evidence and is not rewritten. Policy v3
retains the frozen prefix and historical manifest and adds three closed,
hash-bound inventories:

1. `postcutover_plan_amendments` binds an immutable record lacking an embedded
   plan to one tracked plan by run ID, record SHA-256, plan path, plan SHA-256,
   plan ID, and the sole reason
   `missing_plan_in_immutable_postcutover_record`.
2. `external_artifact_locator_amendments` binds an immutable record whose only
   absolute artifact is an `external_safe_summary` to its record SHA-256 and
   the sole reason `absolute_external_safe_summary_in_immutable_record`.
3. `misclassified_postcutover_runs` quarantines a sidecar whose execution
   witness is known to be false. Each entry binds the run ID, record SHA-256,
   sidecar SHA-256, and the sole reason
   `engine_search_text_only_misclassified_as_engine_search`.

The third inventory is stronger than an amendment. After matching the index
row and loading the run ID, the validator resolves a hash-valid quarantine
entry **before** plan or artifact-locator validation. It verifies the immutable
record and sidecar bytes, records the run as observed, and performs no further
classification validation for that run. Quarantine therefore subsumes a
missing plan, absolute safe-summary locator, and false witness on those exact
bytes. Quarantined runs cannot satisfy a positive or negative execution claim.
A run cannot appear in a corrective-amendment inventory and the quarantine
inventory simultaneously. Every inventory is exact: duplicate, dangling,
unnecessary, overlapping, or hash-mismatched entries fail closed.

## Measurement-plan v3, operation vocabulary, and evidence

`Engine.search_text_only` is added as a distinct data-plane operation using the
existing `fathomdb_engine_search` component kind. It is not an alias for
`Engine.search`. An executed witness for either operation must bind source
artifacts that cover the actual invocation and result-producing wrapper.

Measurement-plan v3 adds the closed `execution_witness_bindings` array. Each
entry is `{witness_id, source_artifact_id, json_pointer}`. Its IDs must be
unique, must resolve to one planned witness and one metrics-payload artifact,
and its pointer must resolve to a positive JSON integer. Every exact
`source_result` witness requires exactly one binding. During repository
validation, the immutable metrics value must equal the sidecar's `call_count`;
a missing, non-integer, or unequal value fails. This validation occurs after
source hashes are checked, so neither a self-consistent sidecar rewrite nor a
configuration-derived count can establish false evidence. Plan v2 does not
gain this field and remains byte-compatible for existing classifications.

The runner records the number of attempted calls at the invocation seam,
aggregates it per arm, and the sidecar builder reads the two observed arm
counts. The plan binds both implementation files by commit-addressed git blobs.

## Compatibility and failure behavior

- Existing valid v2 sidecars and plan-v2 authorities remain valid under policy
  v3.
- Existing policy v1/v2 and historical manifests remain immutable.
- A clean post-cutover run needs no policy inventory entry.
- Unsupported operations, policy keys, reasons, or overlapping correction
  entries fail with `ClassificationError`.
- Quarantine is permanent for the bound bytes. A corrected rerun receives a
  new run ID and ordinary v2 sidecar; it never replaces the old receipt.

## Acceptance tests

- RED/GREEN tests prove policy v3 rejects duplicate, dangling, overlapping,
  unnecessary, and hash-mismatched entries.
- RED/GREEN tests prove a quarantined false sidecar is preserved and cannot be
  validated as evidence.
- RED/GREEN tests exercise a quarantined receipt that also lacks its embedded
  plan and carries an absolute external-safe-summary locator, proving
  quarantine precedence and closed inventory observation.
- RED/GREEN tests prove `Engine.search_text_only` cannot be mislabeled as
  `Engine.search` and accepts only a FathomDB Engine component.
- RED/GREEN tests prove plan-v3 witness bindings reject missing, dangling,
  duplicate, non-integer, and tampered counts.
- RED/GREEN tests prove the Slice 35 sidecar count equals the observed count in
  both arm summaries and that both implementation blobs are bound.
- The portable clean-tree validator passes without access to `data/`.

This amendment becomes READY only after independent design review with no
unresolved P1/P2 findings.
