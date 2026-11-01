---
title: FathomDB 0.8.28 Slice 50 — test plan
status: design investigation; built-in-provider replacement proposed; no implementation authorized
target_release: 0.8.28
observed_on: 2026-11-01
---

# Slice 50 test plan

> **Current direction (2026-11-01):** owner scope `seq-355` limits production
> providers to the built-in implementation before 1.x. Read the revised
> [user needs, requirements and AC](built-in-provider-requirements.md),
> [design and extension inventory](built-in-provider-design.md), and
> [design work plan](built-in-provider-plan.md) before the original phase-1
> material below. Unit S is unchanged. Unit D / D50-1 / D50-2 are historical,
> unselected options, held without implementation; their recommendations are
> not the current proposal. The replacement contract and release placement
> require owner review before production changes.

Each run records the host, `nproc`, the 1-minute load, the commit and the
command. Results go in [status.md](status.md). Scaling logs and results go
under `results/` in this directory.

## 1. Run rules

- **No BGE in this slice's own tests.** The new tests and the scaling
  probe use deterministic synthetic embedders on the CPU (a fixed or
  FNV-seeded vector, as `perf_gates.rs` does), so they need no GPU.
- **The full gate is different.** Its default-embedder targets encode
  with BGE. That run follows the owner rule of 2026-10-31:
  - build with `embed-cuda` and set `FATHOMDB_EMBED_DEVICE=cuda:0`;
  - exclude the K620 with `FATHOMDB_GPU_EXCLUDE`;
  - check `nvidia-smi` for room before the run, and confirm the RTX 3090
    engaged during it;
  - never run it on the CPU or on the K620.
- **ptrace.** `strace` (AC-036 in the full gate) is authorized. If the
  sandbox denies `PTRACE_TRACEME`, rerun the unchanged gate unconfined.
- **Own virtual environment.** The full gate runs from this worktree's own
  `.venv` (`./scripts/bootstrap.sh`). Slice 42 lesson: on uv CPython
  3.13, export `LD_LIBRARY_PATH` to the interpreter's `LIBDIR` for the
  `fathomdb-py --lib` binary.
- **Scratch outside the repository.** Base-sha exports for the scaling run
  and the RED audits live in the session scratch directory. Uncommitted
  mutation checks (AC50-04) are reverted before any commit.

## 2. RED tests, one per behaviour

All tests run on a real SQLite database. Nothing is mocked.

| Commit | Test (file) | AC | Fails at RED because | Runtime |
| --- | --- | --- | --- | --- |
| H-RED | `slice50_scan_hook_reports_production_pages` (`tests/slice50_drain_scan_bound.rs`) | infrastructure | compile error: `projection_scan_page_for_test` does not exist (E0599) | — |
| S-RED | `slice50_scan_plans_bound_each_arm` (same file) | AC50-01 | neither scan has a `CO-ROUTINE` under its compound members | < 1 s |
| S-RED | `slice50_scan_page_cost_is_independent_of_backlog` (same file) | AC50-02 | VM-step ratio about 7–8 against a limit of 1.5 | 10–30 s (debug) |
| D-RED | `slice50_refused_runtime_drain_does_not_wait_for_timeout` (`tests/slice50_refused_drain.rs`) | AC50-09 | `drain(5_000)` returns after about 5 s | about 5 s at RED |
| D2-RED (if D50-2) | `slice50_refused_runtime_transition_proceeds`, `…_configure_projections_proceeds`, `…_lifecycle_actuation_proceeds`, `…_erasure_proceeds` (same file) | AC50-11 | each waits 30 s, then returns `Err(Scheduler)` | 30 s each at RED |

The guards below are expected to pass at their RED sha. They land in the
S-RED commit beside the failing tests. The S-RED body names which
assertions fail and which pass, and why.

- **AC50-03, the property test.** It compares the production scans with
  the shipped SQL kept verbatim in the test. At S-RED both are the same
  text, so a pass there shows that the reference reproduces production.
  After S-GREEN the same pass shows that the bounded scan equals the
  shipped one.
- **AC50-04.** Uncommitted mutations show that AC50-03 can fail.
- **AC50-05, the ordering cases.**

These are refactor oracles for REQ50-S2. They are not coverage of a new
behaviour, so they are not labelled RED.

