---
title: FathomDB 0.8.28 Slice 42 — reviews
status: R-DES APPROVE-WITH-FIXES (2026-10-31), findings applied
target_release: 0.8.28
observed_on: 2026-10-31
---

# Slice 42 reviews

## R-DES — design review of `d6cfab127` (Fable, medium), 2026-10-31

**Verdict: APPROVE-WITH-FIXES. No P1.** All findings are applied in the
review-fix commit that adds this file, except where the disposition says
the change lands in a later commit of the slice.

| # | Sev | Finding | Disposition |
| --- | --- | --- | --- |
| 1 | P2 | The AC42-15 grep hits live files the design does not list: `dev/plans/release-state-0.8.28.json` line 16 and `dev/plans/plan-0.8.28.md` line 184 cite `third_party/libsqlite3-sys-0.38.1/FATHOMDB-PATCH.md`. Add both to design § 2.7, reworded to "the exit conditions in the retired patch note; git history at `59079d5c5`". `plan-0.8.28.md` line 130 "AC42-01 to AC42-12" should read 16. Regenerate the release-state views after any JSON edit | Applied: both added to § 2.7, reworded in the slice's docs commit (with `check-release-state-views.sh --write`); line 130 fixed now |
| 2 | P2 | Decision-index row 64 (`dev/adr/ADR-0.6.0-decision-index.md` line 168) says "published Rust crates documented (`LIBSQLITE3_FLAGS`), not mitigated". Add its update (status "accepted; exit completed 0.8.28 Slice 42") to § 2.7 and REQ42-B7 | Applied to § 2.7 and REQ42-B7; edited in the docs commit |
| 3 | P2 | `dev/design/pinned-override-rot-guard.md` R4 (lines 59–75): rewrite the paragraph to the two admitted mechanisms (the `candle-kernels` Git patch and the fork registry pins), mirroring the new `cargo` description in `pinned-override-rot.json`. Its `cudarc` clause is already stale since Slice 31 | Applied to § 2.3; rewritten in B-GREEN-2 |
| 4 | P3 | `connection_runtime.rs` lines 115–116, the comment about "the bundled VFS correction": reword to SQLite ≥ 3.53.3's local-verbatim handling, or delete | Applied to § 2.4; reworded in B-GREEN-3 |
| 5 | P3 | B-RED-2 adds only the two new cases (a root `libsqlite3-sys` path patch fails as "unsupported Cargo override"; a tree without the SQLite patch passes). B-GREEN-2 drops `seed_sqlite_fixture` (test script lines 198–224), the lock stub and the drift cases. The RED evidence names exactly the two failing cases | Applied to § 2.6 and test-plan § 3.2. The SDK-driver pytest flip (refusal instead of admission) also goes in B-RED-2: it is the same behaviour, and flipping an assertion in a GREEN commit would edit a test to pass. The obsolete driver SQLite patch line and its `missing-sqlite` case go with the other SQLite fixtures in B-GREEN-2 |
| 6 | P3 | Keep the design's order. B-GREEN-1's commit body quotes the live checker's verbatim failure line (`scripts/agent-lint.sh` line 101 and `ci.yml` line 1387 both run it), and status.md records that sha for bisectors | Applied to § 2.6 and test-plan § 3.1 |
| 7 | P3 | Record `winIsRemoteVolume` in § 2.1 (3.54.x `sqlite3.c` line 52543, OR'd at line 52581; absent in the vendored 3.53.2): verbatim paths on a mapped network drive now take the shared handle. AC42-13 covers local disk only | Applied to § 2.1 and AC42-13 |
| 8 | P3 | B-RED-3's evidence quotes the E0425 lines; make the § 2.6 table match test-plan § 3.3 | Applied |
| 9 | P3 | `scripts/slice135_pilot.py` line 34: drop `"third_party"` from `SOURCE_PATHS` in the docs commit | Applied to § 2.3; edited in the docs commit (`REQUIRED_SOURCE_PATHS` is `SOURCE_PATHS[:3]` and does not include it) |
| 10 | P3 | `dev/deps/rusqlite.md` "Version pin: `0.32.1`": update the pin line to 0.41.1 as part of REQ42-B7 | Applied to § 2.7 and REQ42-B7 |
