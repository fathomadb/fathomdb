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
cycle 1)". The release-state candidate is rebound only after FIX-1 review.
The sections below describe the FIX-1 gate.

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
Its feature set is read from the engine `Cargo.toml` `[features]` table; the
reviewed axis features `test-hooks`, `tc5-benchmark` and `operator` are
enumerated in all combinations, every other declared feature gets a
single-feature-closure profile, and an all-features profile enables every
closure. Each profile is crossed with test/non-test, Linux/non-Linux and
debug/release, giving 144 evaluated configurations (previously 16).

The gate freezes item-level callable, type, re-export, field, Engine-method,
inherent-owner, root re-export and macro-definition identities for every edge
with a governed or root endpoint; edges between two non-governed modules are
extracted for reachability but not frozen. It extracts dependencies inside std
macro bodies and from every module-qualified path, struct literal and pattern;
resolves re-exports to their defining owner; and types dot-call receivers
syntactically. Cycles are checked in a whole-crate item graph and a
governed-module graph, and every SCC with a governed member must account for
each governed pair. Unknown or undeclared cfg features, frozen-scope edges
active in no configuration, unparsed governed macro bodies and unresolvable
receivers of governed inherent methods fail closed, each unless a reviewed,
stale-checked, item-specific policy entry admits it.

Warm runtime and peak RSS of `scripts/check-module-boundaries.sh` with a
cached binary (host: 24-core x86_64, `/usr/bin/time -v`, three runs):
1.64 s / 34,844 KB, 1.67 s / 35,072 KB, 1.65 s / 34,852 KB. The gate reads only
the engine sources, its manifest and the policy; it performs no
whole-workspace indexing.

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
  governed-module SCCs, source-derived inventories, 144 configurations, cache
  behavior, negative grammar fixtures, macro-body extraction and
  fingerprinting, and production mutants pass.
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
