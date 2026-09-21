# Performance Gauntlet v1.2 — Slice 30B status

**State:** DESIGN REVISION  
**Slice:** 30B — AC-075 TC-5 and SCALE-02 release overlays

## Current-state reconciliation

- Aggregate Slice 30 design is approved. Slice 30A is complete; its closed cell
  plan shape and artifact-placeholder convention are the interface extended
  here.
- `experiments.tc5_gpu_v2` already owns the complete TC-5 workload, result
  format, artifact checks, and isolated-runtime attestation. It admits 0.8.23
  and 0.8.25 only; its 0.8.25 branch deliberately pairs product version 0.8.25
  with wheel package 0.8.24.
- `experiments.scale_02` already owns the five-point ladder, five fresh
  repetitions per point, measurements, output-local record writer, and
  `record_base_dir` plumbing. Missing work is limited to using the loaded
  release instead of a module constant and exposing the existing record-root
  argument on the CLI.
- Both frozen input configs contain historical absolute runtime and data paths.
  The gauntlet must create new overlays beneath its output root and leave those
  configs byte-for-byte unchanged.
- Slice 20 already resolves the required target Python, wheel, native
  extension, CLI, TC-5 corpus root, qualified manifest, embedder cache, GPU
  UUID, source commit, and config identities. Slice 50 will build and resolve
  the TC-5 benchmark artifact placeholders.

## Requirements and acceptance criteria

1. Preserve exactly three TC-5 release branches: 0.8.23 has no candidate;
   0.8.25 requires candidate version 0.8.25 and package 0.8.24; 0.8.26 requires
   candidate version and package 0.8.26. All other releases fail closed.
2. Preserve TC-5 bridge size 7,667, K=192, top 10, 100 queries, exact-f32
   truth, historical query/bootstrap seeds, 1,000 resamples, CUDA-only
   embedding, and the existing receipt format.
3. The generated TC-5 overlay binds target source commit, target Python/wheel/
   CLI identities, configured CUDA UUID and TC-5 inputs, plus
   `artifact:tc5_benchmark.{path,sha256}`. Its `--binary` argument uses the same
   artifact path placeholder.
4. SCALE-02 accepts only the release declared by its closed config; runtime
   validation, environment records, and receipts all report that release.
   Existing 0.8.23 configs remain valid without rewriting.
5. SCALE-02 `run-point` requires `--record-base-dir`. The gauntlet plans the
   exact ordered points 10,000, 17,272, 25,000, 40,000, and 50,000 against one
   output-local registry and one output-local artifact root. Prior-point lookup
   and new record writes never fall back to repository `experiments/runs`; the
   two frozen dependency receipts remain intentional read-only inputs.
6. Preserve the existing SCALE-02 workload and policy fields, including five
   repetitions, cold/steady query counts, mutation count, seeds, bootstrap,
   storage/RSS/throughput measurements, and advisory calculations.
7. Base configs and all historical records are read-only. Overlay generation
   is deterministic and JSON serializable. TC-5 uses a template followed by a
   final artifact-resolved overlay; SCALE-02 can produce its final overlay in
   one stage. Both enforce the exact pointer whitelists below.
8. The adapter layer contains no ingestion, retrieval, scoring, percentile,
   bootstrap, or benchmark subprocess implementation.
9. Both modules execute from the gauntlet checkout with the target virtualenv
   Python. The plan binds/hashes the owning runner and helper modules, so the
   compatibility code cannot silently come from the unmodified target tag.
10. SCALE-02 prior-point admission requires the same release and resolved
    overlay identity, including corpus and runtime identities; a receipt from
    another release or overlay in the same registry is rejected.

## Design

Extend `scripts/perf-experiments/gauntlet_cells.py` with deterministic overlay
builders and the two 30B plans. The builders load verified config bindings,
copy their documents, assert the frozen workload envelope, and reject a
canonical JSON-pointer diff outside the explicit whitelist.

TC-5 is two-stage because its final config cannot exist before Slice 50 builds
the benchmark binary. Slice 30 returns a template, template SHA, final path,
and typed dependencies on `artifact:tc5_benchmark.{path,sha256}`. Slice 50
resolves both fields, validates the final document with
`experiments.tc5_gpu_v2.load_config`, writes it atomically, computes its final
SHA, and records `artifact:ac075_overlay.{path,sha256}`. Execution consumes only
that final overlay artifact, never the unresolved template.

