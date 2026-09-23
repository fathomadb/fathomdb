---
title: FathomDB 0.8.27 hidden-surface oracle - design
status: PROPOSED
target_release: 0.8.27
---

# Hidden-surface oracle design

Requirements and acceptance: `plan.md` (RH-1 to RH-10; eight rustdoc rows plus
the release probe). Revision 3 closes design review rounds 1 and 2 (D-1 to
D-10, E-1 to E-5; `design-review.md`).

## Shape

A new tool, `dev/tools/hidden_surface.py`, sits beside the Slice 30
comparator. It imports only pure helpers from `surface_comparator.py`
(`canonical_json`, `compare_manifests`, `_unique`, `_run`) and owns its own
scratch, cache, capacity check, and output handling. It never edits the Slice
30 tool, its schema, or its row identities.

```text
hidden_surface.py capture --source-sha <40-hex> --output <new file>
hidden_surface.py export  --source-sha <40-hex>          # prints a persistent owned dir
hidden_surface.py capture --source-dir <owned export dir> --output <new file>
hidden_surface.py compare --baseline <file> --candidate <file>
```

`compare` requires both manifests to carry
`schema: fathomdb.hidden-surface.v1` (a Slice 30 manifest is rejected), then
delegates to `compare_manifests`, which diffs metadata other than
`capture_source_sha` and per-row entries keyed by `kind:path:sha256(signature)`
and never writes its inputs. Exit codes: 0 equal, 1 unequal, 2 usage or
capture error.

## Capture (RH-1, RH-2, RH-9)

1. Validate `--source-sha`: 40 lowercase hex and `git cat-file -t` is
   `commit`; otherwise exit 2 before any build.
2. Require the pinned nightly `nightly-2026-04-24` (the Slice 30 pin); record
   `rustc --version` and host target.
3. Check free space on this tool's own cache and scratch filesystems.
4. Take an exclusive `flock` on the tool's root,
   `/tmp/fathomdb-0.8.27-hidden-surface` (marker
   `.fathomdb-hidden-surface-owned`), so captures that share the cache run one
   at a time. The Slice 30 scratch root is never touched.
5. Export to the stable directory `<root>/src` (cleared first): `git archive
   <sha> | tar -x -m -C <root>/src`. `git archive` reads objects only, so the
   working tree, index, refs, stash, and worktree list are untouched. `-m`
   stamps every file with the current time, so cargo cannot reuse another
   commit's build of a workspace path crate. The stable path keeps cargo unit
   hashes stable, so the cache at `.cache/0.8.27-hidden-surface/target` does
   not grow per capture; `hidden_surface.py prune` deletes the cache.
6. Build each rustdoc row with the working directory set to the export and no
   absolute paths in the arguments:
   `cargo +nightly-2026-04-24 rustdoc --locked -p <crate> --lib
   --no-default-features [--features F] -- -Z unstable-options
   --output-format json --document-hidden-items`. Read
   `<target>/doc/<crate>.json` immediately after each run, before the next row
   overwrites it.
7. Require `format_version == 57`; any other value exits 2 before a row is
   written.
8. Run the release probe (below).
9. Normalize, build the manifest, and write it with an exclusive create: write
   a temporary file in the output directory, `os.link` it to the output path
   (fails if the path exists), and remove the temporary file either way.

Metadata: `schema`, `capture_source_sha`, `source_modified` (true only for
`--source-dir`, with `source_tree_sha256` of the exported files),
`tools.rust-toolchain`, `tools.rustc`, `rustdoc_format_version`, `target`, and
`row_identities` (id, crate, features, profile, cargo arguments). A
`--source-dir` capture refuses an output path under
`dev/plans/0.8.27/features/hidden-surface/`, so an injected tree can never
become a baseline.

## Reachability walk (RH-1)

The walk starts at the root module with public path `<crate>`. It keeps a
cycle guard of item ids on the current chain and a first-found public path
for every local id (breadth-first, shorter paths first, ties broken by
lexicographic order).

| Item | Rule |
| --- | --- |
| module | Entry if `public`; recurse into its `items`. |
| `use`, not glob, `public` | Local target: record the target under `<path>::<name>` (alias honoured) with the target's signature plus `use_hidden` and `use_cfg`; a module target recurses under the new path. External target (not in `index`): entry kind `external`, signature built from the re-export's own `source` spelling plus `use_hidden` and `use_cfg` (see below). |
| `use`, glob, `public` | Local module target: expand its public items, skipping names declared explicitly in the importing module. External target: one entry `external-glob:<source>`. |
| struct, union | Entry if `public`; each `public` field is an entry `<Type>::<field>`; impls as below. |
| enum | Entry if `public`; each variant (visibility `default`) is an entry `<Enum>::<Variant>`; impls as below. |
| trait | Entry if `public`; each trait item (visibility `default`) is an entry `<Trait>::<item>`. |
| inherent impl | Each `public` item is an entry `<Type>::<name>`, carrying `impl_hidden` and `impl_cfg`. |
| trait impl | Skip synthetic (auto-trait) impls and blanket instantiations; every other one is an entry `<Type>::impl <Trait<args>>`, with generic arguments rendered. |
| local blanket impl | An impl defined in this crate whose `for` is a generic parameter is an entry `<crate>::impl <Trait<args>> for <T>`. |
| fn, constant, static, type alias, macro | Entry if `public`. |

Each entry is `{path, kind, signature}`, so the shared compare works.

## Signature normalization (RH-3)

`signature` is compact canonical JSON of:

- `visibility`;
- `own_hidden`, `use_hidden`, `impl_hidden`: booleans, one per site;
- `own_cfg`, `use_cfg`, `impl_cfg`: the cfg predicate at each site, or
  `null`;
- the item's `inner`, normalized as follows.

