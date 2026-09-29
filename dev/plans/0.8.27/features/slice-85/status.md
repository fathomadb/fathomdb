---
title: FathomDB 0.8.27 Slice 85 - implementation status
status: COMPLETE
target_release: 0.8.27
implemented_on: 2026-09-28
reviewed_candidate: 7a2f9bf90783f545603516502bac0016d4b93a14
closeout_commit: 8cd3389dadba89925d51440b83d05044e87d2090
authoritative_binding: PASS
---

# Slice 85 implementation status

## Design review cycle 1 and FIX-1

The frontmatter binding above is the pre-FIX-1 candidate. An independent
adversarial design review of `7a2f9bf9` (cycle 1) returned FAIL on the
enforcement half of the slice (findings D-1..D-13); the ownership moves stand.
FIX-1 remediates every finding RED-then-GREEN on branch `slice-85-fix`; the
per-finding chronology is in `tdd-chronology.md` under "FIX-1 (design review
cycle 1)". Design review cycle 2 of the FIX-1 head (`51f2a9a3`) found
residual gate gaps (D-14..D-22); FIX-2 remediates them on the same branch,
recorded under "FIX-2 (design review cycle 2)". Design review cycle 3 of the
FIX-2 head (`abcb29b2`) found two macro-hidden edge families and three
smaller gaps (D-23..D-27); FIX-3 remediates them, recorded under "FIX-3
(design review cycle 3)". Design review cycle 4 of the FIX-3 head
(`06a3f712`) found a renamed or re-pathed `include!` and qualified-self serde
paths (D-28, D-29); FIX-4 remediates them, recorded under "FIX-4 (design
review cycle 4)". An adversarial test review of the FIX-4 head (`8385ddca`)
then found untested policy-inventory diagnostics, a stale-binary defect in the
gate wrappers under `CARGO_TARGET_DIR`, and weaker fixture and assertion
evidence (T-1..T-8); test FIX-1 remediates them, recorded under "Test review
FIX-1". Test review cycle 2 of the test FIX-1 head (`809b0e9f`) found a
feature-gated source-scraping test the slice left red, paths into inline
modules that the gate dropped, AC27-85E fixtures that did not compile, and a
false positive in the stale-binary guard (T-9..T-12); test FIX-2 remediates
them, recorded under "Test review FIX-2". Test review cycle 3 of the test
FIX-2 head (`4cf78446`) found that a crate-root re-export of an item in a
`lib.rs` inline module still ended paths on the re-export node, plus two
record defects (T-13..T-15); test FIX-3 remediates them, recorded under "Test
review FIX-3". Test review cycle 4 of the test FIX-3 head (`20468e48`) found
that chained, glob, `self::`-anchored and module-alias crate-root paths still
severed or dropped the forbidden item cycle (T-16); test FIX-4 replaces the
enumerated alias handling with one namespace resolver that fails closed,
recorded under "Test review FIX-4". The release-state candidate is rebound
only after these reviews. The sections below describe the gate after test
FIX-4.

`code-review.md` and `review-verification.md` describe the original
candidate `7a2f9bf9` only; the FIX cycles above supersede their counts.

## Complete on the release branch

Slice 85 is complete on `release/0.8.27`. It assigns the reviewed carriers,
facades, protocol, attribution, telemetry, and narrow errors to durable
semantic owners without changing schema, SQL, ordering, transaction lifetime,
wire encoding, feature gates, or public paths.

- `read_api.rs` and `graph_api.rs` own their `Engine` facades;
- `structural_state.rs`, `reader_transaction.rs`, and `wal_attribution.rs` own
  their leaf responsibilities;
- `reader_pool.rs` owns the private request protocol and typed factories;
- read, filter, graph expansion, search, and telemetry own their handler
  results and carriers; and
- all four inherited Slice 80 cycles are absent. No search-reader error leaks
  into graph/read/filter/frozen-read code.

## Enforced boundary

Normal lint now runs a locked standalone `syn` gate with its own cache rule,
minimal dependencies, MIT license, exact policy, and mutation tests. It
classifies all 71 physical/inline modules and governs 18 boundary modules.