## 3. Unit tests in detail

### 3.1 Hook (H-RED, H-GREEN)

`slice50_scan_hook_reports_production_pages`:

- Store: an engine with `FixedEmbedder`, `configure_vector_kind_for_test("doc")`,
  the dispatcher frozen, then 300 nodes and 30 body edges written.
- `projection_scan_page_for_test` for both scans from cursor 0 returns
  256 rows in increasing cursor order, `vm_steps > 0`, and a non-empty
  plan.
- `sql` names the production text: it contains
  `ORDER BY pending.write_cursor`, and for the scheduler
  `CROSS JOIN _fathomdb_projection_generation_current`.

### 3.2 Plan (AC50-01)

`slice50_scan_plans_bound_each_arm` parses each scan's
`id|parent|detail` lines into a tree.

1. Each `LEFT-MOST SUBQUERY` and `UNION ALL` node has one child whose
   detail starts with `CO-ROUTINE`. Today it has none.
2. Each `USE TEMP B-TREE` line has parent 0.
3. The node arm has `SEARCH … USING INDEX canonical_nodes_write_cursor_idx (write_cursor>?)`
   and the edge arm `SEARCH ce USING INDEX canonical_edges_write_cursor_idx (write_cursor>?)`.
   There is no `SCAN canonical_nodes`, `SCAN n` or `SCAN ce`.

Items 2 and 3 hold today. They guard the GREEN shape, because an arm that
sorts on its own, or scans instead, would read the whole arm before its
`LIMIT`. Expected at S-RED: item 1 fails for both scans.

### 3.3 Cost (AC50-02)

`slice50_scan_page_cost_is_independent_of_backlog`:

- **Stores.** Two engines, dispatcher frozen, kind `doc`. Small: 2,000
  nodes and a body edge every fourth row (500 edges). Large: 16,000
  nodes and 4,000 edges. Both arms hold more than a page at both sizes.
- **Measure.** For each scan, `vm_steps` of one page from cursor 0 at
  one fixed `effective_at`.
- **Assert.** `vm_steps(large) / vm_steps(small) ≤ 1.5`.
- **Expected at S-RED.** About 7–8 for both scans. The scratch probe
  measured 7.5 at 1,000 against 8,000 nodes (design § 1.3).
- **Expected after.** About 1.0.
- VM steps count SQLite bytecode operations, so they do not depend on
  timing or load.

### 3.4 Equivalence (AC50-03, AC50-04, AC50-05)

**`slice50_bounded_scans_match_shipped_scan`** (proptest; 64 cases, and
`PROPTEST_CASES` overrides):

- **Fixture.** One engine-created store, dispatcher frozen. Each case
  clears and refills `canonical_nodes`, `canonical_edges` and
  `_fathomdb_projection_terminal` on a second raw `rusqlite`
  connection. The writes are in one transaction and do not touch vec0.
- **Generator.** Unique cursors `1..=k`, k ≤ 700. Each cursor is one of:
  - a node of a vector kind;
  - a node of a non-vector kind;
  - a superseded node;
  - a body edge;
  - a bodiless edge;
  - a superseded edge;
  - an edge with `t_invalid` in the past or the future.

  A random subset gets a terminal (`up_to_date` or `failed`). Both
  arm-dominant layouts are weighted in: runs of more than 256 of one arm.
- **Check.** For every `after` in `0..=k` and `through` in
  `{None, Some(after + d)}` with random `d`, the hook's rows equal the
  reference SQL's rows on the raw connection, at the same
  `effective_at`.
- **Reference.** The shipped text, with
  `pending_edge_projection_from_where` expanded verbatim, kept in the
  test file.

**AC50-04, mutation checks** (uncommitted, after S-GREEN): set an arm's
`LIMIT` to 1, then order an arm `DESC`. Each must make the property test
fail. Record the failing case, then revert.

**`slice50_node_edge_ordering_cases`**, through the public write path
(`FixedEmbedder` that fails for bodies containing `fail`, retry delays
`[]`):

- (a) 600 body edges, then 600 `doc` nodes;
- (b) 600 nodes, then 600 edges;
- (c) interleaved, with failed terminals (the failing bodies), one
  superseded node, one superseded edge, and one edge with a past
  `t_invalid`.

