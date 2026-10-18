# Slice S5 — match-centred snippets (EP-17), sub-slices S5a–S5d

Each sub-slice is its own branch and worktree, cut from the current tip of
`feat/agent-access-block1` after the previous one lands:
`feat/aa-b1-s5a` … `feat/aa-b1-s5d`, worktrees
`/home/coreyt/projects/fathomdb-worktrees/aa-b1-s5<x>`.

## Read first (all sub-slices)

- `AGENTS.md` §§ 1, 3, 5, 6, 10.
- `dev/design/agent-access/match-centred-snippets.md` §§ 3, 6, 7, 8, 10. It is
  the specification; this brief does not restate it.
- `dev/design/mcp-adapter/engine-proposals/EP-17-match-centred-snippets.md`.
- Owner decisions (§ 10): `snippet_bytes` 300; MCP keeps a record head
  snapped to a word boundary at or before `head_bytes`; highlights off by
  default with an opt-in argument; exact term identity by one `highlight()`
  per OR term under the § 6.3 rule 3 conditions; vector-only hits get the
  record-start fallback.
- `ADR-0.8.27-rust-sdk-parity.md`, `dev/design/0.8.8-explain-and-telemetry-adr.md`
  (R-OBS-2).

## S5a — pure selector

Crates: `fathomdb-engine` (new pure module, e.g. `snippet.rs`), `fathomdb-sdk`
(re-export). `SnippetOptions`, `SnippetAnchor`, `SnippetV1` and
`snippet_for(body, spans, options) -> SnippetV1` per § 6.1 (grapheme widening
of span ends), § 6.2–6.6 and § 7.2. Validation of option ranges
(`InvalidArgument`). No database. `unicode-segmentation` is already in
`Cargo.lock`; add it as a dependency of the engine if it is not one.

RED tests: T-4 (proptest), T-5, T-11, T-17, T-18, plus focused unit tests
that drive each rule of § 6.3 (rule 1 whole body; rule 2 prefix on a word
boundary with `" …"`; rule 4 scoring; rule 5 centring; rule 6 snapping; rule
8 merge; rule 9 assembly) and § 6.6 head. Order so each rule is its own
RED → GREEN.

Docs: `dev/interfaces/rust.md` and `rust-sdk.md` for the new types and
function.

## S5b — engine point queries and sidecar

Crates: `fathomdb-engine`, `fathomdb-sdk`. `SearchOptions.snippet` and
`TextSearchOptions.snippet`; `SearchResult.snippets: Option<Vec<SnippetV1>>`;
FTS rowid carried in a search-local map (EP-17 mechanism step 1); point
`highlight()` queries with per-call marker choice (§ 6.1) after truncation and
the identity pass, in the same reader transaction; the exact-identity
per-term pass (rule 3); `max_point_queries` cap; anchor rule; frozen reads
untouched in eligibility and digest; `search_projected_text` left out.

RED tests: T-1, T-2, T-3, T-6, T-7, T-8, T-9, T-10, T-14, T-15, T-16, T-19,
T-20, on real stores. T-7 is a pin if it passes on arrival. T-6 needs a hybrid
hit found only by the vector arm: use a real embedder only if the test
features already allow it; otherwise find how existing engine tests get
vector-only hits.

Docs: `rust.md`, `rust-sdk.md`; EP-17 status (engine built).

## S5c — adapter

Crate: `fathomdb-mcp`. § 7.3 in full: request snippets with `max_bytes:
snippet_bytes`, `head_bytes` (MCP default: pick and justify a value near 48
bytes from the LoCoMo record header `(unknown date) [8 May 2023] Caroline:`),
`highlights: false` unless the new opt-in argument asks; hit field
`snippet_anchor`; `snippet_cut` from `snippets[i].cut`; delete
`shape::snippet` as the source; `cut_bodies` re-runs `snippet_for`; config
`snippet_chars` → `snippet_bytes` with no alias; server instructions text;
hint edits listed in § 7.3 (the `snippet_cut` hint text keyed on
`snippet_anchor`). If the hints design (S4) named a hint for vector-only hits,
build it; it does not, so record that none is added.

RED tests: T-13, plus one RED each for `snippet_anchor` on the wire,
`snippet_bytes` config (and `snippet_chars` now rejected as unknown), the
highlights opt-in argument, and the hint text keyed on anchor. MR-041.1 budget
property must still hold.

Docs: `mcp.md`, `requirements.md` MR-040 (bytes), `edge.rs` instructions,
`hints-sidecar.md` edits named in § 7.3.

## S5d — bindings parity