Configurations: the feature set is read from the engine `Cargo.toml`
`[features]` table. The reviewed axis features `test-hooks`,
`tc5-benchmark` and `operator` are enumerated in all combinations. Every
other declared feature gets a single-feature-closure profile. Two reviewed
consumer profiles combine features: `gpu-product` enables `embed-cuda` and
`rerank-cuda`, and `python-dev` enables `default-embedder` and
`default-reranker` (the default `pip install -e` and pytest build from
`src/python/pyproject.toml`). Each of these profiles is crossed with
`operator` on and off. An all-features profile enables every closure. Each
profile point is crossed with test/non-test, Linux/non-Linux and
debug/release, giving 248 evaluated configurations (232 after FIX-2, 144
after FIX-1, 16 before) out of a limit of 512. Feature sets outside this
enumeration are not evaluated as such. Examples are `test-hooks` or
`tc5-benchmark` together with an ML feature (the N-API debug build
`test-hooks,default-embedder`, the private `tc5-benchmark-cuda` build), and
two ML features that no consumer profile combines. Only all-features combines
them, with every other feature also on.

The gate freezes item-level callable, type, re-export, field, Engine-method,
inherent-owner, root re-export and macro-definition identities for every edge
with a governed or root endpoint; edges between two non-governed modules are
extracted for reachability but not frozen. It extracts dependencies:

- inside std macro bodies;
- from module-qualified paths in expression, call and pattern position,
  including turbofish and const-generic arguments and qualified-self
  (`<T as Trait>::f`);
- from struct literals and type positions;
- from trait paths in bounds, `where` clauses, `impl` headers, `dyn`/`impl
  Trait` and supertrait lists;
- from string literals in serde derive helper attributes: `serialize_with`,
  `deserialize_with`, `with`, `skip_serializing_if`, `default`, `getter`
  and `crate` as expression paths, including qualified-self
  (`<T as Trait>::f`, `<T>::f`); `from`, `try_from`, `into` and `remote` as
  types;
  `bound` (and `bound(serialize = …, deserialize = …)`) as `where`
  predicates. These are read on containers, fields and variants, and inside
  `cfg_attr`, whose condition narrows their configurations;
- from any other attribute string literal that is a multi-segment path.

Every path is resolved by one Rust 2018 namespace resolver over every file
and inline module of the crate (test FIX-4, T-16). It follows `crate`,
`self` and `super` anchors, child modules, items declared in that exact
scope, `use` bindings at any visibility, and glob imports. Each written
target is resolved in the scope that declares it. So root and inline-module
re-exports, chained re-exports, root globs of inline and file modules,
`self::`-anchored re-exports and module aliases all end on the declaring
item. A path whose first segment names nothing in its scope is another
crate's, the prelude's or a local binding's. A module named in value
position is a local binding. A `type` alias keeps its own edge and adds one
to every in-crate path its definition names. An associated-type projection
`<Owner as Trait>::Name` names the impl's `type Name = …` binding, which is
chased the same way.

Dot-call receivers are typed syntactically. A constructor types its binding
only when it is declared to return that type, and an in-crate receiver type
is used only when it has the method. Cycles are checked in a whole-crate item
graph and a governed-module graph, and every SCC with a governed member must
account for each governed pair. An untyped dot call in a governed module
also has an item-graph edge to every same-named visible inherent method. In
any module, an untyped dot call (including one admitted by an
`external-receiver` entry) whose same-named inherent methods include one in
a module the source is forbidden to depend on fails as a forbidden
dependency.
Module-level 2-cycles and SCC membership that touch the governed set are a
frozen, regenerable inventory (6 `module-cycle` pairs, 36 `module-scc`
members; test FIX-4 added `graph_expand` and `graph_expand::execution` under
`test`, see "Test review FIX-4" in the chronology). `forbid-dependency` covers descendant modules. `read`, `filter`
and `frozen_read` are forbidden to depend on `search` (and so on
`SearchReaderError`), like `graph_expand`.

The following fail closed, each unless a reviewed, stale-checked,
item-specific policy entry admits it:

- an unknown or undeclared cfg feature on an item, field, variant, impl item,
  `use`/`mod` declaration, statement, expression, match arm or struct-literal
  field, or on a file's declaring `mod` item or inner `#![cfg]`;
- a frozen-scope edge active in no configuration;
- an unparsed macro body in any module (the five reviewed `proptest!`
  bodies are admitted);
- in any module, with no exception, a serde path-key value that is not an
  expression path;
- in any module, with no exception, a macro invocation whose last path
  segment is `include` (`include!`, `std::include!`, `core::include!`,
  `std::prelude::v1::include!`, …) and any `use` that imports an item
  named `include`, renamed or not, alone or in a group: the included source
  is never parsed, and a renamed import would hide the builtin.
  `include_str!` and `include_bytes!` are data, also when renamed;
