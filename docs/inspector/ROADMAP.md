# Inspector ROADMAP

## Current 0.3.0 release line (FathomDB 0.8.31 / Schema 38)

Inspector `0.3.0` upgrades Inspector onto FathomDB `0.8.31` (schema `38`) and
carries forward local attachment, complete record/tree inspection, native
record-keyed FTS/BM25, stored vectors/L2, cache-only CPU hybrid, interactive
directed graph, whole-body and partial UTF-8 source/dependency/history
inspection, and in-session comparison of two actual observations. It adds:

- **Schema `38` admission & reserved `scope`:** Strict Schema `38` admission
  (`compiled_schema = 38`), refusing Schema `37` (`0.8.30`) cleanly, with
  read-only `scope` visibility (empty in `0.8.31`; multi-tenant scope semantics
  belong to `0.8.32`).
- **Outside-`body` node & edge `properties`:** Expandable structured-value
  inspection (`detail_field`, `chunk`, `tree`), exact copy preserving 64-bit
  integer precision outside `2^53`, `-0`, empty strings, and `SQL NULL`,
  strict `.textContent`-only XSS protection, 64 MiB memory admission parity,
  node/edge `canonical_property_values` associations, and complete 8-flag
  `D-01` erasure non-disclosure.
- **Historical `as_of` graph exploration:** Mutually exclusive
  `as_of_write_cursor` / `as_of_epoch_ms` snapshots in both stored-adjacency
  and native engine graph traversals, preserved across `Back` and comparison
  pins and strictly subordinate to all 8 `D-01` disclosure flags.
- **Scoped local CI, bounded timeouts & GitHub Release delivery:** `--scoped`
  default in `local-ci.sh` (with fail-closed diff guard and `--full-workspace`
  / `--installed-only` modes), bounded subprocess/browser teardown timeouts,
  retained `D4 = B` Ubuntu 24.04 (`glibc 2.39`) Chromium + bundled SQLite
  qualification, and `.github/workflows/release.yml` `build-inspector` GitHub
  Release archive attachment (`fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` +
  `.tar.gz.sha256`).

The owner removed the unperformed five-person study, with no human-validation
claim.

The release preserves all assessed necessary corrections and recommended
improvements from `0.2.x`: incomplete provenance and system-data labeling,
class-correct lifecycle, actual lexical and vector attribution, cache-only
query runtimes, current-schema admission, non-destructive refusal guidance,
shared-mechanism reuse assessment, and automated and actual installed browser
interaction checks.

Initial attachment is local with external writers stopped; Inspector may own
and start the Engine. Normal Engine lifecycle effects are allowed and
disclosed; an immutable forensic view is not promised. The disclosure policy
and exact flags are adopted in
[operating boundaries](operations/index.md#observations-and-disclosure),
including narrow local export of still-retained erasure-pending content under
`--show-erased` and other applicable gates. The selected installed pairing is
`D4 = B` Ubuntu 24.04 / glibc 2.39 userland with kernel `7.0.0-43-generic`
(Landlock ABI 8) and Chromium `156.0.8078.4` (bundled SQLite, zero
GTK4/WebKitGTK or Docker). The native binary requires at most GLIBC 2.39
(Ubuntu 22.04 / glibc 2.35 is not supported); this does not qualify an
unbounded newer-platform matrix. Inspector versions are independent of the
FathomDB database release numbering.

`0.3.0` excludes Engine/SDK/MCP/schema/ranker changes, multi-tenant `scope`
filtering (`0.8.32`), editing/admin/repair, remote/multiuser or extra
platforms, live-app attachment and historical-schema readers. Saved
queries/workspaces, general export, richer facets/statistics, advanced vector
maps/ANN/relevance experiments and graph analytics remain later candidates. No
tutorials, model training, GPU acceleration or plugin framework is added.

`0.3.0` qualifies the selected Engine's supported single head-window vectors.
Multiple emitted windows from one source revision are deferred; exact
metadata/origins, metric and cached CPU semantic/hybrid remain required.

## Later candidates — no assigned release dates

| Group | Candidate capabilities |
| --- | --- |
| Distribution | Additional OS package channels, signed public binaries, or an Inspector-only crates.io source package beyond the `build-inspector` Linux x86_64 GitHub Release tarball. |
| Deeper vector inspection | Compare binary candidate selection with float-neighborhood results side by side. |
| Vector source windows | Multiple emitted windows from one source revision, only after a separately scoped producer/admission contract and ordinary-write positive are supported and exact spans/origins and bounded retrieval/inspection are independently qualified; no assigned release date. |
| Deeper lexical inspection | Inspect inactive or historical lexical structures beyond supported active routes. |
| Summaries and analysis | Role-distribution dashboards, broader field/distribution discovery, and bounded anomaly analysis. |
| Compatibility | Older-schema read adapters and historical-file qualification. |
| Availability | Initially WAL-present or forensic immutable inspection and, separately, attachment to an already-running application. |
| Investigation workflows | Saved investigations, reproducible sessions, and general export/share beyond the narrow erasure-pending export. |
| Optional presentation | Explicitly labeled vector dimension or sampled reduced-dimension views. |
| Implementation consolidation | Further snippet/shared-session reuse where the carried reuse assessment demonstrates value. |

These candidates are preserved, not promised. They enter a future release only
with defined scope and acceptance. In particular, an optional vector map does
not replace a metric-faithful neighborhood, and future extensibility does not
make the required graph and lexical interfaces optional in 0.2.0.

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
