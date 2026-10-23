---
title: Agent access, Block 1 — close plan
date: 2026-10-21
status: ACTIVE
desc: Consolidates the Block 1 branches, proves the integration branch stable, files the open items as 0.8.29 todos on main, and builds three close fixes (a bounded per-term snippet pass, its measurement, non-exhaustive SDK structs) before Block 1 is declared closed.
inputs:
  - dev/plans/agent-access-block1-plan.md
  - dev/design/agent-access/README.md
  - dev/design/agent-access/match-centred-snippets.md
  - dev/design/mcp-adapter/engine-proposals/EP-17-match-centred-snippets.md
  - dev/design/mcp-adapter/conformance-report.md
---

# Agent access, Block 1 — close plan

## Purpose

Close Block 1 of the agent-access program on `feat/agent-access-block1`:

- the branch holds all Block 1 work;
- it is proven stable on one sha;
- every open item is a 0.8.29 todo on `main`;
- three close fixes land, test-first.

This plan does not merge Block 1 to `main`, and it does not start Block 2.

This revision applies the Fable adversarial review of 2026-10-21
(`close-plan-review.md`, PASS-WITH-FIXES). The finding each change answers is
given in brackets.

## Ground rules

- **Single writer.**
  - On the integration branch and its worktree,
    `fathomdb-worktrees/agent-access-block1`, the main thread of session
    `rmcp1` is the only writer. Session `block-1` stood down on 2026-10-21
    and confirmed its tree was clean at `b19551556`.
  - Phase B writes `main` from its own fresh worktree, `aa-b1-todos`, cut
    from `origin/main`. It never touches the shared main checkout.
- **Product code.** The main thread writes none. Each close fix goes to one
  `implementer` subagent, in a fresh worktree cut from the integration tip.
  - The owner directed on 2026-10-21 that this session run the
    implementation. That supersedes the 0.8.0 "the user spawns slice agents"
    rule (memory `orchestration-execution-traps.md` item 3) for this close.
    [P2-11]
- **Preflight.** `scripts/preflight.sh --worktree` hard-fails today because
  no release is live (the `release-state discovery failed` branch of `scripts/preflight.sh`), so before each spawn run its
  checks by hand [P1-2]:
  - `git merge-base --is-ancestor` of the worktree base against the
    integration tip;
  - `df` for disk;
  - the worktree's own `.venv`.
- **TDD.** Every behaviour change gets its own RED commit, with recorded
  failure evidence that it fails on its own. Tests are read-only during
  fix-to-spec.
- **Review.** A codex § 9 review gates each fix. When codex is unavailable,
  use the Fable-medium adversarial fallback. Each review fix gets its own RED
  commit.
- **Gates.** Run the full gates once per code phase on a final clean tree.
  While working, run scoped checks. After a failure, rerun only the failed
  suites.
- **GPU.** Use the RTX 3090s only, never the K620.
  `test-feature-complete.sh` pins `CUDA_DEVICE_ORDER=PCI_BUS_ID` and
  `CUDA_VISIBLE_DEVICES=0,1`.
- **Push and destructive steps.** Push explicit refs. Never force-push, never
  `--no-verify`, never push a `v*` tag. Removing worktrees or branches needs
  the owner's confirmation.

## Phase A — consolidate and prove stable

### A1. Containment (done 2026-10-21)

- Every Block 1 branch has 0 commits outside
  `origin/feat/agent-access-block1` at `b19551556`:
  - the slices `feat/aa-b1-s0`, `-s1a`, `-s1b`, `-s2`, `-s3`, `-s4`, `-s5a`
    to `-s5d`;
  - `feat/aa-b1-gatefix`, `-q2` and `-s6fix`;
  - `exp/aa-b1-remeasure`, `exp/agent-access`, `exp/agent-access-claude` and
    `exp/agent-access-claude-b`.
- Four branches exist only locally (`gatefix`, `q2`, `s6fix` and
  `remeasure`). All four are contained.
- `main` has not moved since the integration branch's merge base,
  `c888b7134`.