- in any module, with no exception, an `extern crate` declaration:
  `extern crate self as x` renames this crate so that its paths read as
  another crate's;
- with no exception, a crate-root `use` whose target is neither an in-crate
  item (a file module, a `lib.rs` inline module or a root item) nor a crate
  the engine manifest declares (or `std`, `core`, `alloc`, `proc_macro`,
  `test`). A crate-root re-export of an item in a `lib.rs` inline module
  resolves to that inline module (T-13);
- with no exception, an unresolved in-crate path (test FIX-4, T-16). This is
  a `crate::`/`self::`/`super::` path, or a bare path whose first segment is
  bound in the crate, that does not reach a declared item or module. The
  diagnostic is `unresolved in-crate path … at file:line:col`, and the edge
  still reaches the nearest in-crate namespace, so other diagnostics fire
  too. The same rule fails a destination that is not a declared item of its
  namespace: a module named in value or type position through an import, a
  glob of an in-crate item such as an enum, or a name that two glob imports
  supply differently. It also fails a `use`, in any module, whose head names
  neither an in-crate binding nor a declared dependency;
- in a governed module, an import through a name the crate root only
  re-exports, including one it imports by glob;
- an untyped dot call in any module whose name is a governed inherent method
  of another module. The 233 receiver entries name the receiver expression
  and its type: 128 `external-receiver` entries on types outside the crate,
  and 105 `typed-receiver` entries on in-crate types, whose calls become
  typed edges.

The grammar remains syntactic. Examples of what it does not see:

- a receiver whose type comes from type inference (such receivers are
  handled by the exception rule above, not typed);
- methods a trait provides by default or derives;
- `cfg` attributes on generic parameters, function parameters and pattern
  fields;
- a string path in a non-serde attribute when it is a single segment or has
  generic arguments;
- an associated constant or function reached through `<Owner as
  Trait>::item` in expression position, which resolves to the trait path.

Warm runtime and peak RSS of `scripts/check-module-boundaries.sh` with a
cached binary (host: 24-core x86_64, `/usr/bin/time -v`, three runs after
test FIX-4): 1.98 s / 47,772 KB, 1.98 s / 48,020 KB, 1.98 s / 47,688 KB
(test FIX-3: 2.26–2.34 s / 42.6–42.9 MB; test FIX-2: 2.14 s / 43.2–43.6 MB; test FIX-1: 2.11–2.12 s / 43.5–44.0 MB; FIX-4: 2.11–2.13 s / 43.0–43.7 MB; FIX-3: 2.12–2.14 s / 42.7–42.9 MB;
FIX-2: 2.03–2.05 s / 40.4–40.7 MB; FIX-1: 1.64–1.67 s / 34.8–35.1 MB).
The wrapper and the mutation suite build the gate into
`dev/tools/module-boundary-gate/target` whatever `CARGO_TARGET_DIR` says, so
the binary they run is always the one built from the current source; after a
build the wrapper marks the binary fresh, so a manifest or lockfile edit that
cargo does not relink for is not judged stale again. The gate reads only the engine sources, its
manifest and the policy; it performs no whole-workspace indexing. The gate
crate has 33 library and 5 binary unit tests, and
`scripts/tests/test_module_boundary_gate.sh` runs 240 production mutation
assertions (234 negative, 6 positive) in about 9.2 minutes.

Every fixture family has a compiled negative fixture. A compile check of
every mutant (`cargo check -p fathomdb-engine --lib --profile test
--features test-hooks,operator`) after test FIX-4: 175 of the 240 gate runs
compile and 65 do not. Test FIX-2 added 43 mutants and all 43 compile: 36
`compiled-*` siblings (T-9) and the 7 inline-module bridges (T-11). Test
FIX-3 added 4 and all 4 compile: the crate-root re-export and `extern crate
self` forms (T-13). Test FIX-4 added 18. The 16 item-cycle and module-level
namespace mutants all compile, and each runs after a full regeneration of
every regenerable inventory. The 2 unresolved-path mutants are syntactic
(T-16). The
`compiled-*` mutants add the stub items they name, so each AC27-85E family
has at least one compiling fixture:

- local free-function shadow: `compiled-shadow` (a block-local item);
- unresolved-alias rejection: `compiled-untyped-receiver`,
  `compiled-generic-parameter-receiver`;
