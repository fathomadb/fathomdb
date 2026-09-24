---
title: FathomDB 0.8.27 hidden-surface oracle - design
status: COMPLETE
target_release: 0.8.27
---

# Hidden-surface oracle design

Requirements and acceptance: `plan.md` (RH-1 to RH-15). Revision 7 closes
design review rounds 1 to 6 (`design-review.md`) and applies the owner rulings
`hidden-surface-effective-and-inventory` and `feature-complete-test-coverage`.
The first ruling came after implementation showed that per-site signing
reports Slice 40's gate moves as differences.

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
- `cfg`: the effective cfg predicate, the conjunction of every site's
  predicate on the same sites, signed in the semantic canonical form defined
  under "Cfg predicates" below (`null` when ungated);
- the item's `inner`, normalized as follows.

Signing effective values, not per-site ones, is an owner ruling
(`hidden-surface-effective-and-inventory`, 2026-09-23). Slice 40 moved gates
from definitions onto re-exports without changing any item's effect, and the
decomposition slices will do the same, so per-site signing reports noise.
What effective values cannot show is a definition left ungated behind a gated
re-export, compiled into default builds as dead code. The rustc `dead_code`
lint catches it when that definition has no other live caller. The gate that
enforces this is the default-feature `-Dwarnings cargo check -p
fathomdb-engine --all-targets` in `scripts/agent-typecheck.sh`, plus its
release test-build siblings. Workspace clippy does not enforce it for
`operator`-gated items, because workspace feature unification enables
`operator`. Residual gaps, verified on the pinned nightly:

- a definition that also has an ungated internal caller;
- a definition under `#[allow(dead_code)]` or `#[allow(unused)]`, of which
  there are 26 sites in the engine and facade `src/`;
- a definition referenced from an ungated static table.

These stay with review.

Effective values are defined over visible sites only. Private modules are
absent from the rustdoc index, so a private module's `cfg` or `doc(hidden)` is
not seen; row presence still reflects it. A field's effective values combine
its own sites with its parent type's effective values. An `external-glob`
entry carries the effective values of its glob `use` site.

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
strips span text and parses it into a predicate over atoms (`name`,
`name = "value"`) with `not`, `any`, and `all`. An attribute that does not
parse fails the capture (exit 2), so a toolchain format change cannot silently
drop gates. The effective predicate is canonicalized semantically, not by
syntax: evaluate its truth table over its sorted atoms, then sign the minimal
sum-of-products form, with terms and literals sorted; when several minimal
forms exist, sign the lexicographically smallest. So
`all(any(feature = "test-hooks", test), feature = "test-hooks")` and
`feature = "test-hooks"` sign identically, and dropping a redundant gate is not
a difference. Gates in this workspace have at most about five atoms. More than
twelve atoms fails the capture rather than being approximated. Dropping or widening any
gate, including `debug_assertions`, becomes a signature change in the rows
that still compile the item.

## Release probe (RH-10)

Rustdoc always enables `debug_assertions`, so no rustdoc row can show a
release build. After the rustdoc rows (and after `source_tree_sha256` is
computed), the capture:

1. **Selects items.** From the `engine-default` and `facade-default` rows, it
   selects every entry whose effective `cfg` is false once
   `debug_assertions` and `test` are false and no features are enabled.
   The evaluator knows only `debug_assertions`, `test`, and `feature = "..."`.
   Any other atom (for example `unix` or `target_os`) fails the capture
   instead of being guessed. Those are expected `unresolved`.
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
tests from compiling or running.

**Static target list (RH-13).** The `test-targets` row has one entry per test
target in every workspace crate, with `path` `<crate>::<target>`, `kind`
`test-target`, and a signature holding its source file, its
`required-features`, and any file-level `#![cfg(...)]` predicate. It is read
from `Cargo.toml` `[[test]]` entries plus auto-discovered `tests/*.rs` files,
with no build. A dropped, renamed, or re-gated target is a difference even
when nothing compiles it.

**Inventory rows.** Inventory rows are listed for each rustdoc row's crate and
features (`tests-<row>`). There is also one `tests-req-<n>` row for every
distinct requirement set in `test-targets` (the `required-features` union and
any file-level feature cfg) that no rustdoc row already compiles. Those sets
are derived, not hand-kept, and `<n>` is the canonical feature list; they are
the same sets as the RH-14 gate's committed matrix, which is regenerated from
this derivation and drift-checked by RH-15. CUDA sets
are included: NVIDIA tools are authorized on this host. This makes every
capture depend on this host's CUDA environment (the same preflight as the
RH-14 gate) and adds about a minute of CUDA build per capture. Per row:

