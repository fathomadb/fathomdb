---
title: FathomDB 0.8.27 Slice 137 — Rust crate consolidation and embedder lockdown (proposed approach)
status: PROPOSED
target_release: 0.8.27
planning_baseline: 2d2f19036
---

# Slice 137 — Rust crate consolidation and embedder lockdown

**Status: PROPOSED.** This document is not a ruling and does not authorize
implementation. It records an approach for the open release decision
`slice-132-external-provider-disposition` (`release-state-0.8.27.json`), which
remains **unruled**. The HITL asked for the approach to be written down
without ruling. Slice 137 is not on the release ladder. If the HITL approves
it, it is added to the ladder at the position the ruling sets.

## Question

Slice 132 shipped `fathomdb-sdk`, a Rust SDK with the Python/TypeScript
surface and no custom-embedder route. That raises two questions:

1. Is the existing `fathomdb` crate still useful?
2. Should external, caller-supplied embedders stay possible at all?

The HITL's stated preference, recorded here as input and not as a ruling, is:
"I do not want external embedders available."

## Current state (at `2d2f19036`)

- **`fathomdb` (published facade).**
  - It re-exports the whole core `fathomdb_engine::Engine` and about 170
    engine types. That puts every inherent core method within reach,
    including `open_with_choice`, the Rust-only search overloads, and the
    debug-build test seams.
  - Its `operator` feature unlocks the recovery seam.
  - Its surface is governed by the BIND-RUST allowlist
    (`ADR-0.8.0-supersede-five-verb-surface-cap.md` Q5, `dev/interfaces/rust.md`).
- **`fathomdb-cli` is the only in-repo dependent of `fathomdb`.** It uses
  `fathomdb = { features = ["operator"] }` to reach `recover` and `doctor`.
- **`fathomdb-sdk` (new in 0.8.27, not yet published).**
  - It is the application surface: it wraps the engine privately and is
    checked at 44/44 by `scripts/check-sdk-surface-parity.py`.
  - Its first crates.io publish needs a one-time HITL token bootstrap
    (todo ledger seq 273).
- **External embedder injection is reachable through crates already published
  on crates.io:**
  - `fathomdb-embedder-api` publishes the `Embedder` trait and promises
    semver stability for third-party implementations in its README.
  - `fathomdb-engine` accepts an implementation through
    `Engine::open_with_choice[_and_config]` with
    `EmbedderChoice::Caller(Arc<dyn Embedder>)` or
    `EmbedderChoice::CallerWithDeviceResolution { .. }`.
  - Retiring `fathomdb` alone does not close this, because both crates must
    stay published as dependencies.
- **Uses of the caller variants inside the repo:**
  - About 50 engine test and test-support sites, plus the `fathomdb-py` and
    `fathomdb-napi` `test_support` modules.
  - Two engine `lib.rs` sites, both inside test modules:
    `CallerWithDeviceResolution` at `lib.rs:1320` and `Caller` at
    `lib.rs:1457`.
  - The unpublished `fathomdb-tc5-benchmark` binary.
  - The documented path for the opt-in cross-vendor ONNX embedder,
    `fathomdb_embedder::OrtBgeEmbedder` (see `ort_bge.rs`).

  Python and TypeScript open only with `EmbedderChoice::Default` or `None`.

## Proposed approach

### Part 1 — retire the facade (choose A or B)

**A. Give the SDK the `fathomdb` name (recommended).**

- Rename the `fathomdb-sdk` crate to `fathomdb`.
  - Its surface, tests, and parity checking are unchanged.
  - Only the package and library names change, together with the docs and
    the release lists (the `fathomdb-sdk` entries added by Slice 132 go away).
- Retire the old facade source.
- Move `fathomdb-cli` onto `fathomdb-engine` directly, with
  `fathomdb-engine/operator`.
- Move or retire the facade's BIND-RUST allowlist and operator-seam tests.
  - The operator re-export pin moves to the CLI or engine boundary.
  - `docs/positions/sdk-parity.md` and the Rust interface docs collapse to one
    Rust contract.
- One crates.io name then matches the PyPI and npm `fathomdb` packages.
- The release breaks Rust users of the facade. It is pre-1.0, and AGENTS.md
  forbids a backwards-compatibility shim.
- The new-crate first-publish bootstrap goes away, because `fathomdb` already
  exists on crates.io with trusted publishing.
- Cost:
  - It reverses the 2026-10-06 direction for a separately named crate.
  - It needs a successor ADR to `ADR-0.8.27-rust-sdk-parity.md` and to BIND-RUST.

**B. Keep `fathomdb-sdk` and retire `fathomdb`.**

