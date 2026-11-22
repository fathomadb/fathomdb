# FathomDB Inspector

**Inspector 0.2.1 is released locally.** Record/tree, lexical,
stored-vector/L2, cache-only CPU hybrid, directed graph, provenance, retained
history and two-observation comparison are implemented on FathomDB 0.8.30
(schema 37) and qualified natively on Ubuntu 24.04 (`glibc 2.39`) against
closed databases produced by both published FathomDB 0.8.29 and published
FathomDB 0.8.30. The local distribution lives at `dist/inspector/0.2.1/` in the
dedicated `inspector/dev/0.2.1` release checkout (`dist/inspector/0.2.0/` is
preserved separately). No registry package or human-study validation is
claimed.

Inspector is a local browser GUI for FathomDB's records, text indexes,
vectors, and graphs. It connects these views using recorded identities and
origins so you can inspect the same data through its different representations.
A generic SQLite table browser does not cover this purpose.

## Implemented 0.2.1 capabilities

- Browse canonical records, attributes, edges, operational collections, and projection declarations.
- Read documents and structured values; follow exact record identities and retained revision relationships.
- Inspect indexed text, lexical requests, actual FTS/BM25 routes, and available match evidence.
- Inspect embeddings, readiness, measured similarity/distance, and the candidate-selection scope behind a neighborhood.
- Navigate a bounded interactive directed graph and inspect nodes, edges, and their content.
- Correlate observations across views without merging equal-content records or inventing missing provenance.
- Pin two bounded observations in session and compare identity membership, actual ranks and method-specific values with each original request/read context.

Initial support is one existing local database at the selected build's current
schema, with external writers stopped. Inspector may start/own the Engine and
its workers; normal startup/background work can change database files and
projection state. Inspector controls inspect data rather than edit it. Query
models must already be cached. Retained history/inactive data is viewable by
default; `--no-show-inactive` hides it. Still-retained erasure-pending content
requires `--show-erased`, which also permits its explicit local export.
Telemetry structure/counts omit payloads unless `--show-payloads` is supplied.
Applicable restrictions combine; no flag restores physically absent content.
See [disclosure rules](operations/index.md#observations-and-disclosure).

## Documentation

- [Getting started and availability](getting-started/index.md)
- [Concepts and identities](concepts/index.md)
- [Inspection views](guides/index.md)
- [Reference and supported scope](reference/index.md)
- [Operating boundaries](operations/index.md)
- [Contributing](contributing/index.md)
- [0.2.1 release notes and distribution](releases/0.2.1.md)
- [0.2.0 release notes (historical)](releases/0.2.0.md)
- [ROADMAP](ROADMAP.md)

Inspector has its own product versions; 0.2.1 is not FathomDB database 0.2
or a promise that a particular database release ships the application.

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