1. `cargo +nightly-2026-04-24 test --locked -p <crate> --no-default-features
   [--features F] --lib --tests --no-run --message-format json` gives every
   test executable with its target name and kind.
2. Each executable is run with `--list --format terse`, and again with
   `--ignored`. Output is read per binary, so tests with the same name in two
   targets stay distinct.

Each entry's `path` is `<target>::<test name>` (`lib` for unit tests), `kind`
is `test`, and `signature` is `run` or `ignored`. Doctests are out of scope.
The heavy check requires every `test-targets` entry to appear in at least one
inventory row. The shared
compare reports a removed test as a removal, a newly ignored test as a change,
and a new test as an addition. Cadence policy: a removal or a newly ignored
test blocks the batch unless the slice status names it with its reason; an
addition is expected when characterization tests are added.

## Feature-complete test gate (RH-14, permanent)

`scripts/test-feature-complete.sh` is the gate that runs what the workspace
gate cannot. It shares one requirement reader with the coverage check (RH-15):
a small module, `scripts/lib/test_targets.py`, reads `Cargo.toml` and
`tests/`.

**Committed inputs (G-2).**

- `scripts/test-feature-matrix.toml`: the crate and feature sets the gate runs.
  It has two tables, and a new set in either shows up as a reviewed diff, never
  as a silent addition:
  - `[[entry]]`: the requirement sets of test targets, derived from source.
    `test_targets.py --write-matrix` regenerates them.
  - `[[extra]]`: sets for tests behind an item-level `#[cfg(feature = ...)]`
    on a feature that no target requires. They are derived from test listings
    (see G-1), and `test-feature-complete.sh --write-matrix` regenerates them.
- `scripts/test-skip-allowlist.toml`: every test the gate may legitimately
  not run. Each entry is a test id or target plus a class and a reason. The
  classes are:
  - `ignored-by-design`: for example Slice 72's watchdog child entry point,
    which must not run directly, and the TC-20 hard-gate body documented as
    never running;
  - `opt-in-experiment`: for example the `ir_c_*` targets, which need
    `IRC_RUN` and gitignored gold files. A single-test entry may set
    `exclude = true` so the gate never runs that test. It is used for
    `calibration_reports_p1_flips_and_p2_l2`, which rewrites a committed
    calibration record. The gate runs that test's assertions through
    `calibration_cpu_baseline_components_hold`, which writes nothing. The entry
    is stale unless the test is listed;
  - `benign-message`: a harmless message that contains a skip word;
  - `platform-excluded`: a whole target whose file-level cfg excludes some
    hosts. Its `excluded_on` host predicate says where the entry applies, so the
    coverage check stays clean on every host.

**Environment preflight (G-3).** The gate fails before building unless:

- `nvcc` is found at the repo's CUDA root, and the gate sets
  `PATH`, `CUDA_ROOT`, `CUDA_PATH`, and `LIBRARY_PATH` itself;
- with `CUDA_DEVICE_ORDER=PCI_BUS_ID` and `CUDA_VISIBLE_DEVICES=0,1`, the
  devices at those indices are the two RTX 3090s. The K620 is never
  selected. The checks reuse `scripts/check-cuda-release-contract.py` where it
  applies.

**Model weights (G-4).** It builds `fathomdb-cli` with `default-embedder` and
runs `doctor warm-cache` to download the embedder. The reranker has no warm
verb, so its weights download on first use inside the tests; a failed download
shows as a skip, which the skip contract turns into a failure. The gate unsets
`FATHOMDB_SKIP_NETWORK_TESTS` and sets the runner variables tests need, for
example `FATHOMDB_SLICE72_RUNNER=approved-nvidia` and
`FATHOMDB_SLICE72_RECEIPT_DIR` under its own scratch directory.

Every other asset is pinned and verified before use:

- **nomic-embed-text-v1.5** for `nomic_smoke`: fetched at revision `e9b67630`
  into `<cache>/fathomdb/embedders/nomic-v1.5`, the root the test resolves
  with `dirs::cache_dir()`. `model.safetensors` is checked against its LFS
  sha256 and `tokenizer.json` against its git blob sha1. The embedder crate
  has no nomic fetcher.
