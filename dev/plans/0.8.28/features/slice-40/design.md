---
title: FathomDB 0.8.28 Slice 40 — design
status: REVISED AFTER R1 (APPROVE-WITH-FIXES, findings F1–F6)
target_release: 0.8.28
observed_on: 2026-10-15
---

# Slice 40 design

Mechanisms for the two defects in [requirements.md](requirements.md).
Both are small, local edits to pure functions. No public signature, schema,
knob or wire shape changes.

## 1. Defect A: one term per arm per body

### 1.1 Mechanism

In `fuse_three_arms` (`src/rust/crates/fathomdb-engine/src/fusion.rs`),
each of the three per-arm loops keeps a set of the bodies that arm has
already credited. A hit whose body is in the set is skipped before
`accumulate` runs; otherwise the body is added and the hit is accumulated
with its positional `rank0` exactly as today.

```text
for each arm (vector, text, graph):
    credited = {}
    for (rank0, hit) in arm:
        if hit.body in credited: continue
        credited.add(hit.body)
        accumulate(hit, rank0, in_vector, weight)   # unchanged
```

The per-arm set holds **entry indices**, not bodies (R1 F6). `accumulate`
already resolves a hit to its accumulator entry by body hash plus an
equality check against the stored entry's body (the `BodySlot` scheme).
Each arm passes its own `HashSet<usize>` of entry indices it has credited:

- the body resolves to an existing entry already in the arm's set: skip;
- it resolves to an existing entry not in the set: add the term, insert
  the index;
- it is a new body: create the entry as today, insert its index.

This is R1's "hash, then confirm equality against the stored entry" with
the hashing done once, by the lookup that already exists: no body clone or
second hash per hit, and no false skip on a hash collision because the
equality check decides. The accumulator, the `BodySlot` collision
handling, the sort and the score write-back are otherwise untouched.

### 1.2 Decisions

- **Positional rank of the first occurrence, not a compacted rank.** A
  skipped duplicate still occupies its position, so the hit after it keeps
  its original rank. Reasons: (1) the explain sidecar's `body_rank_map`
  (`search.rs`, "keeps the FIRST occurrence") reports `i`, the positional
  index of the first occurrence; compacting would make fusion's rank and
  the reported `text_rank` disagree; (2) the hits below a duplicate pair
  really were ranked below two rows by FTS5; (3) it is the smallest change.
  Within one arm, both choices give the same order; they differ only in
  cross-arm score weights by at most one rank step per skipped duplicate.
  R1 ruled Q1: keep the positional first-occurrence rank.
- **Keep keying on `body`, not `write_cursor`.** The body is the merge
  identity across arms: a vector hit and a text hit of the same row are
  joined by body, and the recall harnesses and explain maps key on
  `hit.body` (`dev/design/slice-10-design.md`). Keying on `write_cursor`
  would stop collapsing duplicate rows into one result, which changes the
  result shape (two hits with the same body), changes every corpus with
  duplicate bodies far more than this fix, and breaks the edge-vs-node
  collapse that Slice 23's prefix test relies on. Out of scope.
- **The skip is per arm, not global.** A body in both the vector and text
  arms still receives one term from each (REQ40-02).
- **Edge bodies.** The text arm is node FTS hits followed by edge FTS hits.
  An edge body equal to a node body now adds nothing (the node occurrence
  ranks first). Before, it added a deep extra term; for example, the
  `slice23_text_limit_prefix_stability.rs` fixture's node 10 scored
  `3/41 + 3/81` and ranked first. That test asserts only prefix stability,
  so it stays green.
- **Graph-arm call site.** With `use_graph_arm`, `search.rs` calls
  `fuse_three_arms(two_arm_fused, vec![], graph_candidates)`; the first
  list is already body-unique, so only duplicates inside the graph
  candidates change.

### 1.3 What must not change

The constants (`RRF_K`, the three weights, `RECENCY_WEIGHT`); the sort key;
vector-first representative identity; the `fuse_rrf` delegation; the
`apply_recency_reweight` and `apply_importance_reweight` passes; the
candidate SQL and its ordering; `rerank_fused`.

## 2. Defect B: keep 1–2 character tokens

In `content_tokens` (`src/rust/crates/fathomdb-query/src/lib.rs`), replace
`token.len() < 3 || stop.contains(token)` with
`token.is_empty() || stop.contains(token)`. The `is_empty()` check is
required: splitting on non-alphanumeric characters yields empty fragments
("alpha--beta", runs of spaces), and the length filter was what removed
them.

Unchanged: `STOPWORDS`, lowercasing, the split predicate, first-seen
de-duplication, the `"tok" OR "tok"` content shape, and the fallback
branch of `compile_text_query`. The fallback now fires only when no
non-stopword alphanumeric token exists. A query such as "D3" moves from the
fallback (`"D3"`) to the content path (`"d3"`). FTS5's `unicode61`
tokenizer folds case, so both forms match the same rows; the expression
string differs.

