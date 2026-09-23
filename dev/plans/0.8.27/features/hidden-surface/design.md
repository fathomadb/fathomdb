---
title: FathomDB 0.8.27 hidden-surface oracle - design
status: PROPOSED
target_release: 0.8.27
---

# Hidden-surface oracle design

Requirements and acceptance: `plan.md` (RH-1 to RH-9, nine rows). Revision 2
closes design review findings D-1 to D-10 (`design-review.md`).

## Shape

A new tool, `dev/tools/hidden_surface.py`, sits beside the Slice 30
comparator. It imports only pure helpers from `surface_comparator.py`
(`canonical_json`, `compare_manifests`, `_unique`, `_run`,
`_check_capture_capacity`) and owns its own scratch, cache, and output
handling. It never edits the Slice 30 tool, its schema, or its row identities.

```text
hidden_surface.py capture --source-sha <40-hex> --output <new file>
hidden_surface.py compare --baseline <file> --candidate <file>
```

`compare` first requires both manifests to carry
`schema: fathomdb.hidden-surface.v1` (a Slice 30 manifest is rejected, D-10),
then delegates to `compare_manifests`, which diffs metadata other than
`capture_source_sha` and per-row entries keyed by `kind:path:sha256(signature)`
and never writes its inputs. Exit codes: 0 equal, 1 unequal, 2 usage or
capture error.

## Capture (RH-1, RH-2, RH-9)

1. Validate `--source-sha`: 40 lowercase hex and `git cat-file -t` is `commit`;
   otherwise exit 2 before any build.
2. Require the pinned nightly `nightly-2026-04-24` (the Slice 30 pin); record
   `rustc --version` and host target.
3. Check capture capacity with the Slice 30 guard, then create a per-capture
   directory with `mkdtemp` under this tool's own root,
   `/tmp/fathomdb-0.8.27-hidden-surface`, which carries its own marker
   `.fathomdb-hidden-surface-owned`. Only that per-capture directory is
   removed afterwards, so a concurrent Slice 30 capture or another worktree's
   capture is never touched (D-6).
4. Export with `git archive <sha> | tar -x -m -C <dir>`. `git archive` reads
   objects only, so the working tree, index, refs, stash, and worktree list
   are untouched. `-m` stamps every extracted file with the current time, so
   cargo's mtime freshness check can never reuse another commit's build of a
   workspace path crate from the shared target directory (D-3).
5. Build each row from the exported tree:
   `cargo +nightly-2026-04-24 rustdoc --locked -p <crate> --lib
   --no-default-features [--features F] [--release] -- -Z unstable-options
   --output-format json --document-hidden-items`, with `CARGO_TARGET_DIR` at
   `.cache/0.8.27-hidden-surface/target` so registry dependencies stay warm.
6. Require `format_version == 57`; any other value exits 2 before a row is
   written (RH-9).
7. Normalize each JSON (below), build the manifest, and write it with an
   exclusive create: write a temporary file in the output directory, then
   `os.link` it to the output path, which fails if the path exists; remove the
   temporary file either way (D-10).

Metadata: `schema`, `capture_source_sha`, `tools.rust-toolchain`,
`tools.rustc`, `rustdoc_format_version`, `target`, and `row_identities` (for
each row: id, crate, features, profile, and the full cargo argument list
including `--locked`).

## Reachability walk (RH-1, D-5)

The walk starts at the root module with public path `<crate>`. It keeps a
cycle guard of item ids on the current chain and a first-found public path
for every local id (breadth-first, shorter paths first, ties broken by
lexicographic order).

| Item | Rule |
| --- | --- |
| module | Entry if `public`; recurse into its `items`. |
| `use`, not glob, `public` | If the target is local: record the target under `<path>::<name>` (alias honoured) with the target's signature and `use_hidden`; if the target is a module, recurse under the new path. If the target is external (not in `index`): entry kind `external`, signature `external:<canonical path>`. |
| `use`, glob, `public` | Local module target: expand its public items, skipping names already declared explicitly in the importing module (explicit shadows glob). External target: one entry `external-glob:<source>`. |
| struct, union | Entry if `public`; each `public` field is an entry `<Type>::<field>`; impls as below. |
| enum | Entry if `public`; each variant (visibility `default`) is an entry `<Enum>::<Variant>`; impls as below. |
| trait | Entry if `public`; each trait item (visibility `default`) is an entry `<Trait>::<item>`. |
| inherent impl | Each `public` item is an entry `<Type>::<name>`, carrying `impl_hidden`. |
| trait impl | Skip synthetic (auto-trait) impls and blanket instantiations; every other one is an entry `<Type>::impl <Trait<args>>`, with generic arguments rendered. |
| local blanket impl | An impl defined in this crate whose `for` is a generic parameter is an entry `<crate>::impl <Trait<args>> for <T>`. |
| fn, constant, static, type alias, macro | Entry if `public`. |

