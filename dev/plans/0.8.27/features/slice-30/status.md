---
title: FathomDB 0.8.27 Slice 30 - implementation status
status: ACTIVE
implemented_on: 2026-09-22
baseline_source_commit: 3b726cba1700e55486d3b2fbb6a922f439f25d54
baseline_tracking_commit: 7927abaefea873433cea5e27d9d6286c06d1e0a0
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
- Clean source commit `3b726cba1700e55486d3b2fbb6a922f439f25d54`
  produced two byte-identical 12-row captures. Commit `7927abae` first tracked
  the reviewed baseline and its source-bound advisory inventory.
- No product runtime, public API, schema, ADR, interface contract,
  `dev/acceptance.md`, feature gate, package root, or transaction/locking
  behavior changed.

The baseline is 10,197,184 bytes and 212,386 pretty-printed JSON lines. Normal
hooks impose no repository large-file prohibition. Its size is intentional:
the six complete Rust rows retain 41,411 explainable cargo-public-api entries,
including blanket impl signatures, rather than opaque signature hashes. The
remaining rows retain 119 Python exports, 79 PyO3 registrations, 538 native
stub entries, 81 production NAPI declarations, 149 TypeScript declarations,
and 64 package/runtime entries. `inventory.md` is the human navigation layer;
the machine baseline is not intended for prompt ingestion.

## Capture and verification receipts

- Both real captures used `cargo-public-api 0.52.0`,
  `nightly-2026-04-24`, Rust `1.97.0-nightly (36ba2c771 2026-04-23)`, Node
  `v26.8.2`, TypeScript `6.0.3`, target `x86_64-unknown-linux-gnu`, and exact
  production NAPI `default-embedder` generation. Each file had SHA-256
  `a8cee76657af6557c2117fad77e66119314b85ef3b77a4c61d31943f33a694fe`;
  byte and semantic comparisons passed with no diff.
- The focused comparator contract passed. SDK parity passed 10/10. Rust
  re-export/governed-surface checks passed 4 default and 4 operator-feature
  tests. Python native-stub/package/parity checks passed 20/20. Focused
  TypeScript no-recovery, release-surface, parity, and package-surface checks
  passed 20/20.
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

## Cleanup and remaining gates

The temporary `.venv` link, generated TypeScript malformed-context fixtures,
checkout-owned `.cache/0.8.27-slice30`, owned `/tmp` scratch, and both temporary
capture files were removed. The user-provided durable release worktree and
branch remain; this slice created neither.

Implementation and local verification are ready for independent code review
and independent verification. This record deliberately remains `ACTIVE` and
does not advance release state or claim Slice 30 complete before those gates.