Crates: `fathomdb-py`, `fathomdb-napi`, plus `src/python/fathomdb` and
`src/ts`. The options, the sidecar and `snippet_for` in Python and TypeScript
(T-12), the parity map entries, `python.md`, `typescript.md`. The worktree
needs its own `.venv` (`./scripts/bootstrap.sh` inside the worktree) and `npm
install` in its `src/ts`; never install into another checkout's `.venv`.

RED tests: T-12 split per binding and per surface (options accepted; sidecar
returned; `snippet_for` parity on a shared fixture).

## Review fixes (Fable-medium brief review, APPROVE-WITH-FIXES) — binding

These override the text above where they differ.

1. **S5c head size (blocker).** `head_bytes` must be in `0..=max_bytes/2`.
   MCP uses `head_bytes = min(48, snippet_bytes / 2)`, computed at config
   validation, stated in `mcp.md`. (48 is the default; justify it from the
   LoCoMo header in the commit body.)
2. **S5b term list.** `CompiledQuery` exposes only `match_expression`
   (`fathomdb-query/src/lib.rs:21-23`). Add `terms: Vec<String>` (the OR
   terms as compiled) to `fathomdb-query`, document it in `rust.md`. Never
   re-split the expression string.
3. **S5b frozen.** Add `snippet` to `FrozenSearchOptions`
   (`fathomdb-sdk/src/options.rs:112-119`) too; S5d adds it to the py/napi
   `search_frozen` arguments.
4. **S5b keeps the workspace building.** Add new engine entry points (or a
   trailing `Option<&SnippetOptions>` with the old wrappers kept), so py/napi
   (`fathomdb-py/src/engine.rs:502,590`, `fathomdb-napi/src/engine.rs:526,584`)
   compile unchanged. Update the literal/destructure sites
   (`fathomdb-mcp/src/tools.rs:647`, `fathomdb-sdk/src/engine.rs:186-195`,
   `fathomdb-sdk/tests/behavior.rs`, `fathomdb-mcp/tests/fetch_expand.rs`).
   Scoped check includes `cargo check --workspace --all-targets`.
5. **S5b first RED** is "sidecar is `Some` when requested", so later REDs fail
   on assertion, not compilation. Rowid is added to all five text SELECTs
   (`filter.rs:697`, `search.rs:1855/1881/1971/1997`) and the edge SQL
   (`filter.rs:708-715`). `bm25f_search` / `search_index_v2`
   (`search_api.rs:1256-1264`) is left out beside `search_projected_text`.
6. **S5b test routes (no mocks).** T-6: `Engine::open_with_embedder_for_test`
   (`open.rs:697`) with a hand-rolled `Embedder` (pattern
   `tests/slice45_pagination.rs:18-35`). T-9: a second rusqlite connection
   holds `BEGIN IMMEDIATE` while the search runs; assert through the
   `test-hooks` statement trace that no statement writes. T-3 uses
   `highlights: true`. T-20 uses `search_text_only` with limit 30 and counts
   point queries through the trace.
7. **S5c sanctioned test replacements.** `snippet_chars` → `snippet_bytes`
   changes existing tests (`tests/search.rs:150-170`, `tests/config.rs`,
   `tests/protocol.rs:40`). The owner-approved design (MR-040 to bytes, no
   alias) sanctions replacing them: each replacement goes in the RED commit
   that states the new byte contract, named in its body. S5c may update the
   S4 baseline fixture once, in the GREEN that adds `snippet_anchor`.
8. **S5c rename consumers:** `examples/locomo_eval/main.rs:289`,
   `experiments/mcp_agent_loop.py:955`, `dev/experiments/mcp-agent-loop/README.md:58`,
   `dev/experiments/mcp-hybrid-vs-keyword/README.md:161`,
   `hints-sidecar.md`, `dev/design/mcp-adapter/design.md`,
   `dev/design/agent-access/README.md`; grep for any other.

## Rules (all sub-slices)

- TDD per `tdd-red-must-fail-per-finding`: one RED commit per behaviour, fails
  at its sha, `RED evidence:` / `GREEN evidence:` in bodies; a test that
  passes on arrival is a pin, not RED. Audit RED shas in a detached throwaway
  worktree.
- Tests read-only once committed RED. No mocked database. No schema change.
- Scoped checks only: the named test files, `cargo clippy` on touched crates
  `--all-targets -- -D warnings`, `cargo fmt --check`, the binding test files,
  `./scripts/agent-lint-md.sh`. Do not run `./scripts/agent-verify.sh`.
- Never `--no-verify`, never push.
- Commit messages end with
  `Claude-Session: https://claude.ai/code/session_01RBbKrBEAJUjsNgeFwfAUbr`.

## Report (each sub-slice)

The commit list (sha, RED/GREEN/pin, test), failure evidence, scoped check
results quoted, deviations from the design with reasons.
