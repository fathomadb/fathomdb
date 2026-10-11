# Slice 135 expanded system robustness result — 2026-10-07 UTC

**Status:** focused, exact-source diagnostic. This adds a bounded SQLite-full
write and recovery case to the [earlier three-case result](../2026-10-07-robustness-first/README.md).
It is not the complete Phase 1 robustness matrix or a release gate.

## Identity and execution

- Exact candidate source commit: `7595b31eb8df6cb40746dbf4aed05991ac521757`.
  Product search code includes the fourth Slice 135 fail-closed repair at
  `55f8120f5fff44414f8ee57ef987d40b9f4306ff`.
- Fixture: `src/rust/crates/fathomdb-engine/tests/slice135_robustness_first.rs`,
  SHA-256 `7f0ee35314fe853f1460fdb254e59af7347af37f3e59f5d4e2480bfc8da8d1a7`.
  `Cargo.lock`: `9e9d7b5e82184a0bddfbe96de28fa0615ef4443fc84639b271c1cff29600ccfe`.
  Executed test binary: `target/debug/deps/slice135_robustness_first-c357a6263a4a7fa4`,
  SHA-256 `d6c628c3eb257a40b037fa8c8f36fcd924a9107e9b220826268348d0583f28cf`.
- On the candidate worktree, `cargo test --offline --locked -p fathomdb-engine
  --test slice135_robustness_first -- --nocapture --test-threads=1` exited 0:
  **four passed, two ignored**. The separately selected, intentionally wrong
  `slice135_negative_missing_record_must_fail` exited 101 and identified the
  absent record. See [positive.log](positive.log), [negative.log](negative.log)
  and [resource.txt](resource.txt). The test process was serial; `/usr/bin/time
  -v` reported 0.57 s wall time, 67,988 KiB peak RSS and no swaps. This is
  resource context for the fixture, not a latency comparison.
- All four cases used temporary real SQLite databases, FathomDB `Engine` calls,
  a fresh reopen oracle, and independent SQLite integrity checks. The test
  uses no database mock. The SQLite-full cap is local to its temporary file.

## Observed cases

| Case | Controlled boundary | Observed result |
| --- | --- | --- |
| Concurrent read/write | Two writers add 16 records while two readers repeatedly point-read a durable anchor. | 194 reads, 191 during an active writer; exact 17 records after reopen; file descriptors 5 → 5; integrity `ok`. |
| Process kill/restart | Kill child once before write, once after `Engine::write` returned and the child read its new record. | Before-write reopen lacked the record; after-acknowledged-write reopen contained it. Both victims terminated with signal 9; integrity `ok`. This does not simulate power loss. |
| Injected pretransaction refusal | One-shot `force_next_commit_failure_for_test` fires after validation and before `BEGIN`; write recovery afterward. | Typed `Storage`; rejected record absent before and after reopen; recovery record present; file descriptors 5 → 5; integrity `ok`. |
| Bounded SQLite full | Set `max_page_count` to `page_count + 1` (128 → 129), then attempt a 1 MiB governed write; lift cap and write recovery. | Typed `Storage`; failed record absent immediately and after reopen; base and recovery present; direct `canonical_nodes` count 2; integrity `ok`. |

The [summary.json](summary.json) was independently recomputed from the raw
`SLICE135_ROBUSTNESS` records. Its checks require exactly the four case IDs;
four successful and two ignored tests; identical expected and reopened arrays;
no partial failed write; `Storage` on both injected failures; positive overlap
and unchanged file-descriptor counts where sampled; two physical rows after
SQLite-full recovery; and `integrity_check=ok` in every case. The negative log
must show the deliberately missing row as `None` against an expected body.
The raw logs are authoritative if the summary and log ever disagree.

The retained files have SHA-256 digests: `positive.log`
`dcea70348f5ce8ed8f4aa97a16e42d4c1bb94784f4a4015864c27df416175eec`,
`negative.log` `2377b03e86fbfa1eccebf33a7acdd4a928ca00e4ab083b380583228af78ae3ef`,
`resource.txt` `4de81dc315b27ce12a472d1e4690d545cd03862fb30a2e3d2b8c1e35a81eeae8`,
and `summary.json` `470b610248fd0ad3b0aa6556e410961808e5d7b1aa5eeaa65be379a519824d7f`.

## Scope and next rows

No run failed unexpectedly. This receipt qualifies the four named boundaries
on its source identity only. It does not yet cover a crash *within* transaction
commit, sustained SQLite busy/full conditions, provider failure, projection
queue state, close/cancellation under load, interrupted erasure or randomized
schedules. The full robustness matrix, current-source paired latency and
Pareto coverage overlay remain open. A full `agent-verify.sh` run was not
performed for this focused result.
