# SCALE-01 TC-5 factual input qualification handoff

**Track:** `SCALE-01`
**Worker branch:** `experiments/performance-tc5-input-qualification-20260816`
**Deliberate base:** `febd1155e61b2f8da3ed17870a8966d1034e16a7`
**Scope:** content-free factual preflight only.

## Delivered boundary

- `experiments.tc5_input_qualification` validates a future safe inventory,
  CORPUS-01 matrix binding, and exact all-real `tc5-manifest.v1` without
  opening a payload or running an executor.
- `experiments/configs/scale-01/tc5-input-qualification.v1.json` freezes the
  fidelity-only source-claim boundary and required factual attestations.
- The dated contract documents the distinct state
  `factual_inputs_qualified_pending_coordinator_release`; it is never a live
  execution release.
- `tests/experiments/test_tc5_input_qualification.py` began as red commit
  `15307538` (the module did not exist), then covers exact manifest binding,
  matrix claim limitation, missing host/model/ground-truth/runtime facts, and
  content-free report writing.

## Factual result

No exact eligible 18,472-document all-real source with its canonical
7,667-document bridge was found in the authorized read-only inventory. The
only observed campaign inventory was `locomo-capability-20260814`; it is not a
TC-5 source and no LOCOMO payload, provenance body, or raw artifact path was
copied into this lane.

The durable content-free blocker report is:

- path: `/tmp/fathomdb-performance-artifacts-20260816/tc5-preflight/tc5-input-preflight-20260816.json`
- SHA-256: `d55c4cd38e6549f21e20d5dadf7657a4520fccb83c970eafc6a988f0269beec9`
- state: `blocked_prerequisite`
- missing facts: CORPUS matrix source selection, license copy, exact all-real
  manifest, external corpus/output roots, CPU host, cached pinned model,
  exact-f32 ground truth, and vector-stage runtime.

The report says `no_live_execution: true` and
`eligible_for_coordinator_release: false`. It is not a benchmark result,
receipt, index row, or release record.

## Verification

```text
PYTHONPATH=src/python python -m pytest tests/experiments/test_tc5_input_qualification.py -q
# 6 passed

PYTHONPATH=src/python python -m pytest tests/experiments/test_tc5_manifest.py \
  tests/experiments/test_tc5_characterization.py \
  tests/experiments/test_tc5_live_executor.py \
  tests/experiments/test_tc5_external_driver.py -q
# 54 passed
```

`./scripts/agent-verify.sh` also passed on the complete handoff working tree.
No corpus was acquired or inspected; no model/GPU was loaded; no driver, EU7,
smoke, long characterization, release, index append, or external service
action occurred.

## Independent-review focus

1. Confirm no route turns `factual_inputs_qualified` into a coordinator
   release or a SCALE-02/product claim.
2. Confirm the helper validates the complete all-real manifest and its
   canonical bridge through `tc5_manifest.validate_manifest`, rather than
   accepting asserted counts.
3. Confirm the CORPUS-01 matrix claim boundary is enforced and the external
   blocker report contains no payload, raw corpus path, document ID, host
   identity, query, prediction, or metric.
4. Confirm the actual artifact hash/path above and no-live boundary before
   integration.
