# Slice S3 — EP-03 `embedder_overloaded`

Branch `feat/aa-b1-s3`, worktree
`/home/coreyt/projects/fathomdb-worktrees/aa-b1-s3`, cut from
`feat/agent-access-block1` after S1 and S2 land. Crates: `fathomdb-engine`,
`fathomdb-sdk` (re-exports), `fathomdb-py`, `fathomdb-napi`, `fathomdb-mcp`.

## Read first

- `AGENTS.md` §§ 1, 3, 5, 6, 10.
- `dev/design/mcp-adapter/engine-proposals/EP-03-hybrid-search-degradation.md`
  in full. **Build only the `embedder_overloaded` reason.**
- `dev/design/agent-access/queue-admission.md` § 6.2 ("Former part C") and
  § 9 test 30.
- `dev/plans/agent-access-block1-plan.md` "Decisions taken in this plan".
- `ADR-0.8.27-rust-sdk-parity.md`, `dev/design/0.8.8-explain-and-telemetry-adr.md`
  (R-OBS-2).

## Scope and acceptance criteria

1. **Engine.** In the non-frozen hybrid path, the query-embedding error is
   discarded (`search_api.rs:543-550`). When it is `DispatchError::Saturated`
   or `DispatchError::QueuedExpired`, the search still runs text-only, and the
   result carries `degradations == [ArmDegradation { arm: Vector, reason:
   EmbedderOverloaded }]`. Every other path leaves `degradations` empty.
   - New public types in `search_types.rs`: `ArmDegradation { arm:
     SoftFallbackBranch, reason: DegradationReason }` and `DegradationReason`
     with the one variant `EmbedderOverloaded`; both `#[non_exhaustive]`, with
     docstrings. `SearchResult.degradations: Vec<ArmDegradation>` (empty
     `Vec`, no allocation, when nothing degraded). The other EP-03 reasons are
     not built.
   - Results, ranking, `soft_fallback` and `explanation` are unchanged. Check
     every `SearchResult` construction site (`search_api.rs`, `search.rs`)
     and every other read that embeds a query through the dispatcher; if
     another read can hit `Saturated`/`QueuedExpired`, report the same way or
     say why not.
   - `fathomdb-sdk` re-exports the new types.
2. **Bindings (parity, ADR-0.8.27).** Python and TypeScript expose
   `degradations` on their search result, present only when non-empty (Python
   `None` default; TS optional), with the wire word `embedder_overloaded` and
   arm word as the bindings already spell `soft_fallback.branch`. Add the
   reason to the parity map if one lists degradation words (find it).
3. **Adapter.** `fathomdb-mcp` maps each `degradations` entry to its
   `degradation` list entry `{arm: "vector", reason: "embedder_overloaded"}`
   in `tools.rs` (near `:668-688`). Keep the existing entries; their removal is
   EP-03's later work, not this slice.
4. **Docs in the same slice.** `dev/interfaces/rust.md`, `rust-sdk.md`,
   `python.md`, `typescript.md`, `mcp.md` (the degradation reason list), and
   EP-03's status (the `embedder_overloaded` part built).

## RED tests

| Test | Layer |
| --- | --- |
| `a_saturated_embed_queue_is_reported_as_embedder_overloaded` (§ 9 test 30) | engine integration test, real store, real embed dispatcher held busy until the waiting queue is full (find the existing dispatcher test hooks; add one behind `test-hooks` in its own commit if needed) |
| `a_queued_embed_request_past_its_deadline_is_reported_as_embedder_overloaded` | engine, `QueuedExpired` path |
| `degradations_leave_results_and_soft_fallback_byte_identical` | engine; EP-03's byte test on a fixed fixture with a healthy embedder: `results`, `soft_fallback`, `projection_cursor` unchanged and `degradations` empty. This pins; if it passes on arrival commit it with the first GREEN and say so |
| `python_search_result_reports_embedder_overloaded` / TS equivalent | bindings; if a binding test cannot reach a saturated dispatcher without engine test hooks, test the conversion of a constructed engine value at the binding boundary instead and say so |
| `hybrid_search_reports_embedder_overloaded_degradation` | `fathomdb-mcp`; needs the `default-embedder` feature and a way to saturate the dispatcher through the SDK. If no non-mock route exists, test the adapter's mapping function on a real `SearchResult` value and say so |

