# SCALE-01 TC-5 released live-executor contract

**Track:** `SCALE-01`

**Date:** 2026-08-16

**Status:** implementation preparation only. This contract does not acquire a
corpus, inspect corpus payloads, load a model, invoke EU7, run a smoke or long
characterization, write a campaign artifact, or make an external request.

## Purpose and boundary

`experiments.tc5_live_executor` is a separate execution boundary. It does not
modify `experiments.tc5_characterization`, whose configuration stays disabled
and whose `run_characterization` continues to refuse execution. The new module
can launch a later, external driver only after independent acceptance and a
coordinator-issued `tc5-execution-release.v1` sidecar qualify the exact
invocation.

The executor accepts only the bridge followed by primary arms from the
qualified `tc5-manifest.v1`: 7,667 then 17,272 all-real documents. It passes
the pinned CPU model, `K=192`, 100-query, and 1,000-bootstrap settings to the
external driver through named environment variables. No GPU, padded or
substituted corpus result can become eligible.

Every generated arm directory, arm-result destination, and final receipt
destination is resolved against the declared external output root before any
directory creation, driver start, or receipt write. A path that resolves into
the repository or escapes through a symlink fails closed. The executor repeats
that containment check immediately before driver and receipt operations.

## Frozen inputs

`experiments/configs/scale-01/tc5-live-executor.v1.json` binds the existing
`tc5-execution.v1.json` digest and declares only these actions:

1. `tc5-smoke`
2. `tc5-long-cpu-characterization`

For either action, the executor runs the qualified `bridge` arm before the
qualified `primary` arm. A smoke is still a two-arm qualification under this
interface; a one-arm or synthetic shortcut is not a TC-5 result.

## Coordinator release sidecar

The external release record is an exact-key `tc5-execution-release.v1` object.
It contains a safe release identifier, `issued_by:
track-runner-coordinator`, the current integrated Git SHA, live-config and
manifest digests, the approved action set, UTC expiry, external runner argv,
runner SHA-256, and `release_sha256` over the canonical record excluding that
field.

Before writing anything, the executor rejects an absent, in-repository,
malformed, expired, self-hash-mismatched, wrong-issuer, wrong-checkout,
wrong-config, wrong-manifest, unapproved-action, missing-runner, or
runner-hash-mismatched release. This makes release records non-transferable
across configuration, manifest, runner, or integration drift.

## External driver ABI and result validation

The runner is an existing external executable named by the released argv. It
receives no shell interpolation. For each arm, the executor supplies these
environment variables: `TC5_ACTION`, `TC5_ARM`, `TC5_DOCUMENT_COUNT`,
`TC5_MANIFEST_PATH`, `TC5_MANIFEST_SHA256`, `TC5_CORPUS_ROOT`,
`TC5_OUTPUT_ROOT`, `TC5_ARM_RESULT_PATH`, and the frozen CPU/model/
candidate/query/bootstrap/ground-truth/SUT pins.

The driver must write the declared external `tc5-arm-result.v1` sidecar. Its
strict safe projection includes only arm identity/count, manifest and result
digests, completion count, zero synthetic documents, aggregate recall/CI/sigma,
and pinned input provenance. Provenance repeats and validates the manifest's
Rust version and canonical sorted engine-feature list as well as source,
model, CPU/OS, and measurement pins. It has no raw path, document ID, payload,
prediction, model output, or SCALE-02 claim field. The executor rejects every
unknown or missing field, a missing ground-truth digest, incomplete queries or
bootstrap, GPU, padding/substitution, non-finite (`NaN`/infinite) uncertainty,
or any provenance drift.

Completed qualified arm sidecars are resume-safe: the executor validates and
reuses them rather than launching the arm again. A partial or invalid existing
sidecar fails closed; it is never silently replaced.

## Receipt and index boundary

Only after both qualified sidecars validate does the executor write the safe
external `tc5-live-executor-receipt.v1`. Its index projection is explicitly
`not_appended` and becomes eligible only after a complete safe two-arm receipt.
The executor never writes `experiments/index.jsonl`, repository output, or the
historical `dev/plans/runs/eu7-latest-measurements.json`.

The receipt's `input_digests` projects safe SHA-256 values for the live
executor config, frozen disabled execution config, release sidecar integrity
record, and external runner. It therefore identifies the exact release inputs
without carrying their paths or payloads.

## Future coordinator invocation

After review, integration, a coordinator release, and the factual preflight,
the external-only interface is:

```bash
python -m experiments.tc5_live_executor \
  --action tc5-smoke \
  --release /external/release.json \
  --live-config experiments/configs/scale-01/tc5-live-executor.v1.json \
  --execution-config experiments/configs/scale-01/tc5-execution.v1.json \
  --manifest /external/tc5-manifest.json \
  --corpus-root /external/corpus-root \
  --output-root /external/tc5-output-root
```

Do not execute this command from this preparation packet. The actual release
must name the reviewed integrated SHA and external driver, and its output root
must be allocated outside Git.

## Remaining factual prerequisites

- A CORPUS-01-approved, license-qualified all-real corpus root and immutable
  manifest whose digest and IDs validate without payload copying.
- A dedicated external result driver implementing the stated ABI, plus its
  SHA-256 and an external output root with retention/access controls.
- The independently reviewed integrated checkout SHA, then a coordinator
  release record bound to that SHA, the committed live config, the exact
  manifest, intended action, and driver digest.
- A reserved CPU host whose identity/OS match the manifest, cached pinned model
  asset, exact-f32 ground-truth artifact digest, no GPU selection, and no
  competing workload during the run.
- A coordinator review of the complete safe receipt before any normal
  experiment-index append, remediation interpretation, or SCALE-02 work.
