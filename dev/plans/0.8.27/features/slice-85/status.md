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
(design review cycle 3)". The release-state candidate is rebound only after
FIX-3 review. The sections below describe the FIX-3 gate.

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

It resolves `self::`/`super::` chains (through the crate root like `crate::`).
It resolves `use` re-exports, including those inside inline modules, to the
defining owner. A `type` alias keeps its own edge and adds one to every
in-crate path its definition names. An associated-type projection
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
frozen, regenerable inventory (6 `module-cycle` pairs, 34 `module-scc`
members). `forbid-dependency` covers descendant modules.

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
FIX-4): 2.11 s / 43,728 KB, 2.13 s / 43,428 KB, 2.12 s / 43,044 KB (FIX-3:
2.12–2.14 s / 42.7–42.9 MB; FIX-2: 2.03–2.05 s / 40.4–40.7 MB; FIX-1:
1.64–1.67 s / 34.8–35.1 MB). The gate reads only the engine sources, its
manifest and the policy; it performs no whole-workspace indexing. The gate
crate has 30 library and 2 binary unit tests, and
`scripts/tests/test_module_boundary_gate.sh` runs 159 production mutation
assertions (153 negative, 6 positive).

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
  prohibited and mutation-tested.
- **AC27-85C/D/E:** exact policy, whole-crate extraction, item and
  governed-module SCCs, the frozen module-level cycle inventory,
  source-derived inventories, 248 configurations, cache behavior, negative
  grammar fixtures, macro-body extraction and fingerprinting, and production
  mutants pass.
- **AC27-85F:** exact pre-move commissioning evidence is retained. On the
  reviewed candidate, focused runtime/build checks pass, public surface is
  exactly equal, hidden structure and release probe are equal with one
  additive test across 13 inventory rows, and the candidate-bound native
  receipt passes.
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
