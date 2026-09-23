---
title: FathomDB 0.8.27 hidden-surface oracle - design
status: PROPOSED
target_release: 0.8.27
---

# Hidden-surface oracle design

Requirements and acceptance: `plan.md` (RH-1 to RH-9).

## Shape

A new tool, `dev/tools/hidden_surface.py`, sits beside the Slice 30
comparator and imports its shared helpers (`canonical_json`,
`compare_manifests`, `_atomic_write`, `_guard_capture_output`, the owned
scratch/cache guards, `_run`). It never edits `surface_comparator.py`'s
schema, row identities, or `REQUIRED_ROW_IDENTITIES`, so the Slice 30 baseline
and its compare path stay byte-for-byte meaningful (plan scope).

```text
hidden_surface.py capture --source-sha <40-hex> --output <new file>
hidden_surface.py compare --baseline <file> --candidate <file>
```

`compare` delegates to `surface_comparator.compare_manifests`, which already
diffs metadata (except `capture_source_sha`) and per-row entries keyed by
`kind:path:sha256(signature)`, pairs removals with additions as changes, and
never writes its inputs (RH-4). Exit codes follow the Slice 30 tool: 0 equal,
1 unequal, 2 usage or capture error.

## Capture (RH-1, RH-2, RH-9)

1. Validate `--source-sha` is 40 lowercase hex and `git cat-file -t <sha>` is
   `commit`; otherwise exit 2 before any build.
2. Check the pinned nightly (`nightly-2026-04-24`, the Slice 30 pin) is
   installed and record `rustc +nightly --version` and host target.
3. Export the commit with `git archive <sha>` into a fresh directory under the
   owned scratch root (`/tmp/fathomdb-0.8.27-slice30/hidden-src`, guarded by
   the existing ownership marker). `git archive` reads objects only: the
   working tree, index, refs, stash, and worktree list are untouched, so a
   dirty checkout and a non-`HEAD` SHA are both fine.
4. For each row, run from the exported tree:
   `cargo +nightly-2026-04-24 rustdoc -p fathomdb-engine --lib
   --no-default-features [--features F] -- -Z unstable-options
   --output-format json --document-hidden-items`, with
   `CARGO_TARGET_DIR` under the owned cache
   (`.cache/0.8.27-slice30/hidden-target`) so repeated captures reuse builds.
   Rows, in order: `rust-engine-hidden-default` (no features),
   `rust-engine-hidden-test-hooks`, `rust-engine-hidden-slice72-test-hooks`,
   `rust-engine-hidden-operator-test-hooks`.
5. Require `format_version == 57` (the pinned nightly's value, measured on
   2026-09-23); any other value exits 2 (RH-9).
6. Normalize each JSON (below) into entries, build the manifest, write it
   atomically to a path that must not exist, and remove the exported tree.

Metadata: `schema: fathomdb.hidden-surface.v1`, `capture_source_sha`,
`tools.rust-toolchain`, `tools.rustc`, `rustdoc_format_version`, `target`, and
`row_identities` (id, crate, feature list, command). Any difference other than
the capture SHA makes a comparison unequal.

## Normalization (RH-1, RH-3)

Walk from the crate root module; an item is reachable when it is `public` and
its parent path is reachable.

- **Modules:** public modules recurse, entry kind `module`.
- **`use` items:** a public `use` with `is_glob: false` records the target item
  under the re-export name at the current path (alias honoured). A glob `use`
  expands to the target module's public items. Targets outside the local index
  (external crates) record kind `external` and signature `external:<canonical
  path from paths>`.
- **Structs, enums, unions, traits:** public fields, variants, and trait items
  are entries under `<Type>::<name>`. Impls listed on the type: inherent impls
  (`trait: null`) contribute their public items as `<Type>::<name>`; non-
  synthetic, non-blanket trait impls contribute one entry
  `impl <Trait> for <Type>` with the trait's canonical path. Synthetic
  (auto-trait) and blanket impls are skipped: they are compiler-derived and
  not part of the seam question.
- **Leaf items:** functions, constants, statics, type aliases, macros.

Each entry is `{path, kind, signature}` as in Slice 30 (so the shared compare
works). `path` is the public path from the crate root (`fathomdb_engine::…`),
not the definition location. `signature` is compact canonical JSON of:

- `visibility`;
- `hidden`: whether the item's own attributes contain `#[doc(hidden)]`;
- the item's `inner` with every `id`, `span`, `links`, `docs`, and
  `crate_id` key removed recursively, and every `resolved_path.path` replaced
  by the canonical `::`-joined path from the JSON `paths` table (so
  `super::X` versus `crate::m::X` spelling does not differ after a move).

Spans, ids, and doc text never reach an entry, so a pure move gives no
difference, while a renamed path, a changed kind or type signature, a
visibility change, or a hidden-flag toggle does (RH-3). Entries are sorted and
de-duplicated by the shared `_unique` rule; rows must be non-empty.

## Baseline and evidence (RH-5, RH-6)

- Baseline: `capture --source-sha e3358800<full>` written once to
  `dev/plans/0.8.27/features/hidden-surface/baseline-e3358800.json` and never
  rewritten; a later intended change is recorded as a reviewed diff, not a
  re-capture over the file.
- Slice 40 evidence: capture `5f5c1798<full>` to scratch and compare with the
  baseline; expected equal. Record the result in `status.md` and cite it from
  `slice-40/adversarial-review.md`, replacing the reviewer-only claim.
- Closeout: capture the implementation `HEAD` and compare with the baseline;
  expected equal, since the post-`e3358800` fixes touched only cfg on a glob
  import, private helpers, tests, and scripts.

## Slice procedure (RH-7)

Add to `plan-0.8.27.md`, in the shared rule for engine decomposition slices
(50, 60, 70, 80, 90) and in Slice 140: after each batch, run the hidden
capture of the candidate and compare it with the hidden baseline next to the
Slice 30 comparison; an unexpected hidden difference blocks the batch; an
intended one (for example Slice 140 gating the test seams) is listed with its
reason in the slice status.

## Self-tests (RH-8)

- `scripts/tests/test_hidden_surface.py`, run by `scripts/agent-test.sh` in
  the fast tier.
- Fixture crate `scripts/tests/fixtures/hidden-surface/crate/` (tiny; features
  `hooks` and `extra`; a doc-hidden fn, a hidden module, named/aliased/glob
  re-exports, a struct with an inherent impl and a trait impl, a feature-gated
  hook) and a moved variant with the same public paths.
- `scripts/tests/fixtures/hidden-surface/regenerate.sh` runs the pinned
  nightly rustdoc over both variants and writes the committed JSON fixtures;
  the tests never need the nightly.
- Negative cases mutate a loaded fixture one field at a time: path, kind,
  signature, visibility, hidden attribute, and format version; plus SHA
  validation, output-exists refusal, and metadata mismatch.

## Risks

- **Nightly JSON drift:** pinned toolchain and format-version check (RH-9).
- **Build cost:** four rustdoc builds per capture (about 1–3 minutes each on
  this host); captures are run per batch, not in the fast tier.
- **Fixture staleness:** fixtures carry their format version; a toolchain pin
  change fails the format check until they are regenerated.
