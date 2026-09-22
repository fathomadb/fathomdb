---
title: FathomDB 0.8.27 Slice 30 - surface comparator design
status: PROPOSED
target_release: 0.8.27
---

# Slice 30 surface comparator design

## Boundary and artifacts

One repository tool owns capture, normalization, and comparison. Its tracked
baseline is normalized JSON under this slice directory; a separate reviewed
inventory records navigation and exceptions. Generated compiler/build output
stays in the Slice 10 owned cache and scratch roots and is never an oracle.

The baseline metadata contains its schema, capture-source Git SHA, tool
versions, target, and ordered feature-row identities. Capture runs from the
clean pre-baseline implementation commit and fails if the tree is dirty or
`HEAD` differs from the recorded source. The later commit that first tracks the
baseline is recorded separately in slice status so the manifest never falsely
claims to describe its own later commit. Comparison fails closed on metadata
or row mismatch and reports added, removed, and changed entries.

## Capture rows

- **Rust:** invoke `cargo public-api 0.52.0` with
  `nightly-2026-04-24`. Ordered row IDs and arguments are
  `rust-facade-default` (`-p fathomdb --no-default-features`),
  `rust-facade-operator` (`-p fathomdb --no-default-features --features
  operator`), `rust-engine-default` (`-p fathomdb-engine
  --no-default-features`), `rust-engine-operator` (same plus `--features
  operator`), `rust-engine-test-hooks` (same plus `--features test-hooks`), and
  `rust-engine-operator-test-hooks` (same plus `--features
  operator,test-hooks`). Normalize complete public paths and signatures so
  re-export changes and items gated by both engine features remain visible.
- **Python:** parse the real package export declarations, PyO3 module
  registration calls, and `_fathomdb.pyi` declarations as three independently
  named rows. When runtime native introspection is available from a clean
  artifact it may add evidence, but an editable worktree install is forbidden
  and unavailability is not a pass.
- **NAPI/TypeScript:** regenerate the production NAPI declaration with
  `npm run build:native`, whose manifest expands to `napi build --platform
  --release --cargo-cwd ../rust/crates/fathomdb-napi --features
  default-embedder --js false`, and record that command/features as the
  `napi-production` row identity. Parse that declaration and the TypeScript
  compiler-emitted declaration surface separately. Capture supported
  package-root/subpath entrypoints from package metadata and corresponding
  generated runtime export keys without treating private source module names
  as contract. Existing test-hook leak suites remain the owner for the debug
  artifact; Slice 30 does not make a debug NAPI artifact a published surface.

Adapters emit a common ordered structure of row name, symbol path, kind, and
normalized signature/value. They do not embed an expected symbol allowlist;
the reviewed baseline is the sole full-surface expectation.

## Non-vacuity and immutability

Fixture tests copy minimal real-shaped inputs and apply one controlled mutation
per required arm: Rust add, Rust remove, re-export change, an
operator-plus-test-hooks-only Rust change, Python registration removal, Python
stub removal, NAPI declaration change, TypeScript export or declaration change,
and package export/path change. Each must produce the specific row-level
difference while an unchanged control compares equal. Metadata fixtures also
prove that a wrong row identity cannot reuse another row's surface.

The comparator never rewrites the approved baseline during comparison. Capture
is an explicit operation to a caller-selected output, and replacing the tracked
baseline requires human review. Slices 40-130 consume comparison only; Slice
150 owns any authorized rebaseline or full installed-package qualification.

## Advisory inventory

`inventory.md` records the source-bound path/line inventory, top-level symbols,
direct dependencies, co-change neighbors, focused owning tests, boundary maps,
and exceptions. These aid move planning but do not determine comparator success.
No threshold on file size, module count, or movement batch size becomes a test.

## Failure and resource behavior

Missing tools, failed compiler/declaration generation, ambiguous parsing,
duplicate normalized keys, unsupported feature combinations, dirty generated
output, and metadata mismatch are errors. The tool preserves subprocess exit
status and diagnostics. Heavy capture begins only after the 100 GB check and
uses checkout-owned `.cache/0.8.27-slice30` plus
`/tmp/fathomdb-0.8.27-slice30`; cleanup removes only proven owned output.

This is repository tooling and evidence only. It changes no runtime behavior,
public API, schema, feature gate, package root, transaction boundary, or ADR.
