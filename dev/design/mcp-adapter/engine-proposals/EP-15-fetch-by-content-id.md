---
title: EP-15 — Point reads by content id (h:)
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. Let read.get_many resolve content-addressed h: ids, the hits that have no logical id, through a stored and indexed body hash. Also records that p: ids are not stored records, which corrects MR-042.2.
---

# EP-15 — Point reads by content id (`h:`)

Status: Draft. Priority P2. Not an accepted decision.

## Problem

- Search hit ids are an `IdSpace`: `l:` (logical id), `h:` (SHA-256 of the body,
  for rows without a logical id) and `p:`
  (`fathomdb-engine/src/identity.rs:78-95`). `derive_stable_id` computes `h:` as
  the hex SHA-256 of the body (`identity.rs:475-486`).
- `read.get` and `read.get_many` take logical ids only
  (`fathomdb-sdk/src/read.rs:20-37`). The SQL matches `logical_id IN (...)`
  (`fathomdb-engine/src/read.rs:101-106`).
- `canonical_nodes` has no stored body hash. I found no `body_hash` or
  `content_hash` column in the schema. So an `h:` lookup would have to hash
  every body.
- Correction to `requirements.md` MR-042.2: `p:` is "a synthetic
  `rerank_passages` hit keyed by the caller-supplied passage ordinal"
  (`identity.rs:85-86`). It is not a stored record, and there is nothing to
  fetch. Outside tests, `IdSpace::passage` is built only by the reranker
  (`rerank.rs:139`) and the graph-evidence decoder (`graph_expand/codec.rs:570`),
  so store search does not produce `p:` hits.

Adapter workaround removed: `fetch` and `expand` reject non-`l:` refs per item
with `unsupported_ref` (`src/rust/crates/fathomdb-mcp/src/tools.rs:861-869`,
`tools.rs:993-999`). The dominant corpus hit class (`h:`, per the IdSpace doc)
therefore cannot be fetched in full.

## Proposed surface

- An additive `body_sha256 TEXT` column on `canonical_nodes`, written at insert
  time with `canonical_body_hash` (`write_commit.rs:116-120`), and indexed. The
  write path computes that hash today only for provenanced source writes
  (`write_commit.rs:216`). Ordinary writes would add one SHA-256 per node. Under
  `ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md` only fresh
  databases exist, so there is no backfill.
- `read::get_many_by_id(engine, ids: &[IdSpace], view)` returns
  `Vec<Option<NodeRecord>>`. `l:` resolves as today, `h:` resolves by hash, and
  `p:` is `InvalidArgument`.
- Several active rows can share one body, and so one `h:` id. The read then
  returns the highest `write_cursor`, the same tie rule as `include_superseded`
  point reads (`read.rs:98-101`).

## Invariants and consistency

- `h:` resolution is deterministic under one view and snapshot.
- The hash is of the stored body bytes, matching `derive_stable_id`. Bodies are
  never rewritten in place, I believe, but this was not checked across every
  write path.
- The same column could serve EP-10's exact-duplicate rule (MR-062).

## Interaction with ADRs

- `ADR-0.8.0-canonical-identity-substrate.md` and the C-2 id-space decision own
  `IdSpace`. This adds a resolver, not a new id space.
- `ADR-0.8.27-rust-sdk-parity.md`: the new read lands in all three SDKs.

## Cost and performance

64 hex characters and one index entry per node, plus one SHA-256 per
non-provenanced node write. Lookup is an index probe.

## Test plan sketch

Real database: write two anonymous records and one duplicate body. `h:` of the
first resolves to it. `h:` of the duplicated body resolves to the newer row. `p:`
is rejected. An `h:` id from a search hit round-trips through the new read to
the same body.

## Adapter change once it lands

`fetch` accepts `h:` refs (MR-042.2). `expand` stays `l:`-only, because graph
seeds are logical ids. The interface lists `p:` as never produced.

## Open questions for the owner

1. Should a shared body under one `h:` id return all matching rows, not just the
   newest?
2. Should edges get the same column? Edge-fact hits are `h:` too.
