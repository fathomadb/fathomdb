# FathomDB Inspector

**Inspector targets FathomDB 0.8.32 (schema 39, refusing schema 38 and older).**
Record/tree, outside-`body` node/edge `properties`, active-`scope` switching
and per-view scope filters strictly subordinate to `D-01` nondisclosure,
record-keyed lexical, stored-vector/L2, cache-only CPU hybrid, current and
historical `as_of` directed graph, provenance, retained history, and
two-observation comparison are implemented for FathomDB 0.8.32 (schema 39,
`TABLE_MANIFEST`-backed admission) and qualified natively on Ubuntu 24.04
(`glibc 2.39`) with Chromium and bundled SQLite (`D4 = B`). Release archives
(`fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` and
`fathomdb-inspector-0.3.0-linux-x86_64.tar.gz.sha256`) are built by the
`build-inspector` job in `.github/workflows/release.yml` for GitHub Release
attachment and can also be sealed and verified locally via `local-release.py`.
No human-study validation is claimed.

Inspector is a local browser GUI for FathomDB's records, text indexes,
vectors, and graphs. It connects these views using recorded identities and
origins so you can inspect the same data through its different representations.
A generic SQLite table browser does not cover this purpose.

## Implemented 0.3.0 capabilities

- Browse canonical records, outside-`body` node and edge `properties`, active-`scope` switcher and per-view scope filters (uncluttered on single-scope `""` stores; strictly subordinate to `D-01` nondisclosure), attributes, edges, operational collections, and projection declarations (including both node and edge filterable property metadata).
- Read plain-text or JSON `body` documents and expandable structured-value trees for both `body` and `properties`; copy exact values preserving 64-bit integer precision outside `2^53`, `-0`, empty strings, and `SQL NULL`; follow exact record identities and retained revision relationships.
- Inspect indexed text, lexical requests, actual FTS/BM25 routes, and available match evidence.
- Inspect embeddings, readiness, measured similarity/distance, and the candidate-selection scope behind a neighborhood.
- Navigate a bounded interactive directed graph across current state or historical `as_of` snapshots (`as_of_write_cursor` or `as_of_epoch_ms`, mutually exclusive) in both stored-adjacency and native engine modes, and inspect nodes, edges, and their content.
- Correlate observations across views without merging equal-content records or inventing missing provenance.
- Pin two bounded observations in session (including distinct `as_of` graph snapshots) and compare identity membership, actual ranks and method-specific values with each original request/read context.

Initial support is one existing local database at the selected build's current
schema (schema 39 for FathomDB 0.8.32; schema 38 and older refuse cleanly),
with external writers stopped.
Inspector may start/own the Engine and its workers; normal startup/background
work can change database files and projection state. Inspector controls inspect
data rather than edit it. Query models must already be cached. Retained
history/inactive data is viewable by default; `--no-show-inactive` hides it
(including in historical `as_of` graph queries). Still-retained erasure-pending
content requires `--show-erased`, which also permits its explicit local export;
when withheld, `body`, `properties`, `properties_bytes`, and `typeof(properties)`
are all omitted. Telemetry structure/counts omit payloads unless
`--show-payloads` is supplied. Applicable restrictions combine; no flag
restores physically absent content.
See [disclosure rules](operations/index.md#observations-and-disclosure).

## Documentation

- [Getting started and availability](getting-started/index.md)
- [Concepts and identities](concepts/index.md)
- [Inspection views](guides/index.md)
- [Reference and supported scope](reference/index.md)
- [Operating boundaries](operations/index.md)
- [Contributing](contributing/index.md)
- [0.3.0 release notes and distribution](releases/0.3.0.md)
- [0.2.1 release notes (historical)](releases/0.2.1.md)
- [0.2.0 release notes (historical)](releases/0.2.0.md)
- [ROADMAP](ROADMAP.md)

Inspector has its own product versions; 0.3.0 is independent of FathomDB's
Engine/SDK version numbering.

The owner clarified on 2026-10-31 that retained-erasure display/export applies
**only if legitimate readable content exists**. Physical deletion remains
correct and unchanged. Autonomous physical deletion leaves no readable target.
Existing ExplicitMaintenance databases may retain a queued exact target scope;
`--show-erased` permits that content and narrow export while other gates still
apply. Inspector honors the stored policy and does not queue requests, activate
maintenance, retain content, delay erasure or reconstruct erased bytes.
Positive Engine end-to-end export is required only for a qualified supported
Engine that legitimately exposes this state; other native-view positives remain
required. This does not expand general export scope.