- **Slice 72:** its asset root is staged from the warmed embedder and
  reranker caches, and its target gets exactly one visible device (the first
  RTX 3090).
- **ONNX, for the `onnx-embedder` targets:**
  - `libonnxruntime.so.1.26.0` is extracted from the onnxruntime 1.26.0 wheel
    (wheel sha256 and library sha256 both pinned);
  - the bge-small ONNX graph is checked against its pinned sha256 and
    re-exported with `dev/tools/onnx/export_bge_small_onnx.py` when missing.
    The export runs in a throwaway virtual environment that installs
    `dev/tools/onnx/export-requirements.txt` with `--require-hashes
    --no-deps`, so every package is an exact, hash-pinned version. The export
    is byte-deterministic with that set;
  - the tokenizer is the one the embedder loader pins.

  The gate sets `ORT_DYLIB_PATH`, `FATHOMDB_ONNX_MODEL_PATH`, and
  `FATHOMDB_ONNX_TOKENIZER_PATH`.

**Run and skip contract (G-1).** The gate first lists every test binary of
each crate, lib unit tests included. It builds once under the workspace gate's
features, once under each `[[entry]]` set, and once under the union of the
crate's host-buildable features (`cargo build --tests --keep-going`), then
runs each executable with `--list`.

A feature is host-buildable unless it reaches, through any workspace crate's
feature graph, a dependency feature that only builds elsewhere. Cargo cannot
express that, so `test_targets.PLATFORM_DEPENDENCY_FEATURES` records each such
dependency feature with the cfg of the hosts where it builds. Today these are
the Candle Metal backends (`target_os = "macos"`).

A test listed under the union but under neither the workspace features nor an
`[[entry]]` set is gated on a feature no target requires. The gate lists the
crate's single host-buildable features, smallest feature closure first then by
name, and gives the test the first one that lists it. If none does, the test
gets the union. These derived extra sets must equal the matrix's `[[extra]]`
sets for the crate; a difference fails the gate.

A test that appears under an `[[entry]]` or extra set but not under the
workspace features is run once, under the smallest set that has it. This
covers whole feature-gated targets and item-level `#[cfg(feature = ...)]`
tests inside targets the workspace gate runs. A binary that fails to build
fails the gate. Per crate and set the gate runs:

```text
cargo test --locked -p <crate> --no-default-features --features <set> \
  <--lib | --test <target> | --bin <name>>... \
  -- --exact <tests>... --nocapture --test-threads=1
```

Listing and runs set `CARGO_PROFILE_{DEV,TEST}_DEBUG=0` and
`CARGO_INCREMENTAL=0` to bound disk use.

One thread per binary keeps each test's output contiguous, so every skip
marker is attributed to exactly one test id (the harness's `test <name> ...`
line precedes it).

Then:

- **Skip markers:** the output after the first test binary starts is matched
  against `\bskip(s|ped|ping)?\b` (any case) and `\bPENDING(_EXTERNAL)?\b`.
  That covers `[SKIP] ...`, `SKIP name: ...`, `skipping ...`, and
  `gated-to-skip`. Any match fails the gate unless its test id is on the
  allowlist with class `opt-in-experiment` or `benign-message`.
- **Ignored tests:** the per-target ignored count is compared with the
  `ignored-by-design` entries. Any ignored test not on the allowlist fails
  the gate.
- **Stale entries:** an allowlist entry that no longer matches any test fails
  the gate.
- **Result:** "passes" means every non-allowlisted test ran and passed.
- **Counts:** planned, passed, failed, and ignored are counted per planned
  test, from the status the gate attributes to it, and written to
  `summary.json`. They are never summed from `test result` lines, because a
  test that re-executes its own binary prints its child's results. A planned
  test with no status counts as failed, and one that never appears fails the
  gate.

A later hardening step can replace marker matching with a shared
`FATHOMDB_REQUIRE_LIVE=1` helper that turns a skip into a panic; that is not
part of this unit.

**Where it runs:** not in `agent-verify`, for runtime reasons.
`scripts/check.sh` runs it when `FATHOMDB_FEATURE_COMPLETE=1`. Slice 70, Slice
150, and release qualification also run it.

## Test-target coverage check (RH-15, permanent)

`scripts/check-test-target-coverage.py` runs in the fast tier, with no build.
It reads every test target's requirements through `scripts/lib/test_targets.py`.
Those requirements are `required-features` plus any file-level `#![cfg(...)]`,
parsed by its own small parser for Rust source cfgs, which evaluates
`feature`, `debug_assertions`, `test`, and platform atoms per gate profile and
host. The check fails, naming the target or entry, when:

