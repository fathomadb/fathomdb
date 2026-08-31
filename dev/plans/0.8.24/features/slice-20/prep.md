---
title: 0.8.24 Slice 20 — engine performance preparation
status: READY-FOR-DESIGN-REVIEW
target_release: 0.8.24
---

# Slice 20 — engine performance preparation

**Observed:** 2026-08-24 in the isolated `release/0.8.24-slice20`
worktree.

## Goal and baseline

Integrate the already-selected SCALE-02 streamed FTS rank-boundary behavior as
a minimal current-code change. Correctness tests, not a new timing run, are the
integration gate.

| Fact | Evidence |
| --- | --- |
| Current remote-main engine base | `5e2a05e281571024a3e7bb305373915597a54078` |
| Release branch at Slice 20 start | `d478f82b91c690f937b4583ca81634472355b225` |
| Retained experiment branch | `experiments/performance-0.8.23-plan-20260821` |
| Measured candidate | `9e507553f954c56d5c6177eabf1750faddf3acfd` |
| Production-behavior source | `c7e83bfe2469e6df503fd2d2ba8e5d4c795a0b53` |
| Result artifact manifest | `bad0098141509c6b960e0cad95a149020543c8f0fe375ab3ce9b116b45ee594b` |
| Owner selection | `seq-267`: `stream_default`; no confirming benchmark |

The Slice 20 branch was first verified as an ancestor of `release/0.8.24`, then
fast-forwarded to it. This preserves completed prework and Slice 10 while the
engine remains based on current `origin/main`.

## Draft-plan review

The draft plan is **approved with these adjustments**:

- the implementation is reconstructed from reviewed behavior rather than
  importing the experiment branch;
- experiment route/boundary/query-plan files and connection-setting witnesses
  are excluded from shipped production code;
- the forced-full-sort oracle and route observation may exist only behind the
  private `test-hooks` feature;
- the accepted writer `WAL + synchronous=NORMAL` durability invariant is
  restored explicitly because current main sets writer WAL but leaves
  `synchronous` ambient; reader-pool/runtime and all other connection settings
  remain unchanged;
- existing public requirements and direct-text prefix contracts are retained;
  Slice 20 drafts are local refinements, not canonical replacements; and
- correctness proof covers both the candidate-boundary helper and the full
  public direct-text path.

No relevant engine, retrieval-design, requirement, acceptance, or public
interface change landed after the draft plan was written. Slice 10 added only
CI planning records and has no engine interaction.

## Assigned work and current-code map

| Assigned function | Exists now | Slice 20 net-new behavior |
| --- | --- | --- |
| Direct-text API candidate bound | `search_text_only*_with_limit` supplies `direct_text_candidate_limit = 100`. | None. Preserve the bound for every public limit `1..=100`. |
| FTS node collection | `read_search_in_tx` performs `ORDER BY bm25(...), write_cursor LIMIT 100`. | For eligible direct-text requests, stream `ORDER BY rank` through the score group crossing row 100, restore `(score, write_cursor)` order, then truncate to 100. |
| Exact fallback | Current join, legacy-source, and plain-schema queries implement stable full sort. | Reuse the current join query after any streamed prepare, step, or conversion failure and for every ineligible request. |
| Edge behavior | Edge-body FTS is collected separately and fused with node hits. | Any database containing edge FTS rows remains on the existing exact full-sort node path. |
| Filter behavior | Node metadata filters are applied after node FTS collection. | Filtered searches remain on the existing full-sort path. |
| Hybrid/vector behavior | `direct_text_candidate_limit` is absent for hybrid search. | No change. |
| Public truncation/prefix | Fusion/dedup occurs before final `results.truncate(final_limit)`. | No change; the streamed path must supply the same first 100 node candidates. |
| Reader/cache/mmap/temp-store defaults | Owned by current open/runtime connection code. | No change. |
| Accepted durability mode | ADR-0.6.0 requires writer `WAL + synchronous=NORMAL`; current main explicitly sets writer WAL but not `synchronous`. | Restore writer `synchronous=NORMAL` and verify through a narrowly private test witness. Reader-pool/runtime remain unchanged. |

## Retained lineage and prerequisite disposition

The experiment branch diverged before later current-main WAL attribution work,
so its aggregate diff is not an integration unit. The semantic lineage is:

1. `e5e8e13a`, `1e856a3c`, and `718c21d9` established experiment harness
   candidate-window and overfetch behavior. Their experiment files are not
   production prerequisites.
2. `68f59ea8` introduced an earlier rank-fast experiment path. It was
   superseded by boundary-tie completion and is not copied.
3. `9e507553` introduced the measured `ORDER BY rank` boundary completion and
   helper behavior. This is the semantic prerequisite.
4. `c7e83bfe` removed experiment selection knobs and promoted the stream path
   for eligible production direct-text requests. Its minimal engine behavior,
   not its whole commit, is the implementation source.

The broad experiment witnesses adjacent to these commits are not prerequisites
to the rank-boundary algorithm. The writer-only `synchronous=NORMAL` setting is
a separate accepted-ADR correction retained from review; its narrow observer
remains private to `test-hooks`. Reader/runtime connection changes are excluded.
The current-main `f0712fa5` WAL-attribution change is retained untouched.

## Exact allowlist

Implementation is limited to:

- `src/rust/crates/fathomdb-engine/src/lib.rs`;
- `src/rust/crates/fathomdb-engine/Cargo.toml` only if a private integration
  test must declare `test-hooks` explicitly;
- one focused engine integration test and helper property coverage;
- `dev/design/retrieval.md` for the internal algorithm description; and
- Slice 20 local records plus the maintained document indexes/master-plan
  status links.

Schema, SDK/binding source, public interface documents, query compiler,
hybrid/vector code, connection defaults, benchmark runners, experiment
ledgers, and external artifacts are excluded.

## No-rerun disposition

The retained 60-repetition result and owner ruling are the performance decision
basis. Slice 20 will not execute SCALE-02, a reduced-N timing check, or any
other confirming performance run. Local execution is limited to correctness,
format, lint, type/build, and repository verification.
