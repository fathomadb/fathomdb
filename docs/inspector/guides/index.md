# Inspection views

Inspector 0.3.0 implements these views for FathomDB 0.8.31 (schema 38). They
coordinate exact record identity and separately observed context; physical
representations may contain copied content. Installed technical checks cover
the selected `D4 = B` Ubuntu 24.04 pairing; the unperformed five-person study
remains removed by owner decision.

| View | What it presents | Typical operation |
| --- | --- | --- |
| Structure and contents | FathomDB nodes, edges, kinds/roles, reserved `scope`, operational collections, projections (with `entity_class` / `vector_kinds`), and scoped counts. | Choose a category and browse a bounded page. |
| Record detail | Readable plain-text or JSON `body`, expandable outside-`body` `properties` tree and exact copy (preserving 64-bit integer lexemes outside `2^53`), read-only reserved `scope`, attributes, exact identity/revision, and recorded metadata. | Open complete `body` or `properties`, copy exact values, and follow a verified relationship. |
| FTS/BM25 | Indexed content and origins, node-eligible declared property choices, actual request route, ranked matches, and available lexical evidence. | Run a text request and navigate from a result to its record. |
| Vectors | Profile/dimension/representation facts, custom enrolled vector kinds, readiness, and a metric-faithful similarity neighborhood. | Inspect measured distances and candidate-selection scope, then open an origin. |
| Graph | Directed nodes/edges, kinds, selectable detail (including inline node/edge `properties` when permitted), current or historical `as_of` snapshot selector (`as_of_write_cursor` or `as_of_epoch_ms`), and bounded expansion/collapse. | Expand a current or historical `as_of` neighborhood (stored or native) and inspect an edge's own content and properties. |
| Associations and results | Verified record-to-representation links (including node and edge `canonical_property_values`) and retained request observations. | Pin two bounded observations (including distinct `as_of` graph snapshots); compare typed membership and actual per-method rank/metric beside original requests/contexts. |

The directed SVG graph has a synchronized relationship list for exact values
and keyboard selection, expansion and collapse. In Schema 38, the Graph view
also offers a `Snapshot` selector (`Current`, `At write cursor`, or `At commit
time (ms)`) shared across stored-adjacency and native engine traversal; inputs
are validated locally before dispatch so typos do not evict cached observations,
and `Back` navigation and comparison pins preserve the exact `as_of` request
context. Local zoom, fit, scrolling and selection controls keep large bounded
neighborhoods navigable. Partial results retain explicit expansion owners;
collapsing a branch preserves independently expanded branches. Graph layout
distance is not vector similarity or relationship strength. A vector comparison
uses labeled measured values; any optional reduced-dimension map would be a
separately labeled display transformation.

Browse and detail use bounded stored title/name previews (falling back to
permitted `properties` when `body` is plain text) beside exact identity.
A missing display field, an oversized uninspected body and an exhausted preview
budget have different fallback labels. All user and database strings across
`body`, `properties`, `scope`, and graph labels render purely via DOM
`.textContent` (inert against HTML/SVG/script payloads). Named-property text
inputs offer actual eligible node declarations; invalid input appears beside
the request. Result summaries retain the original method, request and scope,
with later policy/origin checks labeled by their own source and time. Recognized
timestamps show UTC alongside the exact epoch; unknown values remain unknown.
Full raw evidence and exact copy remain available in locally contained regions.

Comparison membership counts typed native result entries. A separate verified
canonical-correspondence section identifies uniquely established shared exact
record revisions. Different native entries keep their own ranks/scores even
when that section links them to the same origin. Zero typed intersection does
not establish zero shared origins; ambiguous or unknown origins are not joined.

## Example investigation

Start with a readable record. Follow its verified text/vector associations,
inspect readiness, run a supported retrieval operation, and inspect graph
participation for the same record. Returning to the earlier view preserves
selection and request context. A missing association remains unknown/unavailable, never guessed. Comparison
keeps distinct revisions and ambiguous origins visible; capped/policy-filtered,
unsupported and unknown cases do not become absent rank zero or a proven cause.
Pinning does not rerun a request, persist an experiment or permit general export.

The UI exposes filters, limits, sampling, incomplete work and unknown facts.
It does not manufacture a cause for an absent result or infer an origin from
similar text. Apply [session disclosure rules](../operations/index.md#observations-and-disclosure)
to every view, link, copy and export. Explicit local export of still-retained
erasure-pending content is required under `--show-erased` and all other
applicable gates. Broader saved investigations and general export remain
roadmap work.

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