Normalization of `inner`:

- Remove every `id`, `span`, `links`, `docs`, and `crate_id` key recursively.
- Rewrite every `resolved_path` reference by target: a local item that is
  publicly reachable becomes its first-found public path; a local item that is
  not reachable becomes `private:<kind>:<name>`, with a `#<n>` suffix only
  when two unreachable items share kind and name (E-4); an external item
  becomes its canonical `paths` path.
- Replace id lists by names or types: struct plain fields become field names
  with each field's `own_hidden` and `own_cfg`; tuple fields become their
  normalized types (`null` for stripped fields); enum `variants`, trait
  `items`, and impl `items` become sorted name lists; `impls` lists and module
  `items` lists are dropped because their members are entries of their own.
- For constants, drop `expr` and keep `value` and `is_literal`.

**External re-exports (E-2).** In the facade rows every re-exported engine item
is external to the facade's rustdoc JSON, and `paths` holds the engine's
definition location. The facade's promise is its own `pub use` spelling, so an
`external` entry's signature is `{"source": <use.source>, "use_hidden",
"use_cfg"}`. It changes only when the facade source changes, and the compiler
checks that it resolves. Engine rows still resolve references to
`fathomdb-query` and `fathomdb-schema` items through their definition paths;
those crates are not decomposed in 0.8.27, so this is accepted.

**Cfg predicates (E-1).** Rustdoc JSON keeps each item's cfg as an attribute
(`#[attr = CfgTrace([...])]`, the pinned nightly's debug format). The capture
strips span text and parses it into a canonical predicate string: `name`,
`name = "value"`, `not(p)`, `any(...)`, `all(...)`, with `any`/`all` operands
sorted. An attribute that does not parse fails the capture (exit 2), so a
toolchain format change cannot silently drop gates. Dropping or widening any
gate, including `debug_assertions`, becomes a signature change in the rows
that still compile the item.

## Release probe (RH-10)

Rustdoc always enables `debug_assertions`, so no rustdoc row can show a
release build. After the rustdoc rows, the capture:

1. Selects every engine or facade entry whose predicate at any site requires
   `debug_assertions` to be true (expected `unresolved` in release), and every
   facade item whose predicate requires `not(debug_assertions)` (expected
   `resolved`; found by a release-profile `cargo check` of the facade in the
   same export, since rustdoc cannot see it).
2. Generates a consumer crate in the export directory that depends on the
   crate by path, with one function per selected item naming its public path.
3. Runs `cargo check --release --locked --message-format json` and maps each
   `unresolved` diagnostic to its function.
4. Emits the `release-probe` row: one entry per item, `kind:
   release-probe`, `signature: resolved | unresolved`. A status that
   contradicts the item's predicate fails the capture (exit 2).

## Baselines and evidence (RH-5, RH-6)

- First baseline: `capture --source-sha <e3358800 full>` written once to
  `dev/plans/0.8.27/features/hidden-surface/baseline-e3358800.json`.
- Successor baselines: an intended hidden change (for example Slice 140
  gating) is captured at its landing commit as `baseline-<sha>.json`, next to
  `baseline-<sha>-diff.md`, which lists each difference against the previous
  baseline and its reason. The baseline to compare against is the one whose
  capture commit is the most recent ancestor of `HEAD`, so parallel branches
  never pick up each other's baselines.
- Slice 40 evidence: capture `5f5c1798` and compare with the baseline;
  expected equal; recorded in `status.md` and cited from
  `slice-40/adversarial-review.md`.
- Defect injection (E-3): `export --source-sha <e3358800 full>`, remove the
  `test-hooks` gate at both the definition (`dependency_trace.rs`) and the
  root re-export of `decode_dependency_trace_root_for_test` (its body needs
  only `EngineError`), `capture --source-dir` to scratch, and require exit 0
  for the capture and exit 1 for the compare with exactly one added
  `engine-default` entry.
- Closeout: capture the implementation `HEAD`; expected equal to the baseline.

## Cadence (RH-7)

`plan-0.8.27.md` changes:

- "Structural slice cadence" step 4 (Slices 40 to 130) reads "Run focused
  tests, the surface comparator, and the hidden-surface comparison against the
  current hidden baseline."
- Slice 140 records its intended hidden differences as a successor baseline.
- Slice 150 re-runs the hidden-surface comparison with the immutable
  public-surface comparator.

## Self-tests (RH-8)

- `scripts/tests/test_hidden_surface.py`, run by `scripts/agent-test.sh` in the
  fast tier.
- Fixture crate `scripts/tests/fixtures/hidden-surface/crate/` with its own
  `[workspace]` table, covering every case in `plan.md`'s test plan, plus a
  moved variant with identical public paths and a facade-shaped second crate
  that re-exports the first.
- `scripts/tests/fixtures/hidden-surface/regenerate.sh` runs the pinned nightly
  rustdoc, removes host-specific fields (`external_crates[*].path`,
  `target.target_features`), prunes `paths` to referenced ids, and writes
  canonical JSON. Fixtures reproduce byte-for-byte on the recorded host
  triple. Tests never need the nightly.
- Tests cover every ACH-3 difference class, a consistent id renumbering
  (equal), the facade move case (equal), cfg parsing and its failure, SHA
  validation, the exclusive-create and baseline-directory refusals, the schema
  check, metadata mismatch, and the format-version check. The release probe's
  diagnostic mapping is tested on recorded `cargo check` JSON output.

## Risks

- **Nightly JSON drift:** pinned toolchain, format-version check, and a
  fail-closed cfg parser.
- **Build cost:** eight rustdoc builds plus two release checks per capture;
  captures run per batch and at closeout, not in the fast tier.
- **Fixture staleness:** a toolchain pin change fails the format check until
  the fixtures are regenerated.