The removed filter was a byte-length check (`len()` on `&str`), so it also
dropped one-character non-ASCII tokens such as "é". Those are now kept,
which is consistent with the rule.

Every caller of `compile_text_query` picks this up: the body FTS arm and
edge FTS arm (`search.rs`), projected-text search (`search.rs`,
`read_projected_text_in_tx`), the hybrid path (`search_api.rs`) and graph
seeding (`graph_expand/execution.rs`).

## 3. Files touched in W4

| File | Change |
| --- | --- |
| `src/rust/crates/fathomdb-engine/src/fusion.rs` | per-arm credited set in `fuse_three_arms`; doc comment states "each arm contributes at most once per body, at its first occurrence's rank" |
| `src/rust/crates/fathomdb-query/src/lib.rs` | `is_empty()` in place of `len() < 3`; doc comments of `content_tokens` and `compile_text_query` |
| `src/rust/crates/fathomdb-query/README.md` | "tokens shorter than three characters" removed |
| `CHANGELOG.md` | two entries under "0.8.28 — unreleased" (R1 F3): fusion credits a body once per arm (result order can change on corpora with duplicate bodies); the compiler keeps 1–2 character tokens |

Already changed in W2 (tests, see [test-plan.md](test-plan.md)):
`tests/slice40_fts_ranking.rs` (new), the `fathomdb-query` test module,
and `tests/fts5_injection_safety.rs` (AC-038: reference mirror, two
short-token corpus rows, short tokens in the query alphabet).

## 4. Risks

| Risk | Effect | Handling |
| --- | --- | --- |
| Hybrid and text ranking shift on corpora with duplicate bodies | Duplicates fall back to their FTS rank; non-duplicate corpora are byte-identical (AC40-09) | Intended. Agent-memory corpora (Memex) hold many duplicates; note it in the 0.8.28 release notes as a ranking fix |
| Recall harness baselines (eu7, IR-C) move | A number recorded on a duplicate-bearing corpus can shift | eu7 measures the vector stage only (`vector_stage_only` skips fusion); others are experiment records, not gates |
| Short tokens add OR terms | More FTS5 work per query; tail latency rises slightly on long queries | The measured gain (0.3228 to 0.3260) outweighs it; the stop list stays, which was the large latency factor |
| Short tokens add false matches (single letters) | Small; bm25's IDF discounts frequent letters | Measured net gain on NFCorpus |
| Possessive and contraction fragments become OR terms (R1 F2) | "Cow's Milk" compiles to `"cow" OR "s" OR "milk"`, "don't" to `"don" OR "t"`. "s" and "t" occur in most English rows, so they enlarge the OR candidate set (latency) with near-zero IDF signal | Measured on NFCorpus: nDCG@10 0.3228 to 0.3260 and p95 2.24 to 2.30 ms with short tokens kept, so the net is positive. No stop-list change here; pinned by `possessive_and_contraction_fragments_are_content_tokens` and `text_only_possessive_query_ranks_topical_row_first`; adding s, t, ll, re, ve to `STOPWORDS` is a follow-up needing its own measurement |
| An existing test encodes the old behaviors | It fails at W4 | W5 runs the full gate; a failing test is analysed, not edited to pass, unless it encodes the ruled-defective behavior (then reported) |
| Explain and fusion ranks disagree | — | Positional first-occurrence rank keeps them equal (§ 1.2) |

## 5. Docs that describe the old behavior

Grep run on 2026-10-15 over `dev/adr`, `dev/interfaces`, `dev/design`,
`docs/` and the crate sources for "keyed on body", "RRF", "stopword",
"3 char", "short token", "shorter than three", "≥3", "sub-3":

- **Must change (W4):** `fathomdb-query/README.md` line 23 ("tokens shorter
  than three characters"); `content_tokens` doc comment ("≥3 chars");
  `compile_text_query` doc comment ("sub-3-char queries"); the
  `fuse_three_arms` doc comment (add the once-per-arm rule).
- **Already correct, no change:** `dev/design/slice-10-design.md` § 2 (one
  term per branch); `docs/guides/hybrid-search-filtering.md` ("over the
  branches that surfaced it"); `docs/guides/structured-search-hits.md`,
  `docs/reference/python-api.md` and `docs/reference/typescript-api.md`
  ("deduplicated on body"); the ADRs mention RRF only as
  `Σ 1/(rank + k)` over lists. No ADR or `dev/interfaces/` file states the
  three-character rule or within-arm accumulation, so no ADR or interface
  change is needed.
- **Stale for another reason, follow-up:** the public docs give
  `Σ 1/(60 + rank)`; the code uses `RRF_K = 30` and 3:1 text weighting.
- **Not changed:** `tests/support/ir_retrieval.rs` keeps its own ≥3-char
  copy; it is a frozen experiment harness.
