# Slice S4 — hints sidecar, stage 1

Branch `feat/aa-b1-s4`, worktree
`/home/coreyt/projects/fathomdb-worktrees/aa-b1-s4`, cut from
`feat/agent-access-block1` after S3 lands. Crates: `fathomdb-engine` (new pure
module), `fathomdb-sdk` (re-export), `fathomdb-mcp`.

## Read first

- `AGENTS.md` §§ 1, 3, 5, 6, 10.
- `dev/design/agent-access/hints-sidecar.md` §§ 2, 3, 4, 5, 7, 8 (Block 1
  tests 1–18), 10, 11. It is the specification; this brief does not restate
  it.
- `dev/design/agent-access/match-centred-snippets.md` § 7.3, last bullet
  group: those hint edits belong to S5c, not here. In S4 the `snippet_cut`
  source is today's `snippet_chars` cut.
- `dev/interfaces/mcp.md`, `dev/interfaces/rust.md`, `dev/design/perf-gates.md`.

## Scope

- **Engine subset** `fathomdb-engine/src/hints.rs`: pure, no SQL/I/O/clock/
  locks. `Hint`, `HintAction`, `HintReason` (`NoResults`, `Degraded`,
  `MoreResults`), `HintCause`, all `#[non_exhaustive]`, with docstrings;
  `pub fn search_hints(facts: &SearchFacts, out: &mut Vec<Hint>)` per § 3.2.
  `fathomdb-sdk` re-exports. No engine search path changes (stage 2 is out
  of scope).
- **Adapter wire type** in `output.rs` with `From<fathomdb_sdk::Hint>`, the
  adapter reasons `snippet_cut`, `body_cut`, `cut_by_budget`,
  `default_store_set`, and the conversion of `more_results{limit}` →
  `more_results{continuation}` (dropped without a continuation; replaced by
  `cut_by_budget` on a budget cut). Hints planned once before the budget
  loop (§ 4.1). Bounds and order of § 2.2. Trust rules of § 2.3, including
  JSON-quoted refs in hit lines **and** hint lines of the text block.
- **`cause` set** includes `embedder_overloaded` (added by S3 to the adapter's
  degradation reasons), besides the § 2.1 list. Update the vocabulary table
  in `mcp.md` accordingly.
- **Configuration** § 5: process key `hints` (default `true`), per-call
  `hints` argument on every tool (including continuation calls), precedence
  per call → process → built-in; not part of `CanonicalSearch`.
- **Budget** § 5.1, and its sentence in `mcp.md` § 3.
- **Server instructions:** one sentence, "`hints` lists suggested next
  calls".
- **Docs in the same slice** (§ 7, stage-1 rows): `mcp.md` §§ 2, 3, 4;
  `rust.md` (engine subset); `requirements.md` new MR for hints; the
  `perf-gates.md` row of § 4.3.4, added with the tests; the ADR: file § 11 as
  `dev/adr/ADR-0.8.29-agent-hints-sidecar.md` (or the repo's next ADR naming;
  check `dev/adr/` and the decision index), status **accepted** (owner
  approval 2026-10-18), with decision (c) noting the SDK default is off until
  the 0.8.29 tests decide; add its decision-index row. Its first commit is
  docs-only and comes first.
- Block 2 (B2-1 … B2-7) is **not** built. The shapes must let B2 add entries
  without redesign; note in the report any place where that looks at risk.

## RED tests

hints-sidecar § 8, Block 1, tests 1–18, by name. Pure engine-subset unit
tests for `search_hints` come first (they drive `hints.rs` without the
adapter). Order so no test forces a big-bang GREEN: engine subset, then
`no_results`/`degraded` wiring, then `snippet_cut`, `more_results`/
`cut_by_budget`, `body_cut`, `default_store_set`, config/argument, then the
property, determinism, schema, quoting, allocation and timing tests.

- Test 16 (`hints_allocation_deltas`) is its own test binary with a counting
  `#[global_allocator]`; test 17 is the always-on timing test, blocking only
  at 200 µs, warnings for 10/25 µs. Both need a shaping-function test seam
  that reports the budget-loop iteration count (§ 4.3).
- Test 11 (`hints_off_output_is_byte_identical_to_baseline`) compares against
  today's determinism fixture bytes; it should pass on arrival and is a pin —
  commit it before any hints code lands, labelled as a pin, and it must stay
  green after every GREEN. Note that quoting refs in hit lines (test 18)
  changes text-block bytes with hints off too; resolve that with § 2.3 (quoting
  is unconditional) and update the baseline only in test 18's GREEN commit,
  saying so — this is the one permitted test edit, owner-sanctioned by the
  design's resolution of open question 5.
