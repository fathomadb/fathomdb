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
recorded under "FIX-2 (design review cycle 2)". The release-state candidate
is rebound only after FIX-2 review. The sections below describe the FIX-2
gate.

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
other declared feature gets a single-feature-closure profile, and the
reviewed `gpu-product` consumer profile enables `embed-cuda` and
`rerank-cuda`; each of these is crossed with `operator` on and off. An
all-features profile enables every closure. Each profile point is crossed
with test/non-test, Linux/non-Linux and debug/release, giving 232 evaluated
configurations (144 after FIX-1, 16 before). Feature sets outside this
enumeration are not evaluated as such: for example `test-hooks` or
`tc5-benchmark` together with an ML feature, or two ML features that no
consumer profile combines. Only all-features combines them, with every other
feature also on.

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
  Trait` and supertrait lists.

It resolves `self::`/`super::` chains (through the crate root like `crate::`).
It resolves `use` re-exports, including those inside inline modules, to the
defining owner. A `type` alias keeps its own edge and adds one to every
in-crate path its definition names.

Dot-call receivers are typed syntactically. A constructor types its binding
only when it is declared to return that type, and an in-crate receiver type
is used only when it has the method. Cycles are checked in a whole-crate item
graph and a governed-module graph, and every SCC with a governed member must
account for each governed pair. An untyped dot call in a governed module
also has an item-graph edge to every same-named visible inherent method.
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
- an untyped dot call in any module whose name is a governed inherent method
  of another module. The 233 receiver entries name the receiver expression
  and its type: 128 `external-receiver` entries on types outside the crate,
  and 105 `typed-receiver` entries on in-crate types, whose calls become
  typed edges.

The grammar remains syntactic. Examples of what it does not see:

- a receiver whose type comes from type inference (such receivers are
  handled by the exception rule above, not typed);
- methods a trait provides by default or derives;
- attributes on generic parameters, function parameters and pattern fields.

Warm runtime and peak RSS of `scripts/check-module-boundaries.sh` with a
cached binary (host: 24-core x86_64, `/usr/bin/time -v`, three runs after
FIX-2): 2.03 s / 40,404 KB, 2.05 s / 40,688 KB, 2.03 s / 40,660 KB (FIX-1:
1.64–1.67 s / 34.8–35.1 MB). The gate reads only the engine sources, its
manifest and the policy; it performs no whole-workspace indexing. The gate
crate has 27 library and 2 binary unit tests, and
`scripts/tests/test_module_boundary_gate.sh` runs 130 production mutation
assertions (125 negative, 5 positive).

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
  source-derived inventories, 232 configurations, cache behavior, negative
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