## Review fixes (Fable-medium brief review, APPROVE-WITH-FIXES) — binding

These override the text above where they differ.

1. **Frozen path is in scope.** `dispatch_embed_vector` is called at
   `search_api.rs:544` (non-frozen), `search.rs:1424` (frozen, inside
   `read_search_in_tx`, also `Err(_) => None`) and `embedding.rs:162`
   (`embed_text`, already an error). Graph reads never embed. Capture the
   frozen error and thread it into the frozen `SearchResult` (`search.rs:815-824`;
   `EvidenceSearchResultV1.search_result` carries it). Extra RED:
   `a_frozen_search_reports_embedder_overloaded`.
2. **`search_expand`** returns `SearchExpandResult` (`graph_expand/traversal.rs:45-53`),
   not `SearchResult`: add `degradations` to it too, with a RED
   (`search_expand_reports_embedder_overloaded`).
3. **Engine saturation route (no new hook):** `EmbedderChoice::Caller` with a
   gated embedder (blocks on a channel in `embed`), `EngineConfig {
   embedder_pool_size: Some(1), embedder_call_timeout_ms: .. }` →
   queue capacity 4; 1 active + 4 queued → the 6th hybrid search gets
   `Saturated` (`embed_dispatch/core.rs:441-446`); a queued request past a
   short deadline gives `QueuedExpired` (`core.rs:186-192`). Precedent:
   `tests/pr9_embed_serialization.rs:33-44`.
4. **Bindings cannot reach a saturated dispatcher.** Test the conversions:
   Rust-side `PySearchResult::from_rust` and napi `SearchResult::from` in-crate
   unit tests on a real engine `SearchResult` value; the wrapper mappers with
   the existing patterns (TS `mapNativeSearchResult`,
   `src/ts/tests/slice55-absent-native-fields-use-legacy-defaults.test.ts`;
   Python `_map_native_search_result`,
   `src/python/tests/test_slice55_wrapper_compat.py:54-58`). Arm words reuse
   the existing `soft_fallback` match (`fathomdb-py/src/read_search.rs:269-273`,
   napi `read_search.rs:493-497`).
5. **Adapter test:** extract the inline mapping at `tools.rs:668-688` into a
   function (e.g. `search_degradations`) or carry `degradations` on
   `SearchRun`, and unit-test it on a real `SearchResult` produced by an
   engine opened through the SDK path the adapter uses. If the SDK cannot
   yield a saturated result, test with a `SearchResult` built from a real
   engine run in the engine test and say so. No adapter-level concurrency
   saturation test (flaky by construction).
6. **Re-exports:** also the umbrella crate `fathomdb/src/lib.rs:186`.
7. **Parity map:** none lists degradation words; add the word to
   `mcp.md` (degradation reasons), `python.md`, `typescript.md`.
8. **Hand-forward:** S4 adds `embedder_overloaded` to the hints `cause` set
   (already in the S4 brief).

## Python and TypeScript

The worktree needs its own `.venv`: run `./scripts/bootstrap.sh` inside this
worktree (it creates a checkout-owned `.venv`). Never install into another
checkout's `.venv`. TS: `npm install` in this worktree's `src/ts`.

## Rules

- TDD per `tdd-red-must-fail-per-finding`, `RED evidence:` / `GREEN
  evidence:` in bodies; audit RED shas in a detached throwaway worktree.
- No schema change. No mocked database.
- Scoped checks only (the named tests, `cargo clippy` on the touched crates
  with `--all-targets -- -D warnings`, `cargo fmt --check`, the binding test
  files, `./scripts/agent-lint-md.sh`). Do not run `./scripts/agent-verify.sh`.
- Never `--no-verify`, never push.
- Commit messages end with
  `Claude-Session: https://claude.ai/code/session_01RBbKrBEAJUjsNgeFwfAUbr`.

## Report

The commit list (sha, RED/GREEN/pin, test), failure evidence, scoped check
results quoted, deviations with reasons.
