---
title: 0.8.25 Slice 73 — Windows Node/N-API CI coverage
status: READY
depends_on: 72
design: design.md
---

# Slice 73 plan

## Outcome

Make the existing Windows x64 native-artifact CI cell execute a fixed,
source-independent sample of the retained TypeScript/N-API contracts from
Slices 15–60. Preserve one Windows build/package/install route and hand an
exact-candidate local-VM receipt to Slice 75. Slice 75 still owns the full
local matrix and final exact-head hosted CI.

## Reconciliation since the draft

1. Slices 71 and 72 are complete. The release state names Slice 73 as the next
   slice, and Slice 72's generic preflight is available unchanged.
2. `.github/workflows/ci.yml` already has a Windows entry in
   `native-artifact-runtime-validation`. It builds a release Python wheel and
   N-API module, packs matched npm packages, installs them locally, and runs a
   small dependency/search smoke. A second build matrix is unnecessary.
3. `smoke-local-native-artifacts.ps1` does not yet run retained feature test
   modules or prove that its JavaScript and native module resolve outside the
   checkout. The POSIX script's broader feature coverage does not close that
   Windows gap.
4. The TypeScript suite now contains stable feature-local modules through
   Slice 60. Prior local Windows work proved only the then-selected feature
   slices; it is retained historical evidence, not continuous CI coverage.
5. The handoff at `88d21040` predates the owner split. Its focused Windows
   Node/N-API requirement belongs here. Its integrated exact-head hosted-CI
   requirement remains exclusively in Slice 75.
6. Package manifests intentionally remain at 0.8.24 until release-cut work.
   Slice 73 must not perform a version bump, release rehearsal, registry
   access, tag, publication, or merge to `main`.

The draft is approved with these adjustments. No product API, schema,
database behavior, ranking, CUDA route, or live-model path is in scope.

## Requirements

- **S73-R1 — Fixed representative allowlist.** A checked-in manifest names
  every selected compiled test module and fixture explicitly. No glob or
  runtime directory discovery may change membership.
- **S73-R2 — Installed bytes.** Build the existing Windows release N-API
  artifact and matched npm packages, install them from local tarballs into an
  isolated consumer, and execute the selected modules against the installed
  JavaScript plus installed platform package. The resolved main-module and
  native-module paths must be under the disposable consumer and outside the
  source checkout.
- **S73-R2a — Staged-byte equality.** Compute the same deterministic file-tree
  digest over installed `node_modules/fathomdb/dist` and the disposable
  `dist/src` mirror, require equality before tests run, and retain both values.
- **S73-R3 — Retained contracts.** The selected modules cover provenance and
  governed search, projection declaration/status, dependency registration,
  atomic actuation and dependency closure, frozen eligibility, pagination and
  unknown-version refusal, exact evidence, explanation validation, and
  constrained graph expansion. Use real SQLite databases for runtime tests.
- **S73-R4 — Honest execution.** Run each module in a fresh Node process.
  Require a non-zero test count, zero failures, zero cancellations, zero
  skips/todos, and equality of passed and discovered tests. A missing module,
  fixture, artifact, or summary field fails closed.
- **S73-R5 — Existing CI route.** Extend only the Windows cell of
  `native-artifact-runtime-validation`. Preserve its dependency-accurate
  routing, package identity, pinned tools/actions, Linux/macOS behavior, and
  informational single-maintainer CI model.
- **S73-R5a — Dependency-accurate routing.** Changes to the manifest or any
  manifest fixture must select the native-artifact validation route. Structural
  mutation tests hold those path-filter entries in place.
- **S73-R6 — Durable receipt.** Retain exact candidate/source-archive identity,
  Windows host and toolchain identity, manifest/fixture/artifact hashes,
  package resolution paths, commands, per-module counts, total counts, exit
  state, and cleanup result. Slice 75 may consume this receipt but may not
  treat it as final exact-head hosted CI.
- **S73-R7 — Focused boundary.** No broad regression, CUDA, live-model,
  registry, release-workflow dispatch, publication, or unrelated platform
  campaign belongs here.

## Acceptance criteria

1. A structural RED test fails on the current tree and then proves the exact
   allowlist, manifest-to-workflow/script wiring, Windows-only execution,
   source-independent resolution checks, per-module fresh-process loop,
   zero-test/skip/failure guards, and unchanged proportional routing.
2. `actionlint` and the focused workflow/routing tests pass after GREEN.
3. An exact committed source archive is hashed on Linux and the authorized
   Windows VM, then built with the pinned repository Node/Rust configuration.
4. The isolated local-tarball consumer resolves both `fathomdb` and
   `fathomdb-native-win32-x64-msvc` outside the transferred checkout, and the
   native hash agrees with the built artifact. Installed and staged SDK tree
   digests are equal.
5. Every manifest module executes on the Windows VM with non-zero tests and
   zero failed, cancelled, skipped, or todo tests. The retained log and receipt
   bind all results to the exact implementation commit.
6. Independent read-only design review and code review pass. A separate
   read-only verifier audits retained evidence without launching a duplicate
   campaign.
7. `status.md`, the receipt, and release-state views truthfully close Slice 73,
   name Slice 75 next, and record removal of exact verifier-owned temporary
   paths.

## Implementation and TDD

### RED

Add `scripts/tests/test_slice73_windows_napi_ci.py` first. Freeze the intended
module list and structural behavior in that test, run it against the current
tree, retain the failure, and commit RED before implementation.

### GREEN

Add the manifest and the smallest PowerShell/workflow changes needed to:

1. pack and install the already-built matched packages;
2. compile test modules into a disposable mirror;
3. replace the mirror's compiled SDK files with the installed package's files;
4. copy only manifest-declared fixtures with preserved relative paths;
5. reject checkout-local module/native resolution;
6. bind `TEMP` and `TMP` beneath the verifier-owned root for every test;
7. run each manifest module separately and validate its Node summary; and
8. emit one machine-readable summary for the receipt.

Do not alter a retained feature test to obtain GREEN. If a selected module is
not portable for a demonstrated reason, correct the manifest through an
explicit oracle amendment and independent review rather than silently
dropping it.

### Focused verification

Run the structural test, actionlint, proportional-routing test, PowerShell
static analysis when available, TypeScript compile, and the existing native
smoke structural checks. Then run one exact-source Windows VM campaign. No
full `agent-verify`, Rust workspace regression, heavy/all tier, or hosted PR
matrix is required by Slice 73.

After GREEN, obtain independent code review of the exact implementation commit
and a separate evidence audit. Write the status/release-state close as a docs
commit; compiled evidence remains bound to the implementation commit.