### A2 to A5. Gates on `b19551556`, run one after another, never concurrently

| Step | Command | Why |
| --- | --- | --- |
| A2 | `./scripts/agent-verify.sh` | The last green run (S6, `f88bf8189`) predates the start-check removal (`7ea8acc8c`) and the re-measure harness |
| A3 | `AGENT_LONG=1 ./scripts/check.sh` | The long gates have not run since EP-18 part A changed reader dispatch |
| A4 | `FATHOMDB_FEATURE_COMPLETE=1 bash scripts/test-feature-complete.sh` | Block 1 changed embed and reader dispatch, and no record shows this gate ran. [P1-4] |
| A5 | `cd src/ts && npm test` | `agent-test.sh` runs pytest but not the TypeScript tests, and `check.sh` only typechecks TS. [P2-12] |

- **A3 coverage**, verified in review [P2-3]:
  - AC-021: `fn ac_021_zero_sqlite_schema_warnings_under_concurrent_reads_and_ddl`;
  - AC-059b: `cursor_read_after_write.rs:21-36`;
  - AC-081a/b: `fn run_ac081_gate` and `fn ac_081_absolute_read_performance` in `perf_gates.rs`.
  - `perf_gates` builds only under `--workspace`, which `check.sh` uses,
    through dev-dependency feature unification.
  - AGENT_LONG also enables AC-012, 013 and 019 at 100k, and the AC-034
    soak, so expect a long run.
  - AC-081 runs here in a debug build, although `perf-gates.md:41` calls it
    release-only. Record this as a note, not a failure.
  - The A3 log must quote the suite names. `--quiet` hides them.
- **Flaky tests.** A2 and A3 already run `--test-threads=1`
  (`test-rust-workspace.sh --serial`). A `slice50_evidence` failure is
  therefore real, not the known parallel race (`TC-7a7bf5b0`), and blocks
  Phase A. [P2-2]

### A6. MCP conformance re-run

- **Method.** Rerun the official suite, `@modelcontextprotocol/conformance`
  0.1.16, and the Inspector commands.
  - Follow `dev/design/mcp-adapter/conformance-report.md` § 4, which is
    present on this branch.
  - Run against a release build of `fathomdb-mcp` at the A2 sha.
- **Pass:** the same classification as the report's § 5 table: no new
  failures.
- **Scope.** The report's untracked scratch probes (`session-check.mjs`,
  `raw_probe.py`) are out of scope. [P2-5]

### A7. LoCoMo agent smoke — skipped

- It is a priced run.
- A2's MCP admission tests (queue-admission § 9) and A6 cover the
  start-check removal.
- The owner can authorise a rerun separately. [P2-4]

**Exit A:** A2 to A6 green on one sha, each result quoted in
`dev/plans/agent-access-block1-plan.md`. Any other failure blocks Phase A: it
is fixed through its own implementer slice, RED first, and A2 to A6 rerun on
the new sha.

## Phase B — 0.8.29 todos on `main` [P0-1]

- **Why `main`.** 0.8.29 planning reads `main`, and Block 1 does not merge to
  `main`. Todos filed only on the integration branch would never be seen.
  Seq 289 and 290 already sit only on this branch; `main` ends at 288.
- **When.** After Exit A, so no gate runs on a changing tree. [P2-1]
- **Steps:**
  1. Cut worktree `aa-b1-todos` from `origin/main`.
  2. Cherry-pick `b2db64ba7` (seq 289) and `0822bc67d` (seq 290). The ledger
     line and the `.seq` sidecar arrive together.
  3. Open each todo below with `dev/agent-tools/ledgerwrite/ledgerwrite.py
     dev/todos-and-considerations-ledger.jsonl --profile todos --open --kind
     todo --field status=open --field target_release=0.8.29 --ref <sha or
     doc>`.
  4. Run `./scripts/agent-lint-md.sh` scoped, then commit, as a ledger-only
     docs commit.
  5. Push `main` as an explicit ref. If `main` has moved, rebase onto it
     first.
  6. In Phase D, merge `main` into the integration branch.
     - The 289/290 lines are identical on both sides.
     - The `.seq` sidecar conflicts; take `main`'s value.

