---
title: 0.8.25 Slice 73 — Windows Node/N-API CI design
status: READY
design_version: 2
target_release: 0.8.25
depends_on: 72
---

# Slice 73 design

## Existing substrate and net-new work

The existing `native-artifact-runtime-validation` matrix is the sole build and
package route. Its Windows cell already builds a release N-API module, injects
the matched platform dependency, packs both npm packages, installs them
offline, and runs a basic smoke. Slice 73 adds no product behavior and no new
CI job. It adds only a deterministic retained-test manifest, installed-module
staging/execution to the Windows PowerShell smoke, structural guards, and an
exact local-VM receipt.

## Manifest

`scripts/release/smoke/slice73-windows-napi-modules.json` has schema version 1
and exact ordered arrays for `modules` and `fixtures`. The approved modules
are:

1. `slice15-identity-provenance.test.js`
2. `slice15b-search-validity.test.js`
3. `slice15d-projection-registry.test.js`
4. `slice20-source-dependencies.test.js`
5. `slice22-projection-status.test.js`
6. `slice25-actuation.test.js`
7. `slice30-dependency-closure.test.js`
8. `slice35-frozen-read.test.js`
9. `slice45-pagination.test.js`
10. `slice50-evidence.test.js`
11. `slice50-evidence-response-validation.test.js`
12. `slice55-candidate-native-explanation.test.js`
13. `slice60-graph-expand.test.js`

This selection covers every required behavior class while avoiding the
default-embedder/live-model cases owned by other routes. The manifest names
only these conformance/fixture files read by the selected modules:

1. `src/conformance/provenance-v1.json`
2. `src/conformance/governed-surface-allowlist.json`
3. `dev/fixtures/slice20-dependency-conformance-v1.json`
4. `dev/fixtures/slice25-actuation-conformance-v1.json`
5. `dev/fixtures/slice60-graph-expand-conformance-v1.json`

A structural test holds both human-approved arrays independently, so deleting
an entry from the manifest does not make the oracle follow the deletion.

## Source-independent staging

The PowerShell smoke keeps the current local-tarball installation. After
installation it resolves the main `fathomdb` module and the matched platform
package and rejects either path unless it is inside the disposable consumer
and outside the checkout.

It then compiles the repository tests into a disposable directory shaped like
the repository (`src/ts/dist/tests`). The compiled `src/` subtree is deleted
and replaced with the installed main package's `dist/` bytes. Therefore each
existing relative `../src/*.js` test import reaches installed package bytes,
while the platform loader resolves the installed optional dependency from the
consumer's `node_modules`. Manifest-declared fixtures are copied into their
same relative locations. Missing files, path escape, symlinks/reparse points,
or hash disagreement reject before tests run.

Before execution, the smoke computes a deterministic digest over the sorted
relative file names and per-file SHA-256 values for installed
`node_modules/fathomdb/dist` and for staged `dist/src`. Both trees must contain
only regular files and their digests must be equal. The receipt retains both
digests, making the relative-import staging claim independently auditable.

Each module runs in its own `node --test` process from the disposable
`src/ts` directory. `TEMP` and `TMP` are set to a verifier-owned directory
beneath that disposable root for every process, containing `freshDbPath()` and
all other test-owned temporary files. The runner parses the standard summary
and requires:

- tests greater than zero;
- pass equals tests;
- fail, cancelled, skipped, and todo all equal zero; and
- process exit zero.

The smoke emits a single JSON summary containing manifest hash, resolved paths,
native hash, ordered module results, aggregate counts, and `outcome: pass`.
The workflow log is the raw CI artifact; the local VM log is retained for the
Slice 73 receipt.

## Workflow integration

Only the existing Windows validation step receives the manifest argument.
Linux/macOS invoke the existing Bash smoke unchanged. The job retains the
existing Rust/Python/TypeScript/native-harness routing and pinned actions,
toolchains, target, timeout, and package build steps. CI remains informational;
Slice 75 owns the final exact-head hosted run.

The `native_artifact_harness` path filter additionally names the manifest and
all five fixtures. The existing `src/ts/**` filter already covers selected
test-module changes. Structural mutation tests reject removal of any external
dependency path, so a manifest/fixture-only change cannot bypass the route.

## Evidence and failure policy

`dev/plans/runs/0.8.25-slice-73/receipt.json` records schema version, exact
candidate, archive and transferred archive hashes, VM/OS/toolchain identity,
manifest and fixture hashes, wheel/main/platform/native hashes, resolved
installed paths, installed/staged SDK tree digests, commands, ordered module
results, totals, raw-log hash, cleanup, and outcome. Unknown/missing fields,
identity drift, missing output, or non-pass results cannot support completion.

The VM transfer uses one exact owned temporary directory. The source archive
excludes `.git`, build output, dependencies, virtual environments, and ignored
artifacts. Linux and Windows archive hashes must agree. After the receipt and
independent evidence audit, remove the transferred directory and all exact
owned build/consumer paths.

No product test oracle is changed. A genuine feature failure is corrected in
its owning code through a separate committed RED/GREEN change and focused
review. An environment/setup failure is recorded and repaired without changing
acceptance. No broad rerun follows a docs-only close.
