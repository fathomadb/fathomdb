---
title: FathomDB 0.8.28 Slice 42 — status
status: IN_PROGRESS (codex R-CODE CONCERN fixed 2026-10-31; gate blockers fixed; not landed)
target_release: 0.8.28
observed_on: 2026-10-31
---

# Slice 42 status

Step log for [plan.md](plan.md). Commits are on
`slice/0.8.28-42-test-hygiene-and-published-cache` (base `59079d5c5`,
`dev/0.8.28`). Not pushed, not landed.

| Step | State (2026-10-31) |
| --- | --- |
| P1 phase 1 docs | done: `d6cfab127` |
| R-DES design review | APPROVE-WITH-FIXES on `d6cfab127` (no P1); ten findings applied in `c7b275daf`, see [review.md](review.md) (finding 9 reverted in `ae7061bfe`) |
| A Unit A | done: `fb5f60055` |
| B-RED/GREEN | done: three pairs, docs, and five fix commits (table below) |
| R-CODE codex review | CONCERN on `59079d5c5..16550b4ba` (one medium: the D27 runner's hard-coded rusqlite 0.40); fixed in `562477d70` / `1ea86b182`, see [code-review.md](code-review.md) |
| V verification | done except the platform legs carried to final verification (below) |
| L land | not started |

## Commits

| Sha | Kind | What |
| --- | --- | --- |
| `fb5f60055` | Unit A | `fd_count_under` and the two ruled lines (RED: the existing test, 30 of 30 parallel runs failing at the base) |
| `a64a1c6b0` | B-RED-1 | `bundled_sqlite_is_3_54_1_or_later` fails: "linked SQLite 3053002 (3.53.2) is older than 3.54.1" |
| `15d564788` | B-GREEN-1 | `rusqlite` 0.41.1 / `libsqlite3-sys` 0.39.1, both patch lines removed, API migration. **Bisect note:** from this sha until `d23b4e963`, the live pinned-override check fails ("missing SDK driver duplicate Cargo patch libsqlite3-sys", "missing approved SQLite patch libsqlite3-sys") |
| `3dfe21d61` | B-RED-2 | the two new pin-guard cases and the SDK-driver pytest refusal fail |
| `d23b4e963` | B-GREEN-2 | vendored crate deleted; checker, JSON, gitleaks, lint configs, design note, roadmap line |
| `d1078aa76` | B-RED-3 | predicate test fails to compile (six E0425) |
| `98f06d928` | B-GREEN-3 | version-only Windows guard; marker removed |
| `2af70d3bc` | docs | ADR exit, decision index, interfaces, install doc, CHANGELOG, `dev/deps`, release state |
| `d598949af` | tool | registry-consumer harness and AC42-11 results |
| `c903321c3` | fix | module-boundary classification for `connection_runtime::tests` (superseded by `ffae52b18`) |
| `ffae52b18` | fix | predicate test moved unchanged into `lib.rs` tests, because the Windows WAL attribution guard strips `mod tests {` to end of file; policy line removed again |
| `ae7061bfe` | fix | `scripts/slice135_pilot.py` restored (its SHA-256 is frozen by the S02-L campaign) |
| `246367c61` | fix | `test_sqlite_dependency_contract.sh` moved to 0.41. A follow-up correction, not an independent RED: the contract was missed in B-RED-1, and its failure evidence was produced against the base export, not at its own parent |
| `7d48766dd` | fix | Slice 90 root inventory records the new `lib.rs` blob |
| `16550b4ba` | docs | verification results and status |
| `09ea0d668` | RED | a durable Slice 90 archive root relocated behind a symlink must validate (fails with the live AC-073 "bundle directory must be canonical and absolute"); guard: a symlink inside the archive is still refused |
| `ceaa7e409` | GREEN | `durable_bundle_directory()` resolves the archive root once; bundles inside must still be canonical |
| `0e4ad3efd` | docs | board line 93 lists the remaining slice ids ("42, 50, 60, 70, 80, 90"), which the board-currency checker reads as a list |
| `562477d70` | RED | codex finding: the D27 runner's generated manifest must take rusqlite from the selected checkout (the 0.41 case fails) |
| `1ea86b182` | GREEN | `rusqlite_requirement()` reads the selected engine manifest; one audit-test fixture gained an engine manifest |

The RED shas were audited in throwaway exports (`git archive <sha>`):
`a64a1c6b0` fails with 3053002; `3dfe21d61` fails on "a root libsqlite3-sys
patch is unsupported" and the pytest; `d1078aa76` fails with six E0425.

## Acceptance criteria

| AC | Result |
| --- | --- |
| AC42-01 | 0 of 30 parallel runs fail at `fb5f60055`; every record shows `"fd_before":0,"fd_after":0` |
| AC42-02 | 30 of 30 parallel runs fail at `59079d5c5`, all at line 437 (left `Some(4)` in 27 runs, `Some(5)`, `Some(8)`; right `Some(11)` to `Some(23)`) |
| AC42-03 | leaked file: FAIL at the descriptor assertion, `left: Some(0)`, `right: Some(1)` |
| AC42-04 | leaked engine: FAIL at the descriptor assertion, `left: Some(0)`, `right: Some(27)` |
| AC42-05 | alone 5 of 5; `--test-threads=1` 3 of 3 |
| AC42-06 | `fb5f60055` touches only the test file: the helper and lines 366 and 429 |
| AC42-07 | RED at `a64a1c6b0`; passes from `15d564788` (Linux and Windows) |
| AC42-08 | `Cargo.lock`: `rusqlite` 0.41.1 (`da9bba6f…`), `libsqlite3-sys` 0.39.1 (`14a5f01b…`), both from crates.io; one `libsqlite3-sys` in `cargo tree` |
| AC42-09 | RED at `3dfe21d61`; at `d23b4e963` the shell test, the pytest and the live checker pass |
| AC42-10 | `third_party/` is gone; no manifest patches `libsqlite3-sys` |
| AC42-11 | pass: [results/registry-consumer](results/registry-consumer/README.md). Base: registry 0.38.2, 3.53.2, flag on, AC40-95 to AC40-97 fail. Final: registry 0.39.1, 3.54.1, flag off, all pass |
| AC42-12 | RED at `d1078aa76`; passes from `98f06d928`; the marker is gone from `src` and `scripts` |
| AC42-13 | pass on the Windows VM: [results/windows](results/windows/README.md). Three Windows lib tests fail or hang on both the base and the final tree (pre-existing; listed there) |
| AC42-14 | lint, typecheck and security pass; test 203 of 204 suites, the one failure being a pre-existing Python 3.13 incompatibility (see the full gate below) |
| AC42-15 | lint-md (apart from the environmental item), lint-docs and lychee pass; the AC42-15 search prints only the deliberate hits named in test-plan § 4.6 |
| AC42-16 | recorded: [results/agent-long-perf-baseline.md](results/agent-long-perf-baseline.md). AC-013 at n = 10,000 fails (p50 128 ms against 80 ms) on both base and final: the known regression for Slices 50 to 90 |

## Full gate (AC42-14)

Final run: `./scripts/agent-verify.sh` on the clean tree at `998882c1a`
(2026-10-31, 14:12 to 14:35, windchill3, load 2 to 6 on a shared host):

```bash
export LD_LIBRARY_PATH="$(.venv/bin/python -c 'import sysconfig;print(sysconfig.get_config_var("LIBDIR"))')"
AGENT_VERBOSE=1 ./scripts/agent-verify.sh
```

The `LD_LIBRARY_PATH` export is environmental. This worktree's `.venv` (from
`scripts/bootstrap.sh`) uses uv CPython 3.13, whose `libpython3.13.so.1.0` is
not on the loader path, and the `fathomdb-py --lib` test binary links it.