| # | Todo | Source |
| --- | --- | --- |
| T1 | The EP-17 per-term snippet pass is unbounded per search (up to 20 × (1 + k) point queries). Bounded by C1. | `snippet_query.rs:57-84` |
| T2 | Full-body segmentation runs even for record-start snippets. This is not a no-op change: UAX #29 boundaries near the cut depend on text after it. Needs a proptest of full-body against prefix-plus-lookahead equality before any skip. [P1-5] | `fn select` and `fn words` in `src/rust/crates/fathomdb-engine/src/snippet.rs` |
| T3 | Measurements: the EP-17 M1 statement-count test and a multi-term cap test. Built in C1. Also an SDK-direct snippet latency receipt. | `match-centred-snippets.md` M1 |
| T4 | `#[non_exhaustive]` on the SDK option and result structs. Built in C3. TC-103(b) stays a separate item. [P2-8] | engine-changes evaluation |
| T5 | Fusion keys on `SearchHit.body`, so distinct records with byte-identical bodies merge into one hit, with a double score and one `source_id`. This affects text-only search too. Needs an ADR, a census of Memex duplicates, and a P2b rerun keyed on the cursor before any fix. A fix silently changes the default ranking. | `fn fuse_rrf` in `fusion.rs`; its call in `search.rs` |
| T6 | B2-8 long-record guard: the owner to approve it as a Block 2 row. Specify a tokenizer-only backfill, not a full re-embed (3.5–5 min per 19k store), and an `Embedder` truncation-report method that defaults to "unknown". | `long-record-embedding.md`; `fathomdb-embedder-api/src/lib.rs:48-64` |
| T7 | The facade does not re-export the snippet or hint types. Python and TypeScript snippet offsets are UTF-8 byte offsets; document them or convert them. | engine-changes evaluation |
| T8 | Validate EP-18 part A on the real engine with an SDK-direct head-of-line probe on the 19k store. Only the simulation shows the gain. | `queue-sim/REPORT.md` § 6, § 12 |
| T9 | Rerun `long-record-embedding/latency.py` on the Block 1 build; the baselines predate Block 1. | `long-record-embedding/results/` |
| T10 | Queue-admission § 9 test 23 is not buildable. Add a `before_enqueue` test hook so the drain race is tested. | block-1 report, 2026-10-19 |
| T11 | EP-03 still reports no degradation for an arm timeout or an absent embedder. | EP-03 |
| T12 | Compact text block: the GPT-6 Astra review findings are not applied (P1 quoted refs; P2 dropped fields, final-render budget, fetch framing, acceptance comparison). | `compact-text-block-astra-review.md` |
| T13 | Re-check the overflow-chunking triggers. This mirrors the ROADMAP 0.8.29 reminder. | `long-record-embedding.md` § 7.2 |
| T14 | The text-search slowdown at 5 concurrent calls, measured before Block 1, is unexplained. | engine-changes evaluation |
| T15 | Run the LoCoMo agent smoke on the closed Block 1 build, if the owner authorises the spend (A7). | A7 |

- The earlier draft's T13, a missing judge-fix test, is dropped:
  `a26d94f6a` adds `tests/test_measurement.py`. [P1-3]
- T1, T3 and T4 move to `done` through guarded updates
  (`--expected-prior-seq`) once C1 and C3 land, written on `main` in the same
  way.

## Phase C — close fixes

There are two implementer slices, run one after another, because both touch
`fathomdb-engine`:

1. **Slice C1:** item 2's measurement plus item 1's fix.
2. **Slice C3:** item 5, cut after C1 lands.

### Slice C1 — bounded per-term snippet pass (items 1 and 2)

**Contract** [P1-1, P2-10]:

- `max_point_queries` keeps its documented meaning, the number of hits that
  get a combined span query (its doc comment in `snippet.rs`, EP-17,
  and `rust.md`).
  - Its type stays `u8`, and its `0..=255` range in Python and TypeScript is
    unchanged.
  - The MCP adapter's setting, one per hit of the run window
    (`max_point_queries: u8::try_from(run_limit)` in `tools.rs`), is unchanged, so continuation pages stay anchored
    alike.