- duplicate names: `compiled-overlapping-engine-method-twins`. It compiles in
  the checked feature set, which lacks `tc5-benchmark`; the gate rejects the
  overlap that a `tc5-benchmark` build would also reject;
- missing maps: `compiled-engine-field-missing` (the field is initialised);
- trait references: `compiled-generic-bound`, `-where-bound`,
  `-impl-trait-header`, `-impl-trait-argument`, `-dyn-trait-reference`,
  `-boxed-dyn-trait`, `-supertrait`, `-qself-trait-call`, `-qself-trait-type`;
- relative and generic paths: `compiled-self-super-chain`,
  `compiled-const-generic-turbofish`;
- capitalised paths: `compiled-capital-const`, `-variant`, `-constructor`,
  `-unit`, `-tuple-pattern`, `-struct-pattern` (with the existing compiling
  `capital-struct-literal`), and `compiled-type-kind`;
- macros: `compiled-macro`, `-macro-vec-repeat`, `-macro-params`,
  `-macro-assert-matches`, `-macro-assert-matches-capital`,
  `-macro-matches-guard`, `-macro-format`, `-macro-write`,
  `-macro-statements`, `-macro-unparsed` (with the existing compiling
  `macro-control` and `macro-vec`);
- descendant forbids and item cycles: `compiled-codec-search-api`,
  `compiled-local-helper-cycle`, `compiled-forbidden-submodule-cycle`.

Every other family already had a compiling mutant in the test review
census; for example, `forbidden-receiver-name-collision` covers external
same-name calls. The 63 non-compiling mutants
stay as syntactic evidence, with the 2 T-16 unresolved-path mutants, which
name a missing module and a missing item on purpose. Most name placeholder
items that do not exist
(`slice85_probe`, `S85Trait`, `slice85_make`, …). A few are invalid on
purpose: `shadow` and `overlapping-engine-method-twins` are name
collisions, the two manifest mutants remove a feature, `engine-field-missing`
leaves the new field out of the constructor, `serde-unparsable-path` is
rejected by serde_derive, and no dependency provides the non-serde derive
that `attribute-string-path` uses.

## Residual test risk

These items are still open after test FIX-4, following the cycle 4 test
review's residual list:

1. **T-16, bounded but not proven complete.** The namespace resolver
   replaces the enumerated alias forms, and an in-crate path it cannot
   resolve now fails closed. It is still a hand-written, syntactic model of
   Rust name resolution, and these limits remain:
   - it does not expand macros, so an item or module that a macro generates
     is invisible. An anchored path to one fails closed, but a bare name is
     read as another crate's;
   - a name that is missing from an in-crate namespace holding another
     crate's glob (only `proptest::prelude::*` in test modules today) is read
     as that crate's;
   - privacy is ignored and only the module/item namespace split is
     modelled, so resolution over-approximates. An over-approximation can
     only add edges or fail closed.
2. **Non-compiling mutants.** 65 of the 240 gate runs do not compile (the 63
   from cycle 4 plus the 2 T-16 unresolved-path mutants). They prove the
   parser's behaviour, not that a real edit would be caught. Each AC27-85E
   family still has at least one compiling fixture.
3. **`compiled-overlapping-engine-method-twins`** compiles only because the
   checked feature set lacks `tc5-benchmark`.
4. **Gate-crate clippy debt.** `cargo clippy` on the gate crate, which is
   outside the workspace and not run by `agent-lint`, still reports four
   pre-existing lints: `nonminimal_bool`, `too_many_arguments` on
   `collect_use_tree`, `type_complexity` on `merged_edges` and
   `collapsible_if`. Test FIX-4 added none.
5. **The process-global test-hooks flake** (ledger seq 102) was not
   re-verified.
6. **The `tc5-benchmark` envelope test** still fails under `--features
   tc5-benchmark`. It is pre-existing and ungated (seq 263,
   `TC-3bda074d-71f4-4f06-a15f-a1b17bafb2b0`).
7. **The hand-rolled manifest parser** now also backs the `use` rule in every
   module:
   - a missed dependency name fails closed;
   - an extra name would exempt a `use` whose head matches it.
8. **Rerank's `inner` shorthand.** In the cycle 4 probe, adding an inline
   `mod inner` made the gate attribute rerank's `inner` shorthand to
   `root inner`. The namespace resolver no longer does this. Test FIX-4
   repeated the probe with both a nested inline `inner` and a root `mod
   inner`, and the gate passed with only the three new classifications
   added.