- Publish one final `fathomdb` release with a deprecation README that points
  to `fathomdb-sdk`, then remove the crate from the publish lists.
- Move `fathomdb-cli` onto `fathomdb-engine`, as in A.
- The seq 273 token bootstrap is still required.
- This keeps the 2026-10-06 naming and leaves the ecosystem names asymmetric.

### Part 2 — lock down external embedders (both options)

1. **Engine API.**
   - Gate `EmbedderChoice::Caller` and `CallerWithDeviceResolution`, and any
     open path that accepts them, behind `cfg(any(test, feature = "test-hooks"))`.
     A published default build then offers only `Default` and `None`.
   - Move the engine test sites and the binding `test_support` sites to
     `test-hooks` builds (`required-features` where needed).
   - The unpublished TC-5 benchmark opts into the feature.
2. **Withdraw the public plugin contract.**
   - Rewrite the `fathomdb-embedder-api` and `fathomdb-engine` READMEs and
     crate docs to say "internal workspace crate; no external stability
     promise; use `fathomdb`/`fathomdb-sdk`".
   - The `Embedder` trait stays `pub` because the workspace crates implement
     it across crate boundaries. With no public injection point in the
     engine, nobody outside the workspace can install an implementation.
3. **Decide the ONNX path.**
   - Either promote ONNX to a first-party `EmbedderChoice` (for example an
     `Onnx` variant behind the existing `fathomdb-embedder` `onnx-embedder`
     feature, forwarded through the engine and resolved the way `Default` is),
   - or drop the external ONNX route.

   This is a HITL choice (decision 3 below).
4. **Prove absence.**
   - Add `compile_fail` absence proofs (pinned error codes) that
     `EmbedderChoice::Caller` and `CallerWithDeviceResolution` do not resolve
     in a default-feature consumer of `fathomdb-engine` or of the SDK.
   - Add a negative fixture to the hidden-surface probe that Slice 150 owns.

## Decisions required before approval

1. **Rule `slice-132-external-provider-disposition`**: retire external
   embedder injection, or keep it as a supported extension boundary.
2. **Crate naming:** option A or B.
3. **ONNX:** first-party choice, or removal.
4. **Sequencing:**
   - Slice 137 changes the engine's open API, so it must not run in parallel
     with other engine-editing work. Slice 135, performance qualification, is
     in progress.
   - Proposed position: after Slice 135 and before Slice 140, so that
     documentation convergence (140) and qualification (150) see the final
     crate set. It could also move to 0.8.28 if 0.8.27 should not grow.

## Blast radius (for planning, if approved)

- **Crates:** `fathomdb`, `fathomdb-sdk` (A renames it), `fathomdb-cli`
  manifest and imports, `fathomdb-engine` `open.rs` and its tests, and the
  `test_support` modules of `fathomdb-py` and `fathomdb-napi`.
- **Embedder crates:** the `fathomdb-embedder-api` README and docs, and the
  `fathomdb-embedder` ONNX documentation.
- **Contracts:**
  - a successor ADR or ADRs;
  - `dev/interfaces/{rust,rust-sdk}.md`, plus `cli.md` if the CLI's boundary
    text changes;
  - `docs/positions/sdk-parity.md`, the `docs/` Rust pages, and
    `CHANGELOG.md` (`### Removed` for every removed public symbol, AC-050c);
  - the `ADR-0.6.0-embedder-protocol` posture.
- **Release machinery:**
  - Axis-W lists in `set-version.sh` and its tests, `PUBLISHABLE_CRATES`, the
    publish-helper dependent list and its test, and the `release.yml` T6/T7
    jobs;
  - the license-consistency counts, the test-feature matrix, the
    public-doc-truth member count, and the AGENTS.md crate list;
  - for option A, the removal of seq 273.
- **Governance:**
  - the facade governed-surface tests (AC-074, BIND-RUST);
  - the parity checker's `--rust-crate` path;
  - the signed allowlist and pin, which change only if option A changes a
    signed member (expected to be unchanged).

## Acceptance sketch (if approved)

- A default-feature consumer of the published Rust crates cannot construct
  or pass a caller-supplied embedder: the `compile_fail` proofs pass. Python
  and TypeScript are unchanged.
- `fathomdb-cli` builds and passes its recovery and doctor tests on the engine
  directly.
- Under A, `cargo add fathomdb` yields the SDK surface (44/44 parity).
  Under B, `fathomdb` is deprecated and no longer in the publish lists.
- The ONNX outcome matches the decision and has tests.
- Strict `./scripts/agent-verify.sh` passes on the candidate.
