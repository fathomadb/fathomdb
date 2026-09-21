# Performance Gauntlet v1.2 — Slice 20 status

**State:** COMPLETE  
**Slice:** 20 — configuration model

## Reconciliation since the project plan

- Slice 10 is complete and fixes `--dry-run` as metadata-only: it must continue
  to echo opaque paths without reading, resolving, hashing, or creating them.
  Slice 20 therefore exposes configuration loading/resolution as import-safe
  driver functions; CLI preflight and execution adopt them in later slices.
- Existing experiment configurations are heterogeneous and already own their
  workload/scoring contracts. The gauntlet configuration must name those files
  and assets without copying their contents.
- Existing sealed campaign code records SHA-256 for file artifacts and a Git
  commit for source identity. The gauntlet should follow that convention rather
  than invent a tree hash.
- The project plan's phrase “whitelisted overlay fields” does not require users
  to author arbitrary JSON-pointer patches. The safer interpretation is that
  the generated resolved document has an exact closed schema; cell-specific
  compatibility overlays belong with the Slice 30 adapters that understand
  each existing configuration.
- Some runtime artifacts and external datasets may be unavailable on a host.
  The configuration represents each optional binding group explicitly as
  either a closed object of absolute paths or `null`; Slice 50 decides whether
  a selected cell is runnable or typed unavailable.
- The output root is project-created state. It must be absolute, outside the
  target source checkout and every input directory, and distinct from every
  named input file. Resolution writes only
  `<output-root>/gauntlet-plan.resolved.json` with exclusive creation.
- Slice 10 duplicates release/source/output and optional cell selection on the
  CLI. When later modes intentionally load the config, those CLI values are
  assertions: release/source/output must match the config. CLI `--cells`, when
  present, must be a canonical-order subset of configured cells; otherwise the
  configured cell list is used.

## Requirements and acceptance

1. Add a reviewed example at
   `experiments/configs/gauntlet/directional-release.example.json`.
2. Accept exactly these top-level fields: `schema_version`, `release`,
   `source`, `output_root`, `runtime`, `configs`, `assets`, `gpu`, `cells`, and
   `timeouts_s`. Reject missing and unknown fields recursively.
3. Require `schema_version` to equal
   `fathomdb.performance-gauntlet.config/v1`, a semantic release string, a
   40-character lowercase hexadecimal source commit, a non-empty,
   duplicate-free canonical-order subsequence of the cell vocabulary, and
   positive integer timeouts.
4. Define source exactly as `{root, commit}`. Require source/output and every
   non-null runtime, config, and asset binding to be absolute paths. Compare
   canonical paths, resolving existing inputs strictly and the absent output
   root non-strictly. Reject output equal to or beneath any input directory, or
   equal to an input file.
5. Keep `runtime`, `configs`, and nested `assets` groups closed over the exact
   named bindings used by the planned ten-cell execution map. `null` means a
   whole binding group is deliberately unavailable; it does not silently
   select a historical default.
6. Resolve a validated document into the closed schema
   `fathomdb.performance-gauntlet.resolved-config/v1`, preserving the requested
   cells and paths, recording the source commit, and adding SHA-256 identities
   for every non-null regular-file binding and the source configuration itself.
7. Fail resolution when a non-null binding is missing or has the wrong declared
   kind. File bindings must be readable regular files. Directory bindings are
   identified by canonical path rather than a misleading content hash.
8. Require the output root not to exist, create it with `exist_ok=False`, and
   write `gauntlet-plan.resolved.json` using exclusive file creation. Never
   modify an existing experiment configuration, receipt, result, or output
   directory.
9. Preserve Slice 10 dry-run behavior and its exact JSON projection.

## Design

- Keep validation in `run_gauntlet.py` so later preflight/execution share one
  contract. Use standard-library JSON, `pathlib`, and `hashlib`; do not add a
  schema package solely for this closed document.
- Define immutable key tuples for runtime/config/asset bindings and validate
  exact dictionary key sets with path-specific errors. Booleans must not pass
  positive-integer timeout validation.
- Runtime bindings are `fathomdb_cli`, `python`, `wheel`, `native_extension`,
  and `virtualenv`. `perf_gates` is deliberately absent: Slice 50 builds its
  hash-suffixed test executable and records it in `runtime/artifact-manifest.json`.
  Frozen config bindings are `ac072`,
  `ac073`, `ac075`, `scale02`, `protected_writes`, `ce_profile`, `search01`,
  and `locomo`.
- Asset groups are closed and nullable as a whole. `ir_c` contains directory
  `data_root` plus files `snapshot`, `manifest`, and `gold`; `ac073` contains directory
  `corpus_root`; `tc5` contains directory `corpus_root`, file
  `qualified_manifest`, and directory `embedder_model_cache`; `locomo` contains
  directory `harness_checkout` plus files `harness_python`, `dataset`, and
  `provenance_manifest`; `ce_profile` contains directory
  `reranker_model_cache`. SCALE-02 explicitly reuses the TC-5 corpus bindings.
