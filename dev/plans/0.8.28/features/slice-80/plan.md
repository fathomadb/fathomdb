---
title: FathomDB 0.8.28 Slice 80 — search view flags
status: approved for implementation; Astra reviewed
target_release: 0.8.28
---

# Slice 80 plan

Baseline: `c662fc89ba1603829385c55c2bfe967d1bffd359`, existing
`dev/0.8.28` checkout, one writer. No new worktree or branch.

## Reconciliation of the draft

1. The master plan allocates SEARCH-VIEW-FLAGS here; no standalone Slice 80
   plan existed. Its later historical-completeness obligation is accepted:
   retaining already embedded edges alone cannot satisfy historical search.
2. Slice 50 provides bounded pending scans and physical completion checks.
   Nodes already include superseded versions in dense membership. Edges
   exclude them in scheduling, completion, rebuild and in-flight commit.
   Both edge supersession paths delete vectors and write synthetic
   `up_to_date` terminals. All these paths must agree after this change.
3. Slice 60 supplies independent vector identity and spans; Slice 70 adds
   lifecycle metadata and native flag combinations before the 192-row cap.
   Reuse those mechanisms and update every mapped vector on supersession.
4. Search still rejects existence flags at several entry points and embeds
   strict predicates in text, dense hydration, graph and property paths.
   Update all serving paths, including frozen, evidence and paginated search;
   retain existing filter, validity, dependency and limit semantics.
5. Python and TypeScript already transport ReadView. No new public parameter
   is needed. Replace obsolete refusal tests and contract text with positive
   parity assertions; retain default stale-body regression expectations.
6. D28-14 filtered direct body FTS belongs to Slice 90. MCP belongs to 0.8.29.
   No chunk generation, new durable index, schema migration, ranking algorithm change, or
   performance sweep is needed. Existing completion/fallback mechanisms
   should be extended, not replaced with a separate historical subsystem.

## Requirements and acceptance

- R80-1: Rust, Python and TypeScript search accept each existence flag and
  both together. Each flag relaxes only its own exclusion. Default searches
  still exclude inactive and superseded rows; validity and filters compose.
- R80-2: node/edge body text, dense hydration, property and graph paths honor
  the view. Frozen, evidence and paginated paths preserve authentication,
  snapshot validity, deterministic ordering and existing result limits.
- R80-3: superseded edge vectors and spans survive both logical-ID and
  endpoint/kind supersession; lifecycle metadata updates transactionally.
  Nodes and eligible body-bearing edges remain dense members when superseded
  before drain, after drain or during embedding. Pending/failed history must
  not be reported complete merely because supersession occurred.
- R80-4: reopen reconciles previously skipped historical edges with empty
  physical projections, removes false completion terminals and rewinds work
  as needed. Rebuild includes historical members. Partial/corrupt tuples
  remain errors; genuine failures are not silently retried. Dependency
  closures and hard erasure remain authoritative.
- R80-5: unavailable historical dense projections produce an explicit
  fallback/completion outcome; no partial vector history is advertised as
  complete. Temporal views retain the existing edge-expiry contract.
- R80-6: real SQLite tests cover the flag matrix, both edge supersession keys,
  timing cases, reopen/rebuild, failure/unavailable projections, binding
  parity and default stale-body protections. Existing bounded-work and
  physical-completion tests remain valid or are revised only where the new
  historical-membership contract deliberately supersedes the old behavior.

## Implementation and verification

Astra reviews [design](design.md) before implementation. Commit observed
failing tests before each behavior group, then implement GREEN: search view
admission and serving; historical projection membership/retention; boot and
rebuild reconciliation; binding parity. Tests whose old contract is explicitly
superseded are updated as requirements changes, never to hide a failure.
Use scoped tests while developing, then Astra code review and independent
subagent verification. Run the full agent gate once on final source; repair
specific findings with scoped checks. Use `gh` only for CI inspection/actions.
Record actual evidence and remaining limits in status, update release state
and generated views, fast-forward the existing release branch and push both
explicit branch refs. No publication or new worktree cleanup is needed.