- Each RED fails at its own sha with `RED evidence:`; GREEN carries `GREEN
  evidence:`. A test that passes on arrival is a pin, not RED.
- Integration tests use real stores through the in-process protocol harness.

## Review fixes (Fable-medium brief review, APPROVE-WITH-FIXES) — binding

These override the text above where they differ.

1. **Shaping seam first.** Before the first adapter RED, a mechanical
   refactor commit extracts the search budget loop (`tools.rs:731-791`) into a
   pure, synchronous function reachable from `tests/` behind `test-seams`,
   e.g. `fn shape_search_page(plan, page, …) -> (Rendered, kept, iterations)`,
   and the same for `fetch`/`expand` body cutting if tests need it. The
   existing determinism and budget tests pin the refactor (no behaviour
   change). Tests 2, 3, 10, 16, 17 use this seam.
2. **Allocation test (16).** BLOCK on `off − baseline == 0` and on the
   *planning* step's delta (`plan_hints(...)` ≤ 1 + refs). The render step's
   on-vs-off delta is measured and printed, notify-only, with the measured
   multiplier recorded in the report and in the perf-gates row. The binary
   has exactly one `#[test]`, no tokio, and counts on the test thread only
   (thread-local counter).
3. **Timing test (17).** Choose `max_tokens` so the seam reports one
   budget-loop iteration; BLOCK at 200 µs per iteration at `limit` 100.
4. **Test 11 baseline.** No golden file exists (`tests/determinism.rs`
   compares two lifetimes). The pin commit records today's bytes of the
   tool-call responses (not `initialize` or `tools/list`, whose schema and
   instructions change by design) under `tests/fixtures/`. This is a
   regression snapshot of the pre-change output that the design's test 11
   calls for, not a generated oracle; say so in the commit body. Test 18's
   GREEN is the one later edit of it (quoting), and S5c may edit it once
   for `snippet_anchor`.
5. **`default_store_set`** is a Block 1 reason (§ 2.1): build the reason with
   its own RED, `list_stores_emits_default_store_set_hint`. The
   `list_stores.default_store` field and the instructions wording stay B2-6.
6. **`body_cut.max_tokens`** = the whole-response estimate from the first
   `excess` call on uncut bodies (`tools.rs:931-935`, `:1088-1092`), not
   "bytes cut ÷ 4 × 2".
7. **Condition 4 of § 3.3** (`dense_disabled`) is tested through the pure
   `search_hints` with `hybrid_available = false`; say so in test 5.
8. **`HintCause`** in the engine carries every cause word (`soft_fallback`,
   `dense_disabled`, `embedder_unavailable`, `embedder_overloaded`, graph
   codes); document in `rust.md`.
9. **Pins expected:** 9, 13, 15. Test 15 includes a negative assertion (an
   unknown `reason` is rejected by the schema); test 13 asserts `"hints":[`
   is present. Test 12 covers six combinations (call none/true/false ×
   config true/false). Test 1 uses `max_tokens` ≈ 20 000. Test 18 covers
   `search`, `fetch` and `expand` lines.
10. **Continuation calls:** add `args.hints` to the `only_paging` exclusion
    and its error string (`tools.rs:566-582`), and `mcp.md` § 5.
11. **ADR:** `dev/adr/ADR-0.8.29-agent-hints-sidecar.md` (no live release;
    newest ADRs are 0.8.27), status accepted citing the owner ruling of
    2026-10-18 recorded in hints-sidecar § 10; decision-index row 62. New MR
    is MR-072. The perf-gates row fills all six columns and names both test
    binaries.
12. Keep the wire `Hint` one struct with optional fields (or an internally
    tagged enum) so Block 2 adds fields. `state`/`verdict` stay as they are
    until B2-1.

## Rules

- Scoped checks only: `cargo test -p fathomdb-mcp`, `cargo test -p
  fathomdb-engine --lib hints` and the new test files, `cargo clippy` on the
  touched crates `--all-targets -- -D warnings`, `cargo fmt --check`,
  `./scripts/agent-lint-md.sh`. Do not run `./scripts/agent-verify.sh`.
- Audit RED shas in a detached throwaway worktree.
- Never `--no-verify`, never `pip install -e`, never push.
- Commit messages end with
  `Claude-Session: https://claude.ai/code/session_01RBbKrBEAJUjsNgeFwfAUbr`.

## Report

The commit list (sha, RED/GREEN/pin, test), failure evidence, scoped check
results quoted, measured overhead numbers from tests 16–17, deviations with
reasons, and any Block 2 shape risk.