For each, scheduler and candidate pages from every 50th cursor equal the
reference. Then unfreeze and `drain(30_000)`:

- `_fathomdb_vector_rows` cursors equal the expected eligible set
  (computed in the test from what was written);
- the failed bodies have `failed` terminals;
- `read_embedding_readiness()` is `Ready` with `pending_count == 0`.

### 3.5 Suites that must pass unchanged (AC50-06)

`cargo test -p fathomdb-engine --features test-hooks` with these targets:

- `slice20c_flush_barrier`, `slice20_dense_readiness`,
  `slice30_embedding_readiness`, `slice21_projection_runtime_state`,
  `slice22_projection_status`, `slice90_projection_runtime_status_surface`;
- `erasure_drain_ordering`, `projection_runtime`, `eu5f_production_pin`,
  `slice15_byo_llm_ingest`, `slice40_projection_generation`,
  `slice40_projection_generation_races`;
- `tc91_projection_commit_hardening`,
  `tc91_duplicate_embeds_and_silent_commit_loss`,
  `vector_equivalence_probe`, `tc68_probe_fingerprint_cache`;
- the `--lib` tests.

Also:

- `rebuild_projections` with `--features test-hooks,operator`;
- `slice40_projection_completion` in a debug build (it is
  `#![cfg(debug_assertions)]`; it covers the below-watermark rewind).

The Python and TypeScript drain and readiness tests run in the full gate.

### 3.6 Unit D (AC50-09, AC50-10, AC50-11)

Fixture: the refused-runtime setup of
`slice30_embedding_readiness.rs::equivalence_refused_embedder_defers_work_without_relabelling_it_as_missing_configuration`:

1. a faithful open that registers `doc`;
2. a second faithful open that records the accepted baseline;
3. a stale verdict;
4. a session without a runtime that writes a body edge and a `note` node
   with `logical_id`;
5. a divergent open, `dense_disabled == true`.

The fixture is copied into the new test file. The Slice 30 file is not
edited.

- **AC50-09.** `drain(5_000)` returns within 1 s with the D50-1 result
  (A: `Err(EngineError::Scheduler)`). At RED it takes about 5.0 s (the
  scratch probe measured 3.000 s for `drain(3_000)`). The test asserts
  `elapsed < 1 s` first, so the RED reason is the wait.
- **AC50-10.** The same fixture with no pending rows: `drain(5_000)` is
  `Ok` within 1 s. It passes at RED (guard). The existing Slice 30
  tests, including `drain(0).expect_err` at line 207, pass unchanged
  under option A.
- **AC50-11** (only if D50-2 approves). On the fixture with pending rows,
  each of these returns `Ok` within 1 s:
  - `transition("n1", Deleted, None)`;
  - `configure_projections` with an unchanged spec;
  - lifecycle actuation of `n1`;
  - `erase_source` of the edge's source (`purge` shares the same
    freeze, `erasure.rs` 667 and 994).

  At RED each returns `Err(Scheduler)` after 30 s (`transition`: measured
  30.000 s). One RED/GREEN pair per caller family: the three
  `drain_for_non_embedding_mutation` callers share one change, and
  erasure has its own. The erasure test also checks that the pending
  edge's rows are gone afterwards.

## 4. Scaling measurement (AC50-07, AC50-08, AC50-14)

### 4.1 Tools (committed in the tool commit)

- **`src/rust/crates/fathomdb-engine/tests/slice50_drain_scaling.rs`.**
  `#![cfg(feature = "test-hooks")]`, one `#[ignore]` test.
  - It uses only APIs that exist at the base `544bf74f8`:
    `open_with_embedder_for_test`, `configure_vector_kind_for_test`,
    `set_projection_scheduler_frozen_for_test`, `write`, `drain` and
    `close`, plus raw `rusqlite`. The same file therefore compiles and
    runs at the base and at the head.
  - It seeds the AC-013 corpus: the `perf_gates.rs` generator, seed
    `0x0AC0_13D0_13D0`, batch 1,024, kind `doc`, and the
    `VaryingEmbedder` algorithm.
  - **Env knobs:**
    - `SLICE50_N` (default 10,000);
    - `SLICE50_DIM` (default 768);
    - `SLICE50_MODE`, `held` (default) or `overlap`;
    - `SLICE50_DRAIN_TIMEOUT_MS` (default 3,600,000);
    - `SLICE50_DIGEST` (default 1).
  - **Output:** one `SLICE50_DRAIN {json}` line with `n`, `dim`, `mode`,
    `write_ms`, `drain_ms`, `us_per_vector`, `sqlite_version`,
    `vector_rows`, `projection_cursor` and `digest`.
  - **Digest:** SHA-256 over the ordered rows of `_fathomdb_vector_rows`,
    `vector_default` without `created_at`,
    `_fathomdb_projection_terminal` and the projection cursor.