The frozen TC-5 base is
`experiments/configs/scale-01/tc5-gpu-v2.json`, SHA-256
`b2ea5c25eee0b93807384259262702d3bb04f1fee4f640579160091ecee2417c`.
The exact overlay pointers are `/release`, `/runtime/python`,
`/runtime/fathomdb_bin`, `/runtime/cuda_uuid`, `/inputs/corpus_root`,
`/inputs/qualified_manifest`, `/inputs/model_asset_directory`, and the added
`/candidate` object. The unchanged identities include corpus index
`624b6b42e7e1d40866fea48d37a34b0ef8d786f46f4aa2ed9a3dd245d22a0a0a`,
qualified manifest
`f6180a00d1551a143a7445aa6ed28ed589533400ee702835663729694e393df5`,
model directory basename `0b2926f8a9b1`, and model digest
`7a7edec71b9b8c9ce82cae05c4be673b274c36eaa474cc40ca2ef81b77847ea2`.

The AC-075 plan invokes the target runtime Python:

```text
<target-python> -m experiments.tc5_gpu_v2 run
  --config artifact:ac075_overlay.path
  --arm bridge
  --output-root <cells/ac075/run>
  --binary artifact:tc5_benchmark.path
```

The plan uses `cwd=<gauntlet-repo-root>` with no ambient `PYTHONPATH` override
and records hashes for `tc5_gpu_v2.py` and `fathomdb_test_setup.py`. It records
the base config, template identity, source, runtime, TC-5 assets, and generated
artifact dependencies. The template derives the
model directory by retaining the frozen model-directory basename beneath the
configured embedder-cache root; Slice 50 verifies the resulting model assets.

The frozen SCALE-02 base is the approved
`experiments/configs/scale-02/a0-envelope.v2.json`, SHA-256
`eb86d5b41e63b4854bde695200b9a0b9552a5c2474650f7cb0762b862851cd28`.
Its exact overlay pointers are `/release`, `/corpus/root`,
`/corpus/qualified_manifest`, `/runtime/python`,
`/runtime/python_package_version`, `/runtime/python_extension`,
`/runtime/python_extension_sha256`, `/runtime/fathomdb_bin`, and
`/runtime/fathomdb_bin_sha256`; the configured corpus and manifest must match
the unchanged frozen hashes. Every other dependency, workload, policy, seed,
and identity is byte-for-byte equal to the base document.

The five planned invocations use `cwd=<gauntlet-repo-root>`, the target runtime
Python, the same final overlay, a shared `<cells/scale02/artifacts>` root, and
required shared `<cells/scale02/registry>`. Their exact argv is:

```text
<target-python> -m experiments.scale_02 run-point
  <overlay> <point> <artifact-root>
  --record-base-dir <shared-registry>
  [--post-boundary-baseline only for 40000 and 50000]
```

The plan records hashes for `scale_02.py`, `_lib.py`, and
`fathomdb_test_setup.py`. Points 10,000, 17,272, and 25,000 retain formal-ladder
mode. Only 40,000 and 50,000 use the historically authorized post-boundary
mode after the 25,000 advisory failure.

Modify `Scale02Config` to carry `release`; replace every execution-time use of
the module `RELEASE` with `config.release`, including runtime attestation,
environment metadata, records, and diagnostics. Keep `RELEASE = "0.8.23"` only
as the legacy default if needed for compatibility, never as validation truth.
Wire `--record-base-dir` through `main` as a required `run-point` argument.
Prior-point scanning compares the earlier record's resolved config to the
current resolved config after removing only `execution_point` and
`execution_mode`; this binds release, corpus, runtime, workload, and policy.
Admit exactly releases 0.8.23, 0.8.25, and 0.8.26, with tests that legacy 0.8.23
is unchanged, 0.8.25/0.8.26 attest their declared versions, and malformed or
other releases fail closed.

Modify only TC-5 config admission logic for the exact 0.8.26 branch. No TC-5
executor, ingestion, query, scoring, or aggregation code changes.

## TDD and review plan

1. RED: add exact 0.8.26 TC-5 admission/rejection tests; SCALE-02 release
   propagation and required CLI record-root tests; and gauntlet overlay/argv,
   immutability, placeholder, ladder, registry, and whitelist tests.
   Include adversarial prior receipts from 0.8.23/different overlays and
   pointer-diff mutations outside each whitelist.
2. GREEN: make the narrow compatibility and planner changes above.
3. Run focused owning-module tests plus the registered experiment boundary.
4. Obtain independent code review, then independent verification. Record exact
   evidence here and close only after both approve.

## Evidence

The first independent design review returned **CHANGES REQUIRED**. This
revision incorporates its two-stage TC-5 lifecycle, historically exact SCALE
flag split, module provenance, frozen base/input identities, prior-receipt
binding, exact argv, and closed release-admission findings. Independent
re-review: **APPROVE**.
