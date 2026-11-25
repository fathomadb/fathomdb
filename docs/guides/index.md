# Guides

Guides cover the current public surface.

- [Working with structured search hits](structured-search-hits.md) — read the
  typed `id` (`IdSpace`), `kind`, `body`, `score`, `branch`, `source_id` and
  `ce_score` fields on each `search()` hit.
- [Hybrid search & filtering](hybrid-search-filtering.md) — RRF ranking and the
  closed `SearchFilter` metadata filter.
- [Graph retrieval and bounded expansion](graph-retrieval.md) — choose hybrid
  convenience paths or bounded lexical/explicit-seed expansion.
- [Retrieve by id](retrieve-by-id.md) — point lookups by `logical_id`
  (`read.get` / `read.get_many`) and paginated op-store read-back
  (`read.collection` / `read.mutations`).
- [Extractor provenance and vector kinds](extractor-provenance.md) — caller
  attribution, legacy extraction limits, and dense eligibility.
- [Frozen evidence](frozen-evidence.md) — choose frozen ranking, exact source
  evidence, or canonical pagination without weakening read authority.
- [Prompt context from search hits](prompt-context.md) — render ranked hits
  into prompt text within a character budget with `pack_context`.
- [MCP server (`fathomdb-mcp`)](mcp-server.md) — serve FathomDB over MCP stdio
  or embedded in Rust with default read-only posture, bounded frozen evidence,
  and opt-in per-store scoped writes.

See also [Erasure](../operations/erasure.md) for `purge` / `erase_source`.
