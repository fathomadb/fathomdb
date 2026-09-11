---
title: Slice 85 — final verification and packaging design
status: APPROVED
---

# Slice 85 design

## Boundary

Slice 85 is an evidence-producing release gate. It owns no new product feature,
public API, schema migration, performance treatment, version cut or publishing
action. The candidate is the reviewed release tree; closeout-only records may
follow it and must identify that distinction. Existing Slice 75 runners remain
the execution mechanisms. One small final-manifest validator is justified to
make completeness and false-pass rules executable; a new scheduler is not.

## Evidence model

`dev/plans/runs/0.8.25-slice-85/manifest.json` is the final inventory. Each row
has a unique ID, legacy/additional origin, relevant input set, disposition,
candidate/source identity, command or retained receipt, raw-log paths, positive
counts, and verdict. Dispositions are `run`, `rerun`, `reuse`, `unavailable`,
or `blocked`. Only `reuse` and successful `run`/`rerun` may satisfy an
obligation. `unavailable` is valid only for an explicitly authorized exception;
the sole initial exception is AC-034c. `blocked` is honest incomplete evidence.

The candidate identity has three layers:

- source commit and tracked-tree digest for broad/CI/docs gates;
- relevant-input digest for retained Slice 72/73/79/80 evidence;
- artifact SHA-256 plus build flags/toolchain for installed consumers.

Closeout documents do not rewrite the frozen source/product identity. A product,
test, runner, workflow, packaging or public-doc correction creates a new
candidate and invalidates rows whose input sets include that change.

## Reuse decisions

- AC-081a/b/c and AC-072 reuse Slice 80 because changes after their accepted
  source are documentation-only and the recorded product/build inputs match.
- Protected 71B writes rerun: Slice 80 changed their inherited broad
  `src/rust/crates/fathomdb-engine/src` invalidation set.
- Slice 72 CE and Slice 73 Windows receipts are not presumed applicable: Slice
  79 changed runtime initialization and installed SDK surfaces. Their candidate
  routes must rerun or exact-candidate hosted/native coverage must subsume them.
- The CE rerun uses a copied Slice 85 manifest whose only permitted difference
  from the immutable Slice 72 manifest is `candidate_sha=FINAL_SHA`; its
  baseline, fixture, features, model identity, repetitions and thresholds stay
  byte-equivalent and are checked before execution.
- Slice 76/77 profiles are historical explanation only. AC-020 is retired.
- Feature-local source tests may be consumed by the single broad round; no
  duplicate focused run is required unless the broad output lacks a positive
  selector/count or an installed-binding interaction.

## Execution order

1. Validate the inventory and preflight tools, caches, runtimes and executors.
2. Run manifest/runner short smokes, lint, full all-target check/clippy, the one
   `agent-verify --tier=all` round and strict MkDocs.
3. Run uncovered integration and long reliability/performance/model cells,
   serializing timing and builds.
4. Build Linux CPU artifacts once; consume those exact bytes in fresh Python,
   Node and CLI processes and runtime-floor/GLOBAL-01 checks.
5. Run or collect exact-candidate Linux CUDA, native five-platform, Windows,
   Tegra and hosted-CI evidence. Hosted dispatch checks out the supplied SHA;
   therefore the candidate must be remotely reachable, but pushing requires
   the repository's normal explicit authorization.
6. Independently audit code and evidence, then close or record blockers.

The frozen variable bindings are `RUN_DIR=dev/plans/runs/0.8.25-slice-85`,
`FINAL_SHA=git rev-parse HEAD` at the clean GREEN commit, CPU artifact paths
under `$RUN_DIR/artifacts/{python,napi,cli}`, and CUDA outputs under
`$RUN_DIR/cuda-*`. Local x64 owns broad, long, model, CPU-package, GLOBAL-01,
CE and 71B execution; GitHub Actions owns Linux ARM64, macOS x64/ARM64 and the
exact-SHA aggregate; `gh-runner-wonl-win11` owns Windows x64; the registered
Jetson workflow owns Tegra. The historical manifest's command arrays and
timeouts are adopted byte-for-byte except for the command arrays, feature sets,
timeouts and positive counts in the execution matrix's sealed-routes table.
There is no unresolved command or feature-set choice.

## TDD and fixes

The validator RED/GREEN cases are specified in the plan. Tests use temporary
manifests and never edit retained receipts. If execution discovers a defect,
first add the narrowest failing test at the owning layer, demonstrate RED,
implement GREEN, refactor, and run blast-radius checks. A product/runner fix
does not authorize a second full round: only affected rows rerun unless the
change touches the broad runner or cross-cutting runtime behavior.

## Failure and safety rules

- Preserve failed and stopped logs; never convert absence to success.
- Never edit substantive historical oracles or accepted performance limits.
- Never use editable Python installation from the worktree.
- No tag, release, registry write, Pages publication or merge to main.
- Missing remote access, GPU, runtime floor or exact-candidate hosted evidence
  blocks readiness rather than shrinking the matrix.
- After closure, merge only to `release/0.8.25`, verify Git, and remove the
  temporary Slice 85 branch/worktree.

## Review boundary

Design review checks requirement coverage, reuse soundness, invalidation rules,
TDD falsifiability and absence of overbuild. Code review checks the validator,
manifest and any bounded fix. Independent verification reruns the validator,
audits Git/artifact identities and samples raw evidence before accepting status.