9. **ML and GPU runtime tests** were not run (they need model assets or
   CUDA). Only compilation was checked.

## Acceptance

- **AC27-85A:** every named carrier/facade/error has its approved non-root
  owner. Effective visibility is not widened: FIX-1 returned the four
  `GraphExpandRetentionCountersForTest` fields to private behind a
  `load_relaxed` accessor, and `pub(super)` within a top-level module is
  enumerated and reviewed. The 49 `pub(super)` fields on `wal_attribution.rs`
  structs (`ActualCheckpointObservation` 9, `WalAttributionCollector` 8,
  `NativeConnectionStateFact` 6, `WalCheckpointRecord` 4,
  `WalAttributionSnapshot` 4, `WalAttributionRoleSnapshot` 4,
  `WalAttributionActivity` 3, `NativeStateInventory` 3,
  `BindingNativeStateObservation` 3, `WalAttributionRoleState` 2,
  `ActualCheckpointObserver` 2, `BindingNativeStateObserver` 1) reach exactly
  the crate root and its descendants, the same reach they had as private
  root fields.
- **AC27-85B:** the four Slice 80 cycles and new search/reader-pool cycles are
  prohibited and mutation-tested. "No dependency on `SearchReaderError`" is
  pinned by `forbid-dependency` lines from `read`, `filter` and
  `frozen_read` (and `graph_expand`) to `search`, with one mutant each. The
  gate has no item-level forbid; these module-level lines are stricter.
  `slice60_graph_expand` (19 tests, including the D-9 accessor evidence)
  still requires the `test-hooks` feature and so runs only in the opt-in
  `FATHOMDB_FEATURE_COMPLETE=1` gate. That gating predates the slice and is
  unchanged.
- **AC27-85C/D/E:** exact policy, whole-crate extraction (including paths
  into inline modules of `lib.rs` and other files, T-11, and crate-root
  re-exports of `lib.rs` inline-module items, T-13, and every chained, glob,
  `self::`-anchored and module-alias form through the namespace resolver,
  T-16), item and
  governed-module SCCs, the frozen module-level cycle inventory,
  source-derived inventories, 248 configurations, cache behavior, negative
  grammar fixtures, macro-body extraction and fingerprinting, and production
  mutants pass. Every AC27-85E family has at least one compiled negative
  fixture (175 of 240 gate runs compile; the list is above); the 65
  non-compiling mutants are additional syntactic evidence.
- **AC27-85F:** exact pre-move commissioning evidence is retained. On the
  reviewed candidate, focused runtime/build checks pass, public surface is
  exactly equal, hidden structure and release probe are equal with one
  additive test across 13 inventory rows, and the candidate-bound native
  receipt passes. The `test-hooks` source-scraping test
  `slice60_fix1_wire` was red at that candidate (see the correction in
  `review-verification.md`); test FIX-2 points it at `graph_api.rs`, and every
  non-ML feature-gated engine target passes serially after test FIX-2. One
  lib test, `reader_request_envelope_stays_bounded_as_search_capabilities_grow`,
  fails when the lib is built with `--features tc5-benchmark` (`actual=144
  bytes`, bound 128) at both the commissioned baseline `4c75bfec` and the
  test FIX-2 head `4cf78446`, so Slice 85 did not cause it. No gate runs that
  configuration: the feature-complete gate runs the test under default
  features only, where it passes. It is tracked in the todos ledger as seq
  263, `TC-3bda074d-71f4-4f06-a15f-a1b17bafb2b0` (runs in
  `tdd-chronology.md`, "Test review FIX-2" and "Test review FIX-3").
- **AC27-85G:** no AC-037 qualification is claimed; Slice 150 still owns the
  exact-final-candidate live run.

Independent GPT-5.6 Sol high code review and GPT-5.6 Terra high targeted
verification both returned PASS at the exact reviewed candidate. Detailed
evidence is in `tdd-chronology.md`, `code-review.md`, and
`review-verification.md`.

## Handoff and cleanup

Slice 90 may now consume these settled boundaries; runtime topology,
configuration forwarding, deadlines, pool sizing, shutdown, and open/facade
closure remain Slice 90 work. Tags, registries, publication, and later slices
were not touched.

The release worktree and branch predated this task and are retained. The
task-created virtual environment, native module, candidate surface captures,
and ignored native receipt were removed after their digests were recorded. No
tracked generated artifact is present.
