---
title: EP-14 — Exact literal constraint on search
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. A must-contain literal on search and search_text_only, compiled as an injection-safe FTS5 phrase ANDed with the relevance query and applied before every cap, so rare identifiers and short tokens can be required exactly.
---

# EP-14 — Exact literal constraint on search

Status: Draft. Priority P2. Not an accepted decision.

## Problem

`compile_text_query` (`fathomdb-query/src/lib.rs:71-87`) turns a query into an
OR over content tokens:

- It lowercases the query and splits on every non-alphanumeric character. It
  drops tokens shorter than 3 characters and stopwords
  (`lib.rs:43-56`).
- So an identifier such as `AB-12` or `v2.1` splits into short pieces that are
  dropped. A query of only such tokens falls back to an OR of whitespace tokens
  as quoted phrases (`lib.rs:73-80`). Mixed with ordinary words, the identifier
  can vanish from the match expression entirely.
- There is no way to require a token or phrase. `SearchFilter` has metadata
  fields and declared attributes only (`fathomdb-engine/src/filter.rs:90-104`).
  The `Json` predicate is rejected on search by `ADR-0.8.11-filter-grammar-unification.md`
  (D3), which is a different thing from a lexical constraint.

Adapter status: MR-020 is reported `unavailable`, and there is no workaround.
The adapter must not post-filter results (MR-070).

## Proposed surface

`SearchOptions.require_literals: Vec<String>` and the same field on
`TextSearchOptions`, at most 8 literals of at most 256 bytes each.

- Each literal is tokenized by the FTS tokenizer and emitted as one quoted FTS5
  phrase, with `"` doubled, the same injection-safe quoting as
  `compile_text_query`'s fallback.
- The text arm's MATCH becomes `(<relevance OR-expr>) AND <phrase>...`.
- Literals are applied before the FTS candidate limit. The vector arm cannot
  test a phrase before KNN. Under the eligibility ADR's rule, it declines with a
  typed soft fallback (`EligibilityDegraded` in EP-03) instead of post-filtering.
- Errors: `InvalidArgument` (`TooManyLiterals`, `EmptyLiteral`, `LiteralTooLong`
  in EP-11's terms).

## Invariants and consistency

- Every returned hit's indexed text contains each literal as an FTS phrase
  under the store's tokenizer. "Exact" means tokenizer-exact, not byte-exact.
  The documentation must say so. For example, `AB-12` matches `ab 12`.
- With no literals, the MATCH expression and the results are byte-identical to
  today.

## Interaction with ADRs

- `ADR-0.8.25-eligibility-and-frozen-reads.md`: this is a new pre-cap
  predicate. It fits the rule, but it widens the search vocabulary that ADR
  fixed, so it needs an amendment.
- `ADR-0.8.1-deferred-f5-fielded-fts-bm25f.md` and the BM25F index
  (`search_index_v2`) may need the same constraint. I did not check which index
  the text arm queries in every configuration.
- Frozen contexts encode eligibility. Literals would become part of the context
  digest only if they are made eligibility, not query. That is an open question.

## Cost and performance

A phrase AND narrows the FTS match set, which is usually cheaper. The vector
arm is skipped when literals are present, so hybrid searches with literals are
lexical in effect.

## Test plan sketch

Real database: a record containing `X9-PAY-7` among 500 records that mention
"payment". A search for "payment failure" with `require_literals=["X9-PAY-7"]`
returns that record first (MR-020.2), and every hit contains the phrase
(MR-020.1). Injection test: literals containing `"`, `*`, `:` and `NEAR` match
literally.

## Adapter change once it lands

Add an `exact` argument to `search`, mapped one-to-one. The `degradation` entry
for the vector arm comes from the engine.

## Open questions for the owner

1. Tokenizer-exact, or byte-exact, which would need a trigram index?
2. Should the vector arm decline, or run unfiltered with results intersected?
   Intersecting would be post-cap filtering, which the ADR forbids.
