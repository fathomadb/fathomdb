---
title: FathomDB 0.8.27 hidden-surface oracle - design
status: PROPOSED
target_release: 0.8.27
---

# Hidden-surface oracle design

Requirements and acceptance: `plan.md` (RH-1 to RH-12; eight rustdoc rows, the
release probe, and eight test-inventory rows). Revision 4 closes design review
rounds 1 to 3 (`design-review.md`) and applies the owner ruling
`hidden-surface-effective-and-inventory`, made after implementation showed
per-site signing reports Slice 40's gate moves as differences.

## Shape

A new tool, `dev/tools/hidden_surface.py`, sits beside the Slice 30
comparator. It imports only pure helpers from `surface_comparator.py`
(`canonical_json`, `compare_manifests`, `_unique`, `_run`) and owns its own
scratch, cache, capacity check, and output handling. It never edits the Slice
30 tool, its schema, or its row identities.

```text
hidden_surface.py capture --source-sha <40-hex> --output <new file>
hidden_surface.py export  --source-sha <40-hex>          # prints <root>/exports/<sha>-<n>
hidden_surface.py discard --source-dir <owned export dir>
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

`export` writes to `<root>/exports/<sha>-<n>`, never to `<root>/src`, which
`capture --source-sha` clears. Only an explicit `discard` or `prune` removes an
export. `--source-dir` accepts only directories under `<root>/exports/`.

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
| `use`, not glob, `public` | Local target: record the target under `<path>::<name>` (alias honoured) with the target's signature, folding the `use` site into its effective `hidden` and `cfg`; a module target recurses under the new path. External target (not in `index`): entry kind `external`, signature built from the re-export's own `source` spelling plus effective `hidden` and `cfg` (see below). |
| `use`, glob, `public` | Local module target: expand its public items, skipping names declared explicitly in the importing module. External target: one entry `external-glob:<source>`. |
| struct, union | Entry if `public`; each `public` field is an entry `<Type>::<field>`; impls as below. |
| enum | Entry if `public`; each variant (visibility `default`) is an entry `<Enum>::<Variant>`; impls as below. |
| trait | Entry if `public`; each trait item (visibility `default`) is an entry `<Trait>::<item>`. |
| inherent impl | Each `public` item is an entry `<Type>::<name>`, folding the `impl` site into its effective `hidden` and `cfg`. |
| trait impl | Skip synthetic (auto-trait) impls and blanket instantiations; every other one is an entry `<Type>::impl <Trait<args>>`, with generic arguments rendered. |
| local blanket impl | An impl defined in this crate whose `for` is a generic parameter is an entry `<crate>::impl <Trait<args>> for <T>`. |
| fn, constant, static, type alias, macro | Entry if `public`. |

Each entry is `{path, kind, signature}`, so the shared compare works.

## Signature normalization (RH-3)

`signature` is compact canonical JSON of:

- `visibility`;
- `hidden`: the effective doc-hidden flag, true when any site on the item's
  public path carries `#[doc(hidden)]` (its definition, the re-exporting
  `use`, its `impl` block, or an enclosing module);
- `cfg`: the effective cfg predicate, the canonical conjunction of every
  site's predicate on the same sites (nested `all` flattened, operands sorted
  and de-duplicated; `null` when ungated);
- the item's `inner`, normalized as follows.

Signing effective values, not per-site ones, is an owner ruling
(`hidden-surface-effective-and-inventory`, 2026-09-23). Slice 40 moved gates
from definitions onto re-exports without changing any item's effect, and the
decomposition slices will do the same, so per-site signing reports noise.
What effective values cannot show — a definition left ungated behind a gated
re-export, compiled into default builds as dead code — is caught by the
dead-code lint in clippy `-D warnings` and the warning-free release test-build
gates in `scripts/agent-typecheck.sh`.

Normalization of `inner`:

- Remove every `id`, `span`, `links`, `docs`, and `crate_id` key recursively.
- Rewrite every `resolved_path` reference by target: a local item that is
  publicly reachable becomes its first-found public path; a local item that is
  not reachable becomes `private:<kind>:<name>`, with a `#<n>` suffix only
  when two unreachable items share kind and name (E-4); an external item
  becomes its canonical `paths` path.
- Replace id lists by names or types: struct plain fields become field names
  with each field's effective `hidden` and `cfg`; tuple fields become their
  normalized types (`null` for stripped fields); enum `variants`, trait
  `items`, and impl `items` become sorted name lists; `impls` lists and module
  `items` lists are dropped because their members are entries of their own.
- For constants, drop `expr` and keep `value` and `is_literal`.

