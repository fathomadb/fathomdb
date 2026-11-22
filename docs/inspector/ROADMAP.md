# Inspector ROADMAP

## Current 0.2.1 local release

The locally released 0.2.1 package upgrades Inspector onto FathomDB 0.8.30
(schema 37) and implements local attachment, complete record/tree inspection,
native record-keyed FTS/BM25, stored vectors/L2, cache-only CPU hybrid (Candle
0.13.0), interactive directed graph, whole-body and partial UTF-8
source/dependency/history inspection and in-session comparison of two actual
observations. Typed membership, original requests/read contexts, actual ranks
and method-specific values stay distinct. No automatic rerun, persisted
experiment or shared score scale is implied. Native Ubuntu 24.04 qualification
and dual published FathomDB 0.8.29 / 0.8.30 checks are complete; Slice 30
promoted the qualified bytes atomically to `dist/inspector/0.2.1/`. The owner
removed the unperformed five-person study, with no human-validation claim.

The release preserves all assessed necessary corrections and recommended
improvements from 0.2.0: incomplete provenance and system-data labeling,
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
Ubuntu 24.04 / glibc 2.39 userland with kernel `7.0.0-43-generic` (Landlock ABI
8) and Chromium `156.0.8078.4`. The native binary requires at most GLIBC 2.39
(Ubuntu 22.04 / glibc 2.35 is dropped); this does not qualify an unbounded
newer-platform matrix. Inspector versions are independent of the FathomDB
database release numbering.

0.2.1 excludes Engine/SDK/MCP/schema/ranker changes, editing/admin/repair,
remote/multiuser or extra platforms, live-app attachment and historical readers.
Saved queries/workspaces, general export, richer facets/statistics, advanced
vector maps/ANN/relevance experiments and graph analytics remain later
candidates. No tutorials, model training, GPU acceleration or plugin framework
is added. The local release is available at `dist/inspector/0.2.1/` in the
release checkout; no registry package is available.

0.2.1 qualifies the selected Engine's supported single head-window vectors.
Multiple emitted windows from one source revision are deferred; exact
metadata/origins, metric and cached CPU semantic/hybrid remain required.

## Later candidates — no assigned release dates

| Group | Candidate capabilities |
| --- | --- |
| Distribution | Public archive downloads or registries after an Inspector-specific version/signing/CI/credential plan; 0.2.1 remains local-only. |
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
