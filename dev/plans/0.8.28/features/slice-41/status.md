---
title: FathomDB 0.8.28 Slice 41 — status
status: phase 1 done and reviewed; phase 2 (units F and Q) implemented, code review next
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 41 status

Branch `slice/0.8.28-41-sdk-evidence-helper`, cut from
`slice/0.8.28-40-fts-ranking-defects` at `aea8e0457`. Worktree
`../fathomdb-worktrees/slice-0.8.28-41`.

| Step | Who | What | State (2026-10-16) |
| --- | --- | --- | --- |
| W1 | implementer | GOAL, requirements, design, test plan, this file | done; see the commit log |
| R0 | reviewer subagent | Review of W1, including Q1–Q6 | APPROVE-WITH-FIXES on `8e4f9a543`; F1–F15 applied with the orchestrator's rulings (test-plan § 6) |
| Owner | owner | Rulings 2026-10-16: allowlist addition approved with the R0 adjustments; tagged body opt-in with hazards documented and a decision-index row (can be revisited); LanceDB on the Slice 40 renderer in M3, labelled | done (test-plan § 6) |
| W2 | implementer | RED tests (test-plan § 2, § 3) | done at `86e15e333` (+ `338d1da9f` Cargo.lock); RED record in test-plan § 7.1 |
| W4 | implementer | Core module, bindings, parity map, docs; no test edits | done: core `3e0903d91`, surface `2ccc6afc6`, pin `334497e4c`, docs in the following commit; GREEN record in test-plan § 7.2 |
| W4 surface | implementer | Write the allowlist entry `_slice41_context_pack_signature` (design § 5) and the decision-index row, both citing the 2026-10-16 owner rulings | done: allowlist and parity map `2ccc6afc6`, re-issued pin `334497e4c`, decision-index row 64 (row 65 after the rebase onto Slice 40, whose page-cache row took 64) |
| W5 | implementer | Scoped checks (the full gate waits for the code review) | rulings 1–4 applied; Python suite 1681 passed, 17 skipped |
| M | orchestrator | AC41-27 differential and M1 preview now; after the Unit H head: M1, § 4.2, M3, § 4.5 | AC41-27 180/180 and the M1 preview done (test-plan § 7.3, § 7.4); the rest waits on Slice 40 Unit H |
| R2 | reviewer | Code review | APPROVE-WITH-FIXES, P3 only, on `24e3215be`. RED tests `e8c581a44` (new files only, no existing test edited); fixes in the following commit: TypeScript refuses non-safe-integer counts host-side; Python refuses a bare-`str` `keep_roles` with `TypeError` (the documented "wrong argument type" class) and forwards non-finite counts so the core names the field; the core names the bound for counts above `u64::MAX`; docstrings on the remaining `pub` items |

## Phase 2 (owner ruling 2026-10-16)