- **`dev/plans/0.8.28/features/slice-50/tools/drain_scaling.py`.**
  - `build --sha <sha> --label <label>`: `git archive` into scratch,
    copy the head's probe file in, then build in release
    (`cargo test --release -p fathomdb-engine --features test-hooks --test slice50_drain_scaling --no-run`).
    It records the binary path and the sha.
  - `run --n 10000,30000,100000 --repeats 2`: interleaves the labels and
    reverses their order on odd repeats. Each run:
    - holds a `flock` and waits for 1-minute load ≤ 2;
    - records the load before, at its peak and after;
    - flags a run whose load exceeds 2.5.
  - `summarize` writes `results/results.json`. `analyze` applies § 4.3.
- **`dev/plans/0.8.28/features/slice-50/tools/test_drain_scaling.py`.**
  Unit tests of the parsing, median and rule functions. They test
  measurement correctness only, with no tamper or drift guards (owner
  rule, 2026-10-15).
- **`dev/plans/0.8.28/features/slice-50/tools/README.md`.** Purpose,
  commands, knobs and outputs for each tool.

### 4.2 Cells

- **Format.** Format 34 (1,024-row vec0 chunks), the default runtime
  cache, two workers, 768-d.
- **Timed runs.** Labels `base` (`544bf74f8`) and `head` (the slice
  head) at n = 10k, 30k and 100k, held mode, two repeats, untraced.
- **Reported only.** One `overlap` run per label at 100k, the gate's
  shape.

### 4.3 Decision rule (stated before any run)

Let `t(N)` be the median `drain_ms / N` over the repeats.

1. **Linear (AC50-08, REQ50-S5).** Head `t(100k) / t(10k) ≤ 1.3`.
2. **Improvement (reported).** Head's 100k held drain is ≤ 0.8 × base's
   in each repeat pair.
3. **No regression at 10k (AC50-08).** Head's 10k median is ≤ 1.05 ×
   base's.
4. **Identical stores (AC50-07).** Every head digest equals the base
   digest at the same N, and the repeats match each other.
5. **Base character (reported).** Base `t(100k) / t(10k)`. If it is
   below 1.3, the scan cost is hidden behind the format-34 commit cost
   on this host. Rule 2 may then not show the fix in wall time, and
   AC50-02 stays the deciding check. The prototype's format-34
   default-cache cell gave 1.43.
6. **Load.** A flagged run that decides rule 1, 2 or 3 is repeated once.

If rule 1 or 3 fails, the rule is not changed after the fact. The slice
reports the numbers to the owner and does not claim REQ50-S5.

### 4.4 Gating and runtime

The probe is `#[ignore]`. `agent-verify` does not run it, and it does not
need `AGENT_LONG`. Estimated cost on windchill3:

- two release builds, about 5–8 min each;
- base held drains of about 3, 10 and 47 s and head drains of about 3,
  9 and 30 s (prototype `BASE` figures), plus write phases;
- about 5 min of timed runs over two repeats, plus load-gate waits.

About 30 min in all.

## 5. Final-tree verification (AC50-12, AC50-13)

- `cargo clippy --workspace --all-targets -- -D warnings -A missing-docs`
  and `cargo check --workspace --all-targets` (toolchain 1.95.0).
- `./scripts/agent-verify.sh` once, on the final clean tree, from this
  worktree's `.venv`, with the § 1 GPU rule. Rerun only failed suites.
- `./scripts/agent-lint-md.sh` and `./scripts/check-release-state-views.sh`.
- The `AGENT_LONG` perf gates (`perf_gates.rs`) run once on the final
  tree for the record: AC-013 seed and drain times and AC-018. No
  latency claim is made. AC-013 at n = 10k is known to fail at the base
  (p50 128 ms), and Slices 60 to 90 address it.
