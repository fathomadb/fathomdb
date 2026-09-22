---
title: FathomDB 0.8.27 Slice 30 - implementation status
status: COMPLETE
implemented_on: 2026-09-22
baseline_source_commit: df8017463ecce6281a4c989a2d1bd01118e2025e
baseline_tracking_commit: 346ed5eb95ec1cb168e4a9ad9a0cd951b839cda7
closeout_commit: 4129a5b013265f4672ab1e4ae8ed3dd5cb8cd89f
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

The original 12-row baseline was 10,206,103 bytes and 212,596 lines; the
adversarial-review recapture below supersedes it. Normal
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

## Adversarial review FIX-1 (Phase 1)

A requirements/design adversarial review found the TypeScript row blind to
re-exported declarations (errors, `read`), no Python wrapper signature row,
cfg-blind PyO3 registrations, an unchecked NAPI build identity, and a
mistranscribed hash in `review-verification.md`. RED commit `eb54ca29` and
GREEN commit `4ce45c51` added re-export resolution, the
`python-wrapper-declarations` row, cfg-aware registration entries, and a
fail-closed `build:native` identity check. A clean Node `v25.9.0` capture at
`4ce45c518789108a9fcc1da35fda4c9b74f19aff` produced the 13-row baseline, SHA-256
`24cc30a01539eddc7515072a11c09b45fb9077b2aa34a77b1ed8e7032f4cf3dd`, 10,431,950 bytes and 218,556 lines. Against the prior baseline
it differed only in `row_identities` metadata, the three cfg-gated
registrations, the new 1,131-entry wrapper row, and the TypeScript row
(208 entries: +62 resolved declarations, −3 opaque re-export statements).
All Rust, NAPI, stub, export, and package rows compared equal.

## Adversarial review FIX-2 (Phase 1)

The re-review closed P1-1..P1-5 and found cfg gates on enclosing items not
propagated, JSDoc/`///` comments embedded in declaration signatures (with
brace-bearing comments able to shift statement boundaries), stale closeout
receipts, and a stale plan requirement table. RED `95441a30` and GREEN `58bc8eb4` fixed the adapters. Two clean Node `v25.9.0` captures of
`58bc8eb4836f6f0f52b40b21befaea87a490b9fa` were byte-identical, SHA-256 `b54a01cc486c4d1755297196831f5490d0311237e12ca6f40d37c03af1b3a1b4`, 10,397,378 bytes and
218,556 lines (AC27-30A). Against the FIX-1 baseline `24cc30a01539…` the only
differences were 22 TypeScript and 22 NAPI entries whose signatures lost their
comments. `plan.md` R27-30B/AC27-30C now name every capture arm. The
adversarial reviewer is the reviewer of record for the FIX-1/FIX-2 diff; the
earlier `code-review.md`/`review-verification.md` PASS records cover the
12-row tool at `b102bceb` only.

## Adversarial review Phase 2 and Phase 3

Phase 2 (test review) closed in one cycle with test-only hardening
(`e908b678`). Phase 3 (code review) found bare local TypeScript export lists
opaque to the flattener, Rust items bound to adjacent marker impls because
generic `impl<…>` headers were not recognised, cfg brace counting that
ignored literals and single-line gates, a diff-pairing heuristic that reused
one added entry, a scratch root nothing wrote into, and minor parsing edges.
RED `a8786164` and GREEN `df801746` fixed them: local export lists resolve to
local declarations; any `impl`/`impl<…>`/`unsafe impl` header owns the items
after it; Rust literals and comments are blanked before brace counting, with
multi-line attributes joined and unbalanced input rejected; pairing consumes
each added entry once; TypeScript declarations are emitted into the owned
scratch root; `const enum` is keyed by name; missing adapter inputs raise
`ComparatorError`. Two clean Node `v25.9.0` captures of `df8017463ecce6281a4c989a2d1bd01118e2025e` were
byte-identical, SHA-256 `7db3d883f99940aea69c58079e64d6794be459ef9ab76561106a314e61900384`, 10,678,832 bytes and 218,556 lines.
Against the prior baseline only the four engine Rust rows changed, with equal
added and removed counts (items rebound to their owning impl), plus the
`typescript-declarations` row identity.

Phase 3 FIX-2: the rewritten cfg scanner truncated literals in multi-line
cfg attributes, and same-path diff pairing could lose to processing order.
RED `06ccc6e1` and GREEN `fb2ba9b3` slice both scanner views identically and pair
same-path entries in a first pass. Neither change alters any row of the
tracked baseline (no multi-line cfg in the PyO3 crate; pairing is report-only).