**External re-exports (E-2).** In the facade rows every re-exported engine item
is external to the facade's rustdoc JSON, and `paths` holds the engine's
definition location. The facade's promise is its own `pub use` spelling, so an
`external` entry's signature is `{"source": <use.source>, "hidden",
"cfg"}`, with the effective values from the facade-side sites. It changes only when the facade source changes, and the compiler
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
release build. After the rustdoc rows (and after `source_tree_sha256` is
computed), the capture:

1. **Selects items.** From the `engine-default` and `facade-default` rows, it
   selects every entry whose effective `cfg` is false once
   `debug_assertions` and `test` are false and no features are enabled. Those are expected `unresolved`.
   Release-only items (true only under `not(debug_assertions)`) are invisible
   to rustdoc. They come from a curated list in the tool, today only the
   facade's `release_surface_raw_sql_absence_proof`, and are expected
   `resolved`. A source scan fails the capture if any public item gated on
   `not(debug_assertions)` in the exported engine or facade `src/` is missing
   from that list.
2. **Generates the probe.** It writes `examples/hs_probe.rs` into the exported
   engine and facade crates. Cargo discovers examples automatically, so
   neither `Cargo.toml` nor `Cargo.lock` changes. There is one function per
   item, and each form depends on the item's kind:
   - a fn, type, constant, static, or module is named by its public path;
   - an associated item is named `Type::name`;
   - a trait impl is checked by a trait-bound helper, where E0277 means
     absent.

   Struct fields and enum variants are covered by their recorded predicates
   only.
3. **Runs the check.** It runs `cargo check --release --locked --offline -p
   <crate> --example hs_probe --message-format json` and maps each error
   (E0425, E0432, E0433, E0412, E0599, E0277) to its function through the
   primary span's line.
4. **Emits the row.** The `release-probe` row has one entry per item, `kind:
   release-probe`, `signature: resolved | unresolved`. A status that
   contradicts the expectation fails the capture (exit 2).

## Test inventory (RH-11)

A surface oracle cannot see tests. During a move, a wrong `#![cfg(...)]` on a
test file, or a hook gate that no longer matches its tests, can silently stop
tests from compiling or running. For each rustdoc row, the capture runs, from
the same export and cache:

```text
cargo +nightly-2026-04-24 test --locked -p <crate> --no-default-features
  [--features F] --lib --tests -- --list --format terse
cargo +nightly-2026-04-24 test --locked -p <crate> --no-default-features
  [--features F] --lib --tests -- --list --format terse --ignored
```

It emits row `tests-<row>` with one entry per test: `path` is
`<test target>::<test name>` (`lib` for unit tests), `kind` is `test`, and
`signature` is `run` or `ignored`. Doctests are out of scope. The shared
compare reports a removed test as a removal, a newly ignored test as a change,
and a new test as an addition. Cadence policy: a removal or a newly ignored
test blocks the batch unless the slice status names it with its reason; an
addition is expected when characterization tests are added.

## Retirement (RH-12)

The oracle exists to make the 0.8.27 decomposition safe, and it is removed
when that is done, so it does not become a permanent maintenance cost.

- **Trigger:** Slice 150 qualification. Slice 150 runs the final hidden and
  test-inventory comparisons against the current baseline, and records their
  results and manifest digests in this unit's `status.md`.
- **Removed in the same slice:**
  - `dev/tools/hidden_surface.py`;
  - `scripts/tests/test_hidden_surface.py`;
  - `scripts/tests/fixtures/hidden-surface/`;
  - the fast-tier line in `scripts/agent-test.sh`;
  - the committed `baseline-*.json` files;
  - the tool's cache and scratch roots (`hidden_surface.py prune` first).

  Git history keeps all of them.
- **Kept:**
  - the warning-free test-build gates in `scripts/agent-typecheck.sh`;
  - the dead-code lint;
  - the removal-changelog gate;
  - the Slice 30 comparator, whose own lifetime is decided by its own plan.
- **Early unwind:** if the decomposition is abandoned or deferred, the same
  removal runs as part of that decision.
- **Revival:** a later release that decomposes again restores the tool from
  history, re-pins the toolchain, and captures a fresh baseline.

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
- Defect injection: `export --source-sha <e3358800 full>`. In the export,
  remove the `#[cfg(feature = "test-hooks")]` on the definition of
  `decode_dependency_trace_root_for_test` in `dependency_trace.rs` (its body
  needs only `EngineError`). Then split the root group
  `#[cfg(feature = "test-hooks")] pub use dependency_trace::{decode_…,
  encode_…};` into a still-gated `pub use` of `encode_…` and an ungated
  `pub use` of `decode_…`. Run `capture --source-dir` to scratch; the capture
  must exit 0 and the compare must exit 1 with exactly one added
  `engine-default` entry.
- Closeout: capture the implementation `HEAD`; expected equal to the baseline.

## Cadence (RH-7)

`plan-0.8.27.md` changes:

- "Structural slice cadence" step 4 (Slices 40 to 130) reads "Run focused
  tests, the surface comparator, and the hidden-surface and test-inventory
  comparison against the current hidden baseline."
- Slice 140 records its intended hidden differences as a successor baseline.
- Slice 150 re-runs the hidden-surface and test-inventory comparison with the
  immutable public-surface comparator, then performs the retirement (RH-12).

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