- **New bound.** The rule-3 per-term pass draws on a second per-search pool
  of the same size, `max_point_queries`.
  - A hit's per-term pass runs only if the pool still holds at least
    `terms.len()`, and then it costs `terms.len()`. It is all or nothing.
  - Otherwise the hit keeps its approximate snippet from the combined spans.
  - A search therefore issues at most 2 × `max_point_queries` point queries,
    whatever k is.
- **Unchanged:**
  - a hit past `max_point_queries` gets the record start;
  - `snippet_for` is untouched;
  - no new option field, so no surface growth.

**REDs**, each its own commit, each failing alone on today's code:

- **RED 1, `snippet_point_queries_stay_within_twice_the_cap_with_many_terms`.**
  - An engine `test-hooks` test counting `snippet_highlight` statement-trace
    events (label `snippet_highlight` in `src/rust/crates/fathomdb-engine/src/search.rs`, counter as in
    `fn highlight_queries` in `snippet_search.rs`).
  - Setup: 30 long hits under a 3-term query whose terms fall in different
    windows, with `max_point_queries = 20`.
  - Assertion: at most 40 events. Today there are 80, as the review
    measured.
  - This is M1 and the multi-term T-20.
- **RED 2, `per_term_pass_that_does_not_fit_keeps_the_approximate_window`.**
  - Uses the `rule3_exact_identity_prefers_the_window_with_both_terms`
    fixture (`fn rule3_exact_identity_prefers_the_window_with_both_terms`), where the approximate and exact
    windows differ, with `max_point_queries = 1` and two terms, so the pool
    of 1 cannot hold both. [P1-7]
  - Assertion: anchor `Match`; the text contains `running` and not `cat`.
    This is the approximate window.
  - Today the per-term pass always runs, picks the `cat` window, and the test
    fails.
- **RED 3, `mcp_multi_term_continuation_pages_anchor_alike`.**
  - An MCP test: a 3-term query over a run window larger than one page.
  - Assertion: every page's hits are anchored the same way as an unpaged run
    of the same window.
  - Before C1 this may already pass. It is a guard: if it passes on today's
    code it is **not** a RED and is committed with GREEN as a regression
    test, saying so plainly. Only RED 1 and RED 2 carry the fix.

**Existing tests that must pass unchanged:**

- T-20 (`t20_point_query_cap_anchors_the_rest_at_the_start`), which has one
  term;
- the `rule3_*` tests under the default cap of 20;
- the MCP continuation test (`fathomdb-mcp/tests/search.rs:536-562`).

**Docs, same slice:**

- the `snippet.rs` doc comment;
- EP-17;
- `match-centred-snippets.md` § 6.5, § 7.2 and a § 10 decision entry;
- `dev/interfaces/rust.md`, `python.md`, `typescript.md` and `mcp.md` (the
  2 × bound);
- `perf-gates.md`, if it carries the snippet budget.

**Cost receipt:** `dev/experiments/agent-access-snippet-cost/`. This is
measurement tooling, so it is tested for correctness only.

- Records of 2, 10 and 50 KB, k = 1, 3 and 5, snippets off and on; p50 and
  p95 per search.
- Run before and after the fix, on CPU (snippets do not use the GPU).
- Commit the summary only.

**Out of scope:** the segmentation skip, now T2, which needs its own proptest
first. [P1-5]

### Slice C3 — non-exhaustive SDK structs (item 5)

**Why it is justified now** [P2-8]:

- `main`'s `fathomdb-sdk/src/options.rs` has no `snippet` field, so Block 1
  already breaks literal construction of these structs in this release line.
- Marking them now adds no new break for a caller who upgrades past Block 1.
- TC-103 seq 178 (the HITL ruling that `#[non_exhaustive]` is breaking)
  applies; this note records why the venue is right.

**Targets:**