| Step | Who | What | State |
| --- | --- | --- | --- |
| P2-W1 | implementer | Requirements § 5, design § 8–9, test-plan § 8 | done; see the commit log |
| P2-R0 | reviewer | Plan review | APPROVE-WITH-FIXES on `80e4ea954`; F-Q1 to F-Q8 and F-F1 to F-F3 applied to requirements § 5, design § 8–9 and test-plan § 8. Rulings: Q-T1 YES (phase 1 expected entries gain `"forced": false` in the RED commit; the deletion check is recorded here); Q-B1 open-time `EngineConfig` setting, not persisted; per-query override and doctor suggester out of scope |
| P2-W2 | implementer | RED: new tests and fixture cases | done: ruling 5 edits `50f0638ed`, tests and skeleton `51428d28b`; record in test-plan § 8.5 |
| P2-W4 | implementer | GREEN: unit F, then unit Q; allowlist text and pin; docs | done: code `b67cc6c6a`, docs and allowlist text `b48550414`, pin `d99ad7f4d`, Python declarations and EARP catalog `8fb1bfd62`, then the module-boundary row; one Python guard test waits on ruling 8 |
| P2-R2 | reviewer | Code review | not started |
| P2-M | orchestrator | MF1 now; MQ1 and MQ2 at the merged head after Unit H | not started. Precondition for AC41-39 at the merged head (Slice 40's H1 RED tests removed) satisfied: reverted at `fb42f612e` |

## Rules

- Every expected value in the in-repo fixtures is hand-derived from
  REQ41-P2 to P9 and checked by hand. The AC41-12 fixture's expected text
  is written by hand from the rules and checked against a reading of
  `render_evidence`, not captured from either implementation. W2 recorded
  the derivation in the fixture's `_derivation` field. The fixture scales the
  caps to `item_chars` 40 and `budget_chars` 95, so a round over the cap
  stays small enough to check by hand.
- This slice does not edit `dev/plans/release-state-0.8.28.json`,
  `dev/plans/runs/STATUS-0.8.28.md` or any generated block.

## Rulings (orchestrator, 2026-10-16, under the owner's approval of the surface)

1. `tagged_and_plain_corpora_fuse_identically`: the drain timeout went from
   30 s to 600 s. CPU embedding of two 200-row stores takes about 100 s.
   This is a timing parameter; no assertion changed.
2. The surface-count pins follow mechanically from the approved surface, and
   each edit changes only pinned counts and lists:
   - `src/ts/tests/sdk-surface-parity.test.ts`: 44/44 → 48/48;
   - `scripts/tests/test_check_sdk_surface_parity.py`: 69 → 77, 44 → 48;
   - `scripts/tests/test_generated_public_docs.py`: 44 → 48;
   - `scripts/tests/test_check_governed_surface_pin.sh`: the expected
     member list, the counts, and a Slice 41 provenance check.
3. Python baseline: the additions are declared in
   `src/python/tests/python_surface_additions.json`: 39 declarations under
   slice 41, plus the new `native_stub_sha256`. With them,
   `test_python_declarations_match_pre_move_baseline_with_approved_deltas`
   passes.
4. `test_package_boundary_matches_pre_move_baseline`: the new names stay
   top-level, as `rerank` is. The expected export set is now the frozen
   baseline exports plus the registry's top-level additions, meaning paths
   that are exactly `fathomdb.<name>`. This is the mechanism the sibling
   declaration test already uses. The test still asserts the baseline holds
   119 exports, and `__all__` must have 119 plus the number of registry
   additions. This is the first top-level addition since Slice 130, so the
   mechanism is new; the change is limited to this one test.

5. Phase 2 (orchestrator, 2026-10-16, option A). The phase 1 comparators
   in TypeScript and Rust list their fields by hand, and every default-policy
   check lists the policy fields, so Q-T1 alone could not go GREEN. Allowed,
   each making a comparator or default check cover the new field:
   - `src/ts/tests/context-pack.test.ts`: `forced` in `asWire`, `full_ids`
     in the default-policy object;
   - `fathomdb-engine/tests/slice41_context_pack.rs`: `forced` in
     `entry_to_json`; `full_ids: Vec::new()` in the proptest literal
     (compile-required);
   - `fathomdb-sdk/tests/slice41_context_pack.rs`: `forced` in `to_json`, a
     `full_ids` arm in `policy_from`;
   - `src/python/tests/test_context_pack_parity.py`: `full_ids` list
     normalisation in the default-policy check;
   - fixture data: `defaults` gains `"full_ids": []` and the 45 phase 1
     expected entries gain `"forced": false`. Check: deleting every
     `"forced": false` key (with its trailing comma and space) and the
     `"full_ids": [],` line reproduces the
     fixture at `9d66443c0` byte for byte (verified in the RED commit:
     45 keys, byte-identical);
   - phase 2 cases live under new keys `full_ids_cases` and
     `full_ids_errors`, which only the new tests load. The "omit `forced`
     when false" wire alternative was rejected.
6. `fathomdb-py/src/test_support.rs:87` gains `query_stop_words: None`, a
   compile-required mechanical change; nothing else there changes
   (orchestrator, P2-R0b). The same one-line change was needed in
   `fathomdb-engine/tests/slice90_engine_configuration.rs:21`, the only other
   full `EngineConfig` literal in a test (found by `cargo check --workspace
   --all-targets`); applied under the same reasoning and flagged for the
   orchestrator. `fathomdb-py/src/engine.rs` (`engine_config_from_py`) is a
   fourth full literal in binding code.
7. W2 found AC41-32 clause (i) false for a second forced item: the first
   forced item spends budget, so a later one can render fewer chars than
   its compact phase 1 form. Clause (i) is restated for the first forced
   item only (requirements § 5.5), which is what the property tests.

8. Orchestrator ruling 6 (2026-10-16):
   `src/python/tests/earp/test_config_resolver.py::test_engine_config_has_exactly_the_known_fields`
   pins the `EngineConfig` field set. Adding `"query_stop_words"` to it
   follows mechanically from the owner-ruled surface, the same kind of change
   as the 44 to 48 operation-count pins; nothing else in the test changes
   (`03f952b2f`; the file passes 51 of 51). The catalog classifies the knob
   as semantic, witnessed by `open_report.stop_words_sha256`.
9. `fathomdb-engine/src/graph_expand/execution.rs` keeps the old
   `read_graph_expand_in_tx` signature as a `#[cfg(test)]` wrapper over the
   built-in list, so the in-crate test in `src/evidence.rs` that calls it
   is unchanged; production dispatch uses
   `read_graph_expand_with_stop_words_in_tx`.
10. Orchestrator ruling 7 (2026-10-16): the `query_stop_words: None` line
    in `fathomdb-engine/tests/slice90_engine_configuration.rs:21` (item 6)
    is approved after the fact as a compile-required one-line change.
11. The `fathomdb` facade re-exports `QueryStopWords`, `StopWordBase`,
    `StopWordRejection` and `DEFAULT_STOP_WORDS_SHA256`. These are covered by
    the owner's 2026-10-16 rulings that added the configurable stop-word
    list to Slice 41, together with the owner's plan approval (orchestrator,
    2026-10-16).
12. `pr_g8_dangling_edges` (2 of 20 failing runs at `80e4ea954`, 10 of 20
    on this branch; test-plan § 8.5): the code reviewer found no mechanism
    in this slice's diff. It is a test-ordering race that predates the
    slice; the trigger is the test's raw `Connection::open`, which can
    initialize SQLite before the first engine open. Follow-up outside this
    slice; nothing changes here.
13. Code review fixes (APPROVE-WITH-FIXES, P3 only): P3-5 moves the
    `full_ids` checks after the REQ41-P8 `expand_groups` position check, so
    every P8 error comes first (REQ41-F3, design § 8.1). The new fixture case
    `p8_position_error_wins_over_full_ids` pins the order. P3-6 sets
    `OpenReport.stop_words_sha256` when the report is built. P3-7 documents
    the stop-word API in the `fathomdb-query` README.

Pre-existing failures under `test-hooks` (the feature is turned on by
unification in multi-crate runs), left as they are: `slice135_robustness_first`
`slice135_persistent_sqlite_full_preserves_state_until_cap_lifts` (file
descriptors 7 → 12), and `tc57_governed_write_race` ×2
(`RuntimeConfiguration(TooLate)`). The Slice 40 worktree's binaries built on
2026-10-16 fail the same way.