Each entry is `{path, kind, signature}`, so the shared compare works.

## Signature normalization (RH-3)

`signature` is compact canonical JSON of:

- `visibility`;
- `own_hidden`, `use_hidden`, `impl_hidden` — booleans, recorded separately so
  moving or toggling `#[doc(hidden)]` at any site is a difference (D-4);
- the item's `inner`, normalized as follows.

Normalization of `inner`:

- Remove every `id`, `span`, `links`, `docs`, and `crate_id` key recursively.
- Rewrite every `resolved_path` reference by target (D-1): a local item that is
  publicly reachable becomes its first-found public path; a local item that is
  not reachable becomes `private:<defining path>`; an external item becomes
  its canonical `paths` path. Source spelling (`super::X`, `crate::m::X`) and
  definition location therefore never reach a signature.
- Replace id lists by names or types (D-2): struct plain fields become field
  names with each field's `own_hidden`; tuple fields become their normalized
  types (`null` for stripped fields); enum `variants`, trait `items`, and impl
  `items` become sorted name lists; `impls` lists and module `items` lists are
  dropped because their members are entries of their own.
- For constants, drop `expr` (source spelling) and keep `value` and
  `is_literal`.

## Baselines and evidence (RH-5, RH-6)

- First baseline: `capture --source-sha <e3358800 full>` written once to
  `dev/plans/0.8.27/features/hidden-surface/baseline-e3358800.json`.
- Successor baselines (D-8): an intended hidden change (for example Slice 140
  gating) is captured at the landing commit as `baseline-<sha>.json`, next to
  `baseline-<sha>-diff.md`, which lists each difference against the previous
  baseline and its reason. Later comparisons use the newest baseline.
- Slice 40 evidence: capture `5f5c1798` and compare with the baseline;
  expected equal; recorded in `status.md` and cited from
  `slice-40/adversarial-review.md`.
- Defect injection: export `e3358800`, remove the `#[cfg(feature =
  "test-hooks")]` above one `pub use test_hooks::…` re-export, capture that
  tree through the same normalizer (a `--source-dir` option restricted to
  directories under the tool's owned root), and require `engine-default` to
  report the added hook.
- Closeout: capture the implementation `HEAD`; expected equal to the baseline.

## Cadence (RH-7, D-8)

`plan-0.8.27.md` changes:

- "Structural slice cadence" step 4 (Slices 40 to 130) reads "Run focused
  tests, the surface comparator, and the hidden-surface comparison against the
  newest hidden baseline."
- Slice 140 records its intended hidden differences as a successor baseline.
- Slice 150 re-runs the hidden-surface comparison with the immutable
  public-surface comparator.

## Self-tests (RH-8, D-9)

- `scripts/tests/test_hidden_surface.py`, run by `scripts/agent-test.sh` in the
  fast tier.
- Fixture crate `scripts/tests/fixtures/hidden-surface/crate/` with its own
  `[workspace]` table, covering every case listed in `plan.md`'s test plan,
  plus a moved variant with identical public paths.
- `scripts/tests/fixtures/hidden-surface/regenerate.sh` runs the pinned nightly
  rustdoc on both variants, removes host-specific fields
  (`external_crates[*].path`, `target.target_features`), and writes
  canonical JSON. Tests load the committed fixtures and never need the
  nightly.
- Tests cover every ACH-3 difference class, a consistent id renumbering
  (equal), SHA validation, the exclusive-create refusal, the schema check,
  metadata mismatch, the format-version check, and the fixture bytes being
  canonical.

## Risks

- **Nightly JSON drift:** pinned toolchain and format-version check.
- **Build cost:** nine rustdoc builds per capture, with registry dependencies
  cached; captures run per batch and at closeout, not in the fast tier.
- **Fixture staleness:** a toolchain pin change fails the format check until
  the fixtures are regenerated.