- `fathomdb-sdk` `SearchOptions`, `TextSearchOptions` and
  `FrozenSearchOptions`;
- engine `SnippetOptions`;
- engine `SearchExpandResult`.
- The implementer first confirms each one is reachable from the `fathomdb`
  facade or the SDK, and drops any that is not.
- `CompiledQuery` is in only if it is public through the facade.

**Shape: follow the repo precedent, no new builders** [P2-6]:

- The precedent is `SearchFilter` (`filter.rs:88-90`): `Default` plus
  mutation of the public fields, as in
  `let mut o = SnippetOptions::default(); o.max_bytes = 300;`
  (`slice85_filter_error_routes.rs:17-21`).
- No `with_*` setters, which would be new public surface.

**Sites that must change: 47** [P1-6]. Each moves from a struct literal or
functional record update (E0639) to `default()` plus field assignment:

| Sites | Where |
| --- | --- |
| 29 | `fathomdb-engine/tests` (21 in the selector tests, 8 in the search tests) |
| 9 | `fathomdb-sdk/tests/behavior.rs` |
| 5 | `fathomdb-mcp`: `src/tools.rs:921`, `:942`, `:946`; `src/seams.rs:305`; `tests/fetch_expand.rs:184` |
| 2 | `fathomdb-py/src/snippet.rs:24`, `:79` |
| 2 | `fathomdb-napi/src/snippet.rs:25`, `:89` |

- The implementer recounts them with `cargo check --workspace --all-targets`
  after adding the attribute, and lists every site in the slice record.

**RED, one per struct.** A `compile_fail,E0639` doctest that builds the
struct with a literal from outside the crate. [P2-9]

- It compiles today, so the doctest fails.
- `cargo test --workspace` runs doctests.

**GREEN:**

- Add the attributes.
- Convert the 47 sites, construction only: no assertion, expected value or
  test name changes.
- This is the API change the owner asked for, not fix-to-spec. The diff
  review checks that test files change only in construction lines.

**Docs and surface** [P2-7]:

- Update `dev/interfaces/rust.md`.
- Correct the false `..Default::default()` claims in `rust.md` and
  `filter.rs`, and the `SnippetOptions` doc comment in `snippet.rs`.
- The governed allowlist names don't change.
- Delete the stale "AWAITING HITL SIGN-OFF" comment at
  `fathomdb/tests/governed_surface.rs:115`; the owner approved 37 → 39 on
  2026-10-19. This is a comment-only edit.

### C exit

On the clean integration tip after C3 lands, run:

- A2, the full gate;
- A5, the TypeScript tests;
- A6, conformance, because C1 changes the snippet contract.

A3 and A4 are not rerun, because Phase C touches neither reader dispatch nor
embedding. The record says so.

## Phase D — close

- **Merge `main`** (the Phase B todos) into the integration branch, then rerun
  the scoped Markdown lint and `check-release-state-views.sh`.
- **Plan file.** Update the per-slice record in
  `dev/plans/agent-access-block1-plan.md` with:
  - Q2 (start check);
  - the re-measure merge;
  - A2 to A7;
  - C1 and C3;
  - the final gate results.
- **README.** Mark Block 1 CLOSED in `dev/design/agent-access/README.md`, and
  update the memory entry.
- **Todos.** Move T1, T3 and T4 to `done` on `main`.
- **Owner confirmation needed** (destructive), before removing:
  - worktrees `aa-b1-q2`, `aa-b1-exp`, `aa-b1-todos`, the C1 and C3
    implementer worktrees, `agent-access-claude`, `agent-access-claude-b`
    and `agent-access`;
  - the contained local-only branches. [P2-12]
- **Report to the owner:**
  - what landed;
  - gate results, quoted;
  - deviations;
  - the 0.8.29 todo ids.

Do not merge Block 1 to `main`.

## Open decisions (not blocking)

- **B2-8:** approve it as a Block 2 row (T6).
- **Test 23 hook (T10):** build it in Block 2 or 0.8.29.
- **Compact text block (T12):** who applies the review findings.
- **LoCoMo smoke rerun (T15):** a priced run.
