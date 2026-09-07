# Slice 60 RED oracle correction

Date: 2026-09-07

An independent read-only audit confirmed two test-mechanics defects after the
Slice 60 GREEN implementation exposed them. Neither correction changes public
intent or the READY v5 design.

## Corrections

1. `result_limit_selects_top_n_only_after_the_walk_is_complete` now sets
   `target_kinds` to `fact`. The READY design's target-kind rule is return-only:
   a non-matching node remains traversable but is excluded from returned
   targets. This keeps `middle` traversable to `a-hop2` while making the test's
   intended comparison exactly `z-hop1` versus `a-hop2`, proving that hop count
   outranks lexical target ID and that traversal continues after the first
   returnable result.
2. The closed degradation-enum case now replaces `/degradationCodes` with
   `["unknown"]`. The shared response fixture correctly carries an empty
   top-level degradation array, so mutating `/degradationCodes/0` directly
   could not create the case and panicked before the decoder ran. The expected
   decoder diagnostic remains `graph_corrupt` at `/degradationCodes/0`, as
   required by READY v5's additive-response/closed-variant rules.

Relevant READY v5 sections are **Canonical wire schema and evolution**
(response readers accept additive fields but reject unknown enum variants and
report exact paths), **Direction and edge/target-kind filters** (`targetKinds`
is return-only), and **Deterministic ordering and result limiting** (global
top-N is selected only after traversal completes).

## Frozen-file hashes

| Path | Before SHA-256 | After SHA-256 |
| --- | --- | --- |
| `src/rust/crates/fathomdb-engine/tests/slice60_graph_expand.rs` | `04ffd3dc30678e51d424ad6cc420b4b90a384bf167a806af4991439ca6a3a87e` | `d2480f043b7ebdd55e8197d2bb60809be65b277ae6ac42c240eeeff0a1a108fb` |
| `src/rust/crates/fathomdb-engine/tests/slice60_wire.rs` | `b52abd744c4291499100fd9521ce8307e05a4c1629344442d0be6c084fda0cd8` | `0f09a9e3ecc625b32cf2e527cc5421405c9a8459268c31eb8faa264af78aa656` |

This is the one-time audited exception to RED-test immutability. The shared
fixture and the other four frozen Slice 60 test files were not changed.