| Verb | Result |
| --- | --- |
| preflight | pass |
| lint | pass, including module boundaries, workspace clippy, rustfmt, the pinned-override check, `check-runtime-checkpoints` (fixed in `ceaa7e409`), lint-md and lychee |
| typecheck | pass (Rust, Python, TypeScript) |
| security (`STRICT=1 AC037_LIVE_OPTIONAL=1`) | pass: 0 violations, 0 blockers, 1 allowed downgrade (AC-037 live netns layer unavailable here). AC-036 ran under strace in the sandbox: "no successful listen() captured", so no unsandboxed rerun was needed |
| test (`--tier=all`) | 203 suites pass and 1 fails, `test-pypi-publish-roundtrip` |

That one failure is deterministic, not a flake. The suite's minimal PyPI
server imports `cgi`, which Python 3.13 removed. Inside the gate `python3` is
this `.venv`'s 3.13, so the server dies before printing its port ("minimal
PyPI index failed to bind"); `PATH=.venv/bin:$PATH bash
scripts/tests/test_pypi_publish_roundtrip.sh` reproduces the failure, and
`.venv/bin/python -c "import cgi"` raises `ModuleNotFoundError`. Run
standalone with system Python 3.12 at `998882c1a`, the suite passes ("All
PyPI REAL round-trip tests passed"). It is outside Slice 42 and fails on any
3.13 virtual environment.

The suites that failed on the first run all pass in this one:
`test-sqlite-dependency-contract`, `test-slice90-root-reconciliation`,
`test-check-board-currency`, `test-windows-wal-attribution-ci-job`,
`test-slice135-harness`, `test-runtime-checkpoints` and `test-rust`
(fathomdb-py included).

## Carried to 0.8.28 final verification

- macOS: no local host; the CI matrix covers it.
- Windows CI (`windows-wal-attribution` jobs): not run; the local VM covered
  AC42-13.

## Findings for the coordinator

- Pre-existing parallel-run flake (Linux): `slice50_evidence.rs`,
  `evidence_search_races_are_snapshot_atomic_or_wholly_refused` and
  `evidence_linearizes_at_sidecar_and_resolver_return_seams` fail with
  `EvidenceUnavailable` in 6 of 15 parallel runs at `59079d5c5`; serial runs
  pass.
- Pre-existing Windows failures: two slice90 startup-fault tests fail, and
  `current_opener_holds_lock_before_admission_classification` hangs, on both
  trees ([results/windows](results/windows/README.md)).
- `test-pypi-publish-roundtrip` imports `cgi` and fails under Python 3.13 (above).
- The worktree bootstrap picks uv CPython 3.13, so `fathomdb-py --lib` needs
  `LD_LIBRARY_PATH` to run, and the PyPI round-trip suite fails.
