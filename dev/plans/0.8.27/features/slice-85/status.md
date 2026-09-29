---
title: FathomDB 0.8.27 Slice 85 - implementation status
status: COMPLETE
target_release: 0.8.27
implemented_on: 2026-09-28
reviewed_candidate: 7a2f9bf90783f545603516502bac0016d4b93a14
authoritative_binding: PASS
---

# Slice 85 implementation status

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
classifies all 71 physical/inline modules, governs 18 boundary modules, and
evaluates 16 Linux/non-Linux, feature, and test configurations. It freezes
item-level executable, contract/type, field, method, inherent-owner, root
re-export, and macro-definition identities. Unknown predicates and unreviewed
syntax fail closed.

## Acceptance

- **AC27-85A:** every named carrier/facade/error has its approved non-root
  owner; private fields remain private.
- **AC27-85B:** the four Slice 80 cycles and new search/reader-pool cycles are
  prohibited and mutation-tested.
- **AC27-85C/D/E:** exact policy, whole-crate extraction, item-level SCCs,
  source-derived inventories, 16 configurations, cache behavior, negative
  grammar fixtures, macro fingerprinting, and production mutants pass.
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