- a target's requirements are met neither by the workspace gate nor by an
  entry in the committed matrix outside the allowlist. The workspace gate's
  per-crate features come from `cargo metadata --filter-platform <host>`
  feature resolution for the workspace, which is what `cargo test
  --workspace` unifies;
- the requirement sets derived from the source differ from the committed
  matrix;
- an `[[extra]]` set names an unknown crate or undeclared features, or
  duplicates an `[[entry]]` set. Whether the extra sets are complete is checked
  only by the gate, because that needs test listings;
- an allowlist entry is stale.

It also prints the targets that only the feature-complete gate covers.
Self-tests cover:

- an uncovered `required-features` target;
- an uncovered file-level `#![cfg(feature = ...)]` target;
- a matrix drift;
- a stale allowlist entry.

## Retirement (RH-12)

The oracle exists to make the 0.8.27 decomposition safe, and it is removed
when that is done, so it does not become a permanent maintenance cost.

- **Trigger:** Slice 150 qualification, as its last step, after the whole
  qualification has passed. Retiring earlier would strand Slice 150's loop of
  sending defects back to their owning slice. Slice 150 first runs the final
  hidden and test-inventory comparisons against the current baseline, and
  records their results and manifest digests in this unit's `status.md`.
- **Plan text:** the retirement commit rewrites the cadence step 4 and Slice
  150 wording in `plan-0.8.27.md` to say that the oracle ran and was retired,
  citing the `status.md` digests, so that no text points at a deleted tool.
- **Removed in the same slice:**
  - `dev/tools/hidden_surface.py`;
  - `scripts/tests/test_hidden_surface.py`;
  - `scripts/tests/fixtures/hidden-surface/`;
  - the fast-tier line in `scripts/agent-test.sh`;
  - the committed `baseline-*.json` files;
  - the tool's cache and scratch roots (`hidden_surface.py prune` first).

  Git history keeps all of them.
- **Kept:**
  - the feature-complete test gate (RH-14), the coverage check (RH-15), their
    shared `scripts/lib/test_targets.py`, and their committed inputs
    `scripts/test-feature-matrix.toml` and `scripts/test-skip-allowlist.toml`:
    these are what let the code carry itself once the oracle is gone;
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
  expected equal on every rustdoc and release-probe row. Test-inventory
  differences are the tests Slice 40 added, each listed. The result is
  recorded in `status.md` and cited from `slice-40/adversarial-review.md`.
- Defect injection: `export --source-sha <e3358800 full>`. In the export,
  remove the `#[cfg(feature = "test-hooks")]` on the definition of
  `decode_dependency_trace_root_for_test` in `dependency_trace.rs` (its body
  needs only `EngineError`). Then split the root group
  `#[cfg(feature = "test-hooks")] pub use dependency_trace::{decode_…,
  encode_…};` into a still-gated `pub use` of `encode_…` and an ungated
  `pub use` of `decode_…`. Run `capture --source-dir` to scratch; the capture
  must exit 0 and the compare must exit 1 with exactly one added
  `engine-default` entry.
- Closeout: capture the implementation `HEAD` and compare it with the
  baseline. Known differences, such as the Slice 40 review's `test` term on
  `execute_for_test` and the test targets it re-gated, are recorded as the
  successor baseline `baseline-<HEAD>.json` with `baseline-<HEAD>-diff.md`.

## Cadence (RH-7)

`plan-0.8.27.md` changes:

- "Structural slice cadence" step 4 (Slices 40 to 130) reads "Run focused
  tests, the surface comparator, and the hidden-surface and test-inventory
  comparison against the current hidden baseline." A following paragraph
  states the inventory policy: a removed or newly ignored test blocks the
  batch unless the slice status names it with its reason, and an added test is
  expected with new characterization coverage.
- Slice 70, whose reranking and embedding code the feature-gated targets
  exercise, also runs the feature-complete test gate (RH-14).
- Slice 140 records its intended hidden differences as a successor baseline.
- Slice 150 re-runs the hidden-surface and test-inventory comparison with the
  immutable public-surface comparator, runs the feature-complete test gate,
  and, as its last step after qualification passes, performs the retirement
  (RH-12).

The implementation's existing edits to `plan-0.8.27.md` (commit `f96c97a3`)
predate revisions 4 and 5 and are updated to this text.

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
