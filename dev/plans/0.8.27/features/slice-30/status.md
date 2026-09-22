---
title: FathomDB 0.8.27 Slice 30 - implementation status
status: COMPLETE
implemented_on: 2026-09-22
baseline_source_commit: add4f3f4f066f0c4b1b47d7a91c1a4f979eba6bf
baseline_tracking_commit: d80a66236ca9dc0dd6456e2a0d5322c3722d364c
---

# Slice 30 implementation status

## Implemented scope

- Genuine RED commit `d1cf4481` defined deterministic capture, all required
  mutation arms, row identity mismatch, and duplicate-manifest rejection before
  the tool existed.
- GREEN/tool commits `61e52258` and `07c8bf55` implemented one explicit
  capture/compare entry point with exact tool/target/row metadata, clean-tree
  and source-SHA validation, owned resource roots, immutable comparison, and
  complete diagnostics.
- Real `cargo public-api` output found three fixture gaps. RED/GREEN pairs
  `7eeb4d59`/`6eeeffd8`, `d823f2e2`/`273d3efb`, and
  `8631f0eb`/`3b726cba` added associated-path overloads, trait-qualified
  identical method signatures, and repeated identical compiler-entry
  normalization without weakening duplicate-manifest rejection.
- Initial code review found five comparator/evidence defects. RED commit
  `f621c9cd` specified source-SHA-independent semantic comparison, complete
  Python alias/inheritance capture, exact tool handling, missing-executable
  errors, hardlink-safe immutability, atomic writes, and gate registration.
  GREEN commit `add4f3f4` closed them without product-code changes.
- Clean source commit `add4f3f4f066f0c4b1b47d7a91c1a4f979eba6bf`
  produced two byte-identical 12-row captures. Commit `d80a6623` tracks the
  corrected reviewed baseline and its source-bound advisory inventory.
- No product runtime, public API, schema, ADR, interface contract,
  `dev/acceptance.md`, feature gate, package root, or transaction/locking
  behavior changed.

The baseline is 10,206,103 bytes and 212,596 pretty-printed JSON lines. Normal
hooks impose no repository large-file prohibition. Its size is intentional:
the six complete Rust rows retain 41,411 explainable cargo-public-api entries,
including blanket impl signatures, rather than opaque signature hashes. The
remaining rows retain 119 Python exports, 121 PyO3 registrations, 538 native
stub entries, 81 production NAPI declarations, 149 TypeScript declarations,
and 64 package/runtime entries. `inventory.md` is the human navigation layer;
the machine baseline is not intended for prompt ingestion.

## Capture and verification receipts

- Both corrected real captures used `cargo-public-api 0.52.0`,
  `nightly-2026-04-24`, Rust `1.97.0-nightly (36ba2c771 2026-04-23)`, Node
  `v25.9.0`, TypeScript `6.0.3`, target `x86_64-unknown-linux-gnu`, and exact
  production NAPI `default-embedder` generation. Each file had SHA-256
  `7c76bd0d409cecc5dc073e29baf7806b329379bd9475c7ab138dda12cde5cdf2`;
  byte and semantic comparisons passed with no diff.
- A fresh candidate from tracking commit
  `d80a66236ca9dc0dd6456e2a0d5322c3722d364c` had SHA-256
  `c78b89da8c40be4821c9e25fa53dd6a834de79ef4b7d917079a35d0c512c5413`.
  It compared semantically equal with empty metadata/row diffs while reporting
  the distinct baseline and candidate provenance SHAs.
- The focused comparator contract passed. SDK parity passed 10/10. Rust
  re-export/governed-surface checks passed 4 default and 5 operator-feature
  tests. Python native-stub/package/parity checks passed 20/20. Focused
  TypeScript no-recovery, release-surface, parity, and package-surface checks
  passed 20/20.
- The review-requested TypeScript release-surface route was rerun under Node
  `v25.9.0` with `RELEASE_SURFACE_TESTS=1`: its production native no-test-hook
  inspection executed and passed. TAP reported 2/2; the default-embedder open
  branch returned early under the explicitly recorded
  `FATHOMDB_SKIP_NETWORK_TESTS=1` condition and is not claimed as exercised.
- The canonical gate's lint, Rust/Python/TypeScript type checks, strict
  unconfined security checks (0 violations, 0 blockers, 0 downgrades), Rust
  suite, TypeScript suite, and 118 repository harness suites passed. The full
  Python suite reported 1,527 passed and 27 skipped; its sole failure was the
  ownership guard correctly rejecting the temporary worktree `.venv` link
  needed by Pyright. After removing that link, the unchanged failed test passed
  1/1. No full gate was rerun a fourth time merely to combine those already
  observed results.
- The sandboxed NAPI build initially refused its `/bin/sh` helper and AC-036
  initially refused ptrace. The unchanged commands passed through the approved
  unconfined route. Runtime Python native introspection was intentionally not
  collected because editable worktree installation is forbidden; source
  registration, native stub, and package export rows are all present.

## Independent review, cleanup, and closeout

The temporary `.venv` link, generated TypeScript malformed-context fixtures,
checkout-owned `.cache/0.8.27-slice30`, owned `/tmp` scratch, and both temporary
capture files were removed. The user-provided durable release worktree and
branch remain; this slice created neither.

Independent code review first rejected three P1 and three P2 findings. RED
commit `f621c9cd`, GREEN commit `add4f3f4`, corrected baseline commit
`d80a6623`, and evidence commit `b102bceb` closed all six. The same reviewer
returned PASS. Independent reverification also returned PASS: a fresh Node
25.9.0 capture at `b102bceb` compared equal to the immutable baseline despite
its distinct valid provenance SHA; focused parity and surface suites passed;
and the baseline hash, counts, size, and provenance matched this record.

Slice 30 is complete on `release/0.8.27`. Slice 40 and all later slices, main
integration, tags, registries, qualification, and publication remain separate.