- `gpu` is exactly `{enabled, cuda_uuid}`. A UUID matching the existing
  `GPU-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` identity is required only when GPU
  is enabled; a disabled configuration must use `null` so stale device identity
  cannot leak into a run.
- `timeouts_s` contains `default` plus optional per-cell overrides in a closed
  `cells` dictionary whose keys must be known cell IDs. Resolution expands the
  effective timeout for every selected cell.
- File identities are `{path, sha256}`. Directory identities are `{path}`.
  Null groups stay null. The generated document has no generic extension or
  arbitrary overlay field.
- Resolution is path-owned: read the configuration bytes once, reject duplicate
  JSON keys, hash those exact bytes, parse and validate that snapshot, and use
  only that document to build the resolved plan. This prevents a caller from
  pairing a mutated in-memory document with an unrelated file identity.
- `write_resolved_configuration` accepts the optional validated CLI cell
  subset, creates the output directory only after full validation and identity
  resolution succeed, writes an exclusive temporary file, flushes and fsyncs
  it, then atomically replaces `gauntlet-plan.resolved.json`. Re-running against
  the same root fails rather than replacing evidence; a caught write failure
  removes the temporary file and newly created empty root.

### Exact input shape

```text
schema_version, release
source: {root, commit}
output_root
runtime: {fathomdb_cli, python, wheel, native_extension, virtualenv}
configs: {ac072, ac073, ac075, scale02, protected_writes, ce_profile, search01, locomo}
assets:
  ir_c: null | {data_root, snapshot, manifest, gold}
  ac073: null | {corpus_root}
  tc5: null | {corpus_root, qualified_manifest, embedder_model_cache}
  locomo: null | {harness_checkout, harness_python, dataset, provenance_manifest}
  ce_profile: null | {reranker_model_cache}
gpu: {enabled, cuda_uuid}
cells: [non-empty, duplicate-free canonical-order subsequence]
timeouts_s: {default, cells: {known-cell-id: positive-integer}}
```

Each runtime/config value is an absolute path or `null`; each asset group is
the exact object shown or `null`. `virtualenv` is a directory; other runtime
bindings are files. Every config binding is a file.

### Exact resolved shape

```text
schema_version, release
source: {root, commit}
source_config: {path, sha256}
output_root
runtime: {same keys; null | {path[, sha256]}}
configs: {same keys; null | {path, sha256}}
assets: {same groups and keys; null | {path[, sha256]}}
gpu: {enabled, cuda_uuid}
cells: [effective configured/CLI-selected cells]
timeouts_s: {each effective cell: expanded positive-integer timeout}
```

When configuration is loaded by later CLI modes, the CLI release, canonical
source root, and canonical output root must equal the validated configuration.
CLI `--cells` must be a subset of `config.cells`; absence uses `config.cells`.
Slice 10 metadata-only dry-run continues to load none of these paths.

## TDD plan

1. RED: add tests for the checked-in example, recursive unknown/missing fields,
   invalid cells/timeouts/GPU coupling, path safety, artifact hashes, missing
   bindings, exclusive output creation, and unchanged metadata-only dry-run.
2. GREEN: implement only strict load/validate/resolve/write functions.
3. REFACTOR: keep cell commands, compatibility rewrites, prerequisite policy,
   and subprocess execution absent; their later slices consume this contract.

## Review and verification plan

- Obtain an independent code-grounded design review before tests/code.
- Obtain independent code review after GREEN.
- Have a separate verification subagent run the focused tests, example load,
  hash/path-safety checks, and regression checks for Slice 10.
- Record evidence and mark complete only after the repository gate is attempted
  and any unrelated baseline failure is identified precisely.

## Evidence

- The initial independent design review returned **CHANGES REQUIRED** for real
  IR-C/TC-5/LOCOMO input shapes, duplicated CLI/config authority, canonical
  path safety, exact resolved shape, generated `perf_gates` lifecycle, and cell
  ordering. After the documented corrections, design re-review returned
  **APPROVE** with no remaining findings.
- RED was captured twice: the first configuration-contract suite reported 23
  failures before implementation; review-driven provenance/atomicity tests then
  reported 11 failures against the first GREEN implementation.
- GREEN: `PYTHONPATH=. .venv/bin/python -m pytest -q
  tests/experiments/test_perf_gauntlet.py` reports 43 passed. The registered
  experiment suite reports 92 passed.
- Independent code review found three blocking gaps: a document/file identity
  mismatch, disconnected CLI subset selection, and non-atomic final writes.
  The path-owned byte snapshot, threaded selection, exclusive temporary file,
  fsync, atomic replace, and failure cleanup resolved them; re-review returned
  **APPROVE**.
- Independent verification found one additional disk-hygiene bug: failed nested
  output creation removed the leaf but left newly created empty ancestors. A
  new RED test reproduced it; leaf-to-root cleanup fixed it. Re-verification
  returned **PASS** with no findings and directly proved that the output root
  and all task-created ancestors are absent after injected failure.
- Ruff, Python compilation, Markdown lint, and diff whitespace checks pass.
- The repository gate remains blocked in its lint tier by the unrelated root
  release-documentation mismatch already recorded in Slice 10:
  `README.md lacks a current published 0.8.26 statement`.
