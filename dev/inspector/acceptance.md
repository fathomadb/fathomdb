---
title: Inspector acceptance criteria
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.2.0
desc: Stable Inspector criteria, fixtures, and v0.2.0 action signals
status: ACTIVE
---

# Inspector acceptance criteria

Current delivery scope is [owner-adopted 0.2.0](plans/0.2.0/scope.md), amended by
[INS-020-LOCAL-RELEASE](plans/0.2.0/release-decisions.md). Functional INS/V01/X,
T-01–T-05 and High investigation goals remain required. The five-person study
was removed by owner decision; no human sessions occurred or count as passed.
Slices20–60/70 technical qualification and75 UX remediation are complete;
80 completed readiness and[90 released locally](plans/0.2.0/slice-90/status.md)
to ignored `dist/inspector/0.2.0/`. Historical
source/design evidence retains its actual version and execution scope.

## Acceptance criteria

Criteria describe future behavior, not executed tests. All INS-01 through
INS-09, INS-X01–X11 conditions, and V01 signals below are required for v0.2.0. Local IDs remain stable
for traceability. Preconditions are the supported attachment/model envelope in
requirements.md and the named fixture; actions are the user/API operations
in each criterion. The verification plan supplies independent expected facts,
negative boundaries, evidence and owners. Every list/detail/result criterion
also exercises INS-X02–X05 and INS-X07–X11 rather than assuming those pass. Fixtures are specified below; UI layout is not the oracle.

### INS-01 — database organization

- **INS-01.1:** The structure view separately identifies canonical nodes, canonical edges, operational collections, and projections. Populated categories lead to actual contents; empty, unconfigured, and unavailable categories are distinguishable.
- **INS-01.2:** Canonical kind and structural role, operational collection, and projection name appear in distinct labeled fields or groups. The same facts use consistent terminology across views; node state/window and edge temporal/dependency axes remain class-specific.
- **INS-01.3:** Fixture F-01 presents a source canonical record, a derived canonical record, and a projection as distinct objects with the correct recorded links. The derived record is not nested or counted as though it were merely a projection of the source.
- **INS-01.4:** Operation requires no embedded tutorial or generated explanation. A user may consult the associated product documentation independently.

### INS-02 — database contents

- **INS-02.1:** The inventory identifies every populated category in F-01 and opens bounded contents without code or SQL.
- **INS-02.2:** Counts distinguish logical identities where defined, stored canonical revisions, edges, operational current values, mutation entries, and derived projection entries. Counts specify read scope, exact/estimated status, and any incompleteness; they are not summed into an ambiguous records total.
- **INS-02.3:** Browsing exposes actual fields and values. An observed-field overview states its category, filter, sample size, and whether it reflects registered schema or sampled contents; it does not claim that an unobserved field is absent everywhere.
- **INS-02.4:** Database identity, location, schema compatibility, and attachment status are visible. Missing, locked, corrupt, unsupported, and empty databases are distinguishable outcomes. Durable attachment status follows accepted session state independently of transient request feedback; successful reattachment clears prior attach errors, and superseded replies cannot restore them.
- **INS-02.5:** In F-03, append-only collections expose retained entries and entry identities; latest-state collections expose current values. Collection mode is visible. Overwritten latest-state payloads are not presented as retained history. Equal keys in separate collections do not imply a relationship without explicit recorded evidence.

### INS-03 — browse, locate, and inspect

- **INS-03.1:** Users can browse by populated kind or collection and locate F-01 records using exposed identities and supported text, attribute, or source constraints without first knowing a record ID. Constraints unavailable for a category are identified before execution. Browse/detail offer admitted bounded stored title/name previews with an explicit fallback, alongside unchanged exact identity; oversized bodies are not fully parsed or preloaded for labels.
- **INS-03.2:** Detail includes complete stored content, identity, class, kind, available attributes, and recorded source metadata. Missing, null, empty, truncated, and unavailable values are distinguishable; full content is accessible through bounded detail reads.
- **INS-03.3:** Edge detail exposes endpoints separately from edge properties. An unavailable endpoint is not replaced with a similarly named record.
- **INS-03.4:** Following a link and returning preserves filter, ordering, page position, selected item, expanded structured values, content-chunk position and initiating keyboard focus within the session; evicted/invalidated observations require explicit refresh.
- **INS-03.5:** Distinct fixture records containing identical text remain independently inspectable. Colliding bare values in different identity namespaces or object classes do not resolve to each other.

### INS-04 — retrieval representations

- **INS-04.1:** Record detail lists the applicable projection declarations separately from materializations verified for that record/revision. Declaration alone does not produce a ready indicator.
- **INS-04.2:** A materialization with recorded origin evidence links back to its originating record/revision. A scalar materialization has an actionable exact canonical-origin control while retaining its own representation identity. Missing linkage appears as unknown or unresolved, never inferred from equal text.
- **INS-04.3:** Retrieval controls identify their supported inputs, active filters, and selected method. Direct lookup, text search, hybrid retrieval, and graph traversal remain distinct operations; unavailable modes are identified before execution. Named-property choices come from complete eligible declared facts, including the default tokenizer; invalid/stale input has actionable nearby feedback and successful retry clears it while old results retain their own request.
- **INS-04.4:** F-04 separately displays unconfigured, pending, failed, ready, unavailable, and unknown states where supported by evidence. Global status is not displayed as a record-specific guarantee.

### INS-05 — identity across results

- **INS-05.1:** In F-01, results resolving to one canonical revision open the same revision detail. Namespace, value, database, and record class participate in resolution; text similarity does not.
- **INS-05.2:** F-02 includes distinct identities with equal text, two revisions of one logical identity, and an ambiguous content result. Distinct records remain distinct; related revisions are labeled; ambiguity is retained.
- **INS-05.3:** Opening a result preserves its observed content and read context subject to the adopted disclosure gates; newly withheld prior content is labeled refused/unavailable without substituting another payload. Later detail is shown separately if it represents another observation or revision. Unavailable original content is stated rather than silently replaced.
- **INS-05.4:** Each result set retains the exact request, mode, filters, effective options, limits, observation context, and available execution evidence. A representative branch label is not expanded into a claim about every contributing branch.
- **INS-05.5:** Scores identify their method and meaning. Different score families are not visually compared on a shared scale without a defined conversion. No unreturned candidate is presented as having a known score or rank.

- **INS-05.7:** Pin two bounded actual observations in F-07 without re-execution. Each side retains typed database/class/namespace/value/revision, query or seed, effective options/filters/limits, population and read context. Membership, available actual rank and each method's own metric remain separately labeled; graph sets have membership without a fabricated rank or value.
- **INS-05.8:** Independently known overlapping/disjoint results, equal text, class/namespace collisions, revision differences and zero/one/multiple origins remain distinct in comparison. Unknown/ambiguous origins are not coerced into shared canonical identity. Different observation contexts are visible and never claimed as one snapshot without evidence; Back restores each original context. Membership explicitly compares typed native entries. Separately show permitted uniquely verified same-exact-canonical-revision correspondences; zero typed intersection does not imply zero shared origins. Missing, multiple, unknown or conflicting origin facts do not create correspondence. A representative retrieval arm cannot split one typed physical result identity; distinct physical vector entries sharing an origin remain distinct.
- **INS-05.9:** Compare bounded/capped, policy-filtered, unsupported and unknown cases without assigning absent rank zero or fabricating omission causes. Only independently established scope facts are asserted, and restricted identities/content remain withheld under D-01 rather than disclosed through membership or labels.
- **INS-05.10:** Pin/replace/unpin, stale/refused/expired and budget-evicted observations obey existing observation/byte/queue/cancel limits and all disclosure gates. Announce eviction and preserve only permitted original locator/context; late results never silently replace pinned evidence. Positive and failure cases exercise backend and keyboard/pointer browser behavior. A source-only eviction keeps an independently admitted result usable with its original context and safe focus; unpin never disables current-policy checks of an open original view. No automatic rerun, persisted experiment or general export is introduced.

### INS-06 — storage and visibility

- **INS-06.1:** F-04 exposes stored state, eligible/ineligible state under a specified view where known, projection readiness, and presence/absence in a particular bounded result set as separate values.
- **INS-06.2:** The interface never labels absence from results as missing from storage or as a proven eligibility/projection failure. Unobserved facts use unknown, not false.
- **INS-06.3:** Database configuration, record-specific materialization state, and session runtime availability are separately scoped and time-stamped. A successful scalar/document inspection is labeled representation-read readiness, not retrieval readiness or eligibility; each fact identifies its source and observation.
- **INS-06.4:** Core usability task T-05 assesses the distinction between these facts; detailed missing-result diagnosis under INS-08 is not required to pass the core evaluation.

### INS-07 — provenance and revisions

- **INS-07.1:** Retained links in F-01/F-02 support navigation between source revisions, derived canonical records, dependencies, and revisions of one logical identity.
- **INS-07.2:** Relationship labels distinguish source attribution, registered dependency, graph edge, projection origin, and supersession.
- **INS-07.3:** A source reference without retained content remains a reference. A missing target is unavailable; erased information is not reconstructed. F-04 includes both unavailable and erased targets without requiring disclosure of erased identities.

### INS-08 — outcome evidence

- **INS-08.1:** Available per-hit inclusion, representative origin, ranks, or ranking components are shown with their originating request and evidence scope.
- **INS-08.2:** Record-specific eligibility or readiness checks state their view, time, and source of evidence. A check performed after retrieval is not presented as the state at retrieval time. Compact summaries identify the original request/method/filter/view/limit and each check's own evidence source/time near the result. Recognized epochs have safe UTC formatting alongside exact raw values; unknown units or invalid/out-of-range values remain unformatted.
- **INS-08.3:** If the backend cannot establish the cause of a missing result, the outcome remains undetermined; generic possibilities are not displayed as observed causes.

### Additional exactness and evidence cases

- **INS-05.6:** F-02 tests zero, one and multiple verified origins for a result/representation. Only a uniquely resolved exact revision opens as that revision; multiple origins are presented as choices with cardinality, and a passage/content identity is never coerced into a logical record locator. Session/class/namespace mismatches and expired references fail before reading.
- **INS-06.5:** F-04 tests node active/inactive state, supersession and half-open validity at before/equal/after bounds independently from edge invalid-time and dependency closure at the same specified instant. Edges have no fabricated node state; `t_valid` is not assumed to impose a filter where the selected Engine route does not. Recorded provenance role/completeness stays visible on ordinary current writes; absent links remain unknown.
- **INS-08.4:** Every supported lexical operation reports effective route/indexed fields/tokenizer/scoring and every vector operation reports representation, metric/direction, measured population, selection stages and relevant centering. A recorded tokenizer override does not prove effective use. Metadata absent from a selected backend remains explicitly unknown; required positive route/metric capabilities cannot be waived as unavailable.

### INS-09 — structure-appropriate views

- **INS-09.1:** F-07 opens a canonical record as readable text or a structured-value tree as appropriate, with system metadata and identity separate from payload. A spreadsheet row is not the only available detail form.
- **INS-09.2:** An FTS/BM25 view exposes indexed content and origin, executes a lexical request, and presents ordered passage/document results with actual match evidence and method-specific scores. Body/edge RRF, named-property negated BM25 and explicitly qualified native BM25F remain separate; incompatible BM25F filter/view controls refuse before execution. Original payload windows state byte count/completeness, native snippets retain byte ranges, and later indexed/origin reads state their independent context. Missing spans remain unavailable; required positive routes cannot be waived. Keyboard query/origin/Back preserves the original request and observation.
- **INS-09.3:** A vector view exposes profile/model identity, dimensionality, relevant stored representation metadata, origin, and readiness. A bounded similarity-neighborhood request presents actual metric/distance values and linked records using a metric-appropriate visual comparison. Hybrid fused rank is not substituted for vector distance.
- **INS-09.4:** A graph view presents a bounded interactive node/edge diagram with direction, edge kind, node selection, edge detail, and expansion/collapse from a selected node. The displayed scope and truncation are visible. Layout proximity is not presented as stored relation strength or vector similarity. A relationship table may accompany the diagram but cannot replace it. Partial hub expansion retains the selected exact expansion owner and permits collapse without dropping independently owned/shared branches. A stable bounded viewport offers legible local navigation and synchronized exact selection; scope stays beside the diagram.
- **INS-09.5:** A representation-association view links a selected canonical record/revision to its verified text/vector representations and graph participation. It distinguishes stored associations from query-time contribution; graph edges are canonical relationships, not automatically a generated projection of every record.
- **INS-09.6:** Selection and navigation preserve exact identity and observation across views. F-07 demonstrates moving from source content to lexical and vector representations, to a related graph neighborhood, and back without merging equal-content records or discarding query context.
- **INS-09.7:** Visual encodings have labeled values and keyboard-accessible inspection of the same facts. An optional vector dimension plot or dimensionality-reduction map is labeled as a display transformation and does not establish semantic meaning or exact distance in the original space.

### Cross-cutting conditions

- **INS-X01 — inspection operations and Engine lifecycle:** Inspector requests expose no edit, actuation, projection-configuration, repair, migration or bootstrap operation. Engine startup is allowed, including its qualified ordinary startup/background/shutdown effects; the UI identifies this mode and possible changes before opening. F-05 verifies the independently specified allowed effect classes and intended pre/post facts, using an equivalent Engine lifecycle for attribution, without byte-preservation claims on successful startup. Preflight refusals before Engine open preserve files; failures after Engine startup report that effects may have occurred. No implicit model download/cache repair or extra rebuild/drain/repair is introduced by Inspector. F-05/F-07 hostile-parent-environment cases verify selected child configuration: experimental dependency PRAGMAs cannot change independent persistent header sentinels, lexical/hybrid candidates match the selected ordinary configuration, and cache/network/log invariants remain intact.
- **INS-X02 — scope:** Every list, count, sample, and traversal exposes its filters, ordering, limits, continuation or truncation, and unavailable data. Sampling never silently stands for a complete scan.
- **INS-X03 — bounded work:** F-06 tests explicit row/byte/work limits and cancellation for lists, payloads, counts, text/vector retrieval, and graph expansion. Cancelled or budget-exhausted operations remain distinguishable from complete empty results.
- **INS-X04 — observation:** Record references and results carry the attachment/read context. Engine background progress is a new observation, not an identity invalidation by itself. Comparisons across separate observations do not imply a shared snapshot. Refresh does not silently retarget an existing result; exact revision resolution never substitutes current content.
- **INS-X05 — faithful values:** Copying an identity or value preserves its exact underlying value; display formatting is not copied as though stored. Large values use bounded navigation rather than silent loss.

- **INS-X06 — disclosure:** Enforce the owner-adopted D-01 defaults and exact `--no-show-inactive`, `--show-erased` and `--show-payloads` launch policy in requirements.md. F-04/F-05 exercise all eight flag combinations and independent overlap gates on every payload, evidence, link, copy and export path, including stale observations and worker progress. Default history/inactive viewing works; erasure-pending content refuses by default and works with `--show-erased` only when still retained and all other gates permit it; telemetry structure/counts omit payloads unless `--show-payloads` permits them. `--no-show-inactive` hides applicable records even with other flags. No flag reconstructs absent content. Under the 2026-10-31 owner ruling, an Engine with atomic deletion and no legitimate retained-erasure target verifies unavailable/not-applicable content; the flag does not require creating that state.

- **INS-X07 — local access and stored content:** F-05 rejects non-loopback exposure, disallowed origin/Host, absent/wrong credentials and stale UI/service/session pairs before database operations. F-07 malicious stored markup/URLs remains inert with no automatic fetch, script execution or privilege-bearing link. Responses/errors/logs/URLs do not disclose credentials or payload/query/erased-identity content beyond the selected disclosure policy.
- **INS-X08 — accessibility:** In F-07 every required view supports keyboard selection, links, expansion/collapse where applicable and back navigation, visible focus and labeled numeric/text facts equivalent to visual encodings. Pointer-only graph/vector operation fails. At the qualified default 1042×793 and narrower 640×793 viewports, long content remains locally contained without page-wide horizontal overflow; query, origin, selection and return actions precede long raw dumps.
- **INS-X09 — installed lifecycle:** On the declared exact target, an installed/bundled artifact launches without a development server or Internet, discloses Engine mode before attach, opens a supported F-07 database, exercises all native views and exits within the stated bound with owned locks/handles/workers released. Missing models and refusal paths remain actionable; a skipped required positive view fails qualification.
- **INS-X10 — aggregate resource contract:** F-06 meets every row of the requirements operating-budget table across repeated navigation, cancel/retry, queued work and detach. Validate bytes/rows/examined links, peak memory, retained observations, acknowledged cancellation versus execution completion and finite shutdown. Budget exhaustion yields scoped partial/cancelled/busy outcomes, never complete empty success. Exact large/Unicode/numeric values round-trip through chunks and explicit copy.

- **INS-X11 — erasure-pending export:** With `--show-erased` and other applicable gates satisfied, explicitly export selected still-retained erasure-pending content to a local destination with exact value/identity/revision/context, bounded streaming and honest complete/partial/cancelled/error outcomes. F-04/F-06 checks bytes/structured values, all overlap restrictions, content disappearing before/during read and no mutation/network/erasure delay. With no flag or a denying overlapping gate, export, copy and alternate representation/evidence paths cannot disclose that content. Positive Engine end-to-end export is required only where a qualified supported Engine legitimately exposes readable retained-erasure content. Otherwise real Engine tests verify atomic deletion/absence/no reconstruction and no eligible target under all flags; typed policy/export contract tests cover supplied-content gate/stream semantics without mocking the database or fabricating storage state. Report that qualification scope explicitly. General export tooling remains outside v0.2.0.

## Fixtures and evaluation

The fixture oracle is authored before implementation from intended stored facts
and actual engine semantics. It is independent of Inspector output; no
regenerated snapshots or agent-invented expected retrieval ranks establish
correctness. Supporting design defines executable setup and shared-engine
conformance checks before implementation.

| Fixture | Required cases | Coverage |
| --- | --- | --- |
| F-01 — structure and retrieval | Canonical source, distinct derived node and edge, ordinary-write legacy/incomplete provenance, labeled engine bookkeeping, multiple kinds/roles, recorded dependencies, operational collection, text/vector representations, and a record reachable by multiple supported methods. | INS-01 through INS-05; INS-07 |
| F-02 — identity and revision | Equal text under distinct logical identities; identical bare values in different classes/namespaces; two retained revisions of one identity; anonymous/content identity ambiguity; unresolved result reference. A synthetic passage is tested only if that mode is supported, never fabricated as a stored record. | INS-03, INS-05, INS-X04 |
| F-03 — operational modes | Separate append-only-log and latest-state collections; repeated keys with multiple retained log entries; replacement of a latest-state value whose old payload is not recoverable; equal keys across collections; and a missing current value. Setup uses ordinary collection registration and writes, not fabricated cross-mode history. | INS-02 |
| F-04 — availability and visibility | Node inactive-state and edge expiry/invalidation cases, retained superseded data, owed physical closure and telemetry-redaction cases kept separate, ineligible and pending records, a failed materialization, a ready representation with unavailable query runtime, an eligible record outside a known bounded result set, unconfigured projection, missing linkage, and unavailable/erased targets. An omission cause is asserted only where independently established. | INS-04, INS-06 through INS-08 |
| F-05 — attachment and lifecycle | Missing/empty path, missing or held product lock, older/newer schemas, malformed input, pending work, initially nonempty WAL/journal, attach/cancel/detach, failed startup and preflight refusal. All external writers stop; the Inspector-owned Engine may run workers. Verify no bootstrap/migration/repair or Inspector mutation commands. Specify allowed lifecycle-effect classes independently, then compare with the same Engine build/options on equivalent real fixtures to attribute effects; equivalence alone is not the correctness oracle. Do not require identical timing or worker order. Preflight refusals preserve files; Engine-start failures disclose possible effects. Test evolving generation/read contexts, owned shutdown, and invalidation on replacement or incompatible identity/schema, without treating ordinary Engine writes as replacement. | INS-02.4, INS-X01 |
| F-07 — coordinated structure views | A canonical source/derived pair with verified text and vector origins, a known lexical match, independently checked vector distances, and directed multi-kind graph relationships including an edge with its own content. Test cross-view navigation, an empty representation, and bounded graph expansion. Graph participation is seeded explicitly, not inferred from embedding proximity. | INS-09, INS-03 through INS-06 |
| F-06 — scale and cancellation | At least 100,000 canonical revisions, 100,000 mutation entries, a 10 MiB payload, and a graph with 10,000 incident directed edges. Test limits, cancellation, usable partial states, and return to context. | INS-X02 through INS-X05, INS-X10/11 |

For adopted 0.2.0, F-07 includes the person investigation from INSP-B02:
biography/metadata, actual indexed-text match, supported single head-window vector/profile with exact recorded span and source-revision metadata,
known directed relations with edge content, readiness/lifecycle, equal names,
retained revisions and unknown links. Its two-observation extension independently
specifies known overlap/disjoint typed identities, origin cardinalities,
method-specific values and differing query/seed/filter/limit/read contexts,
plus capped, policy-refused, stale/evicted and unsupported cases. This extends
F-07/F-02/F-04/F-06; it does not derive expected ranks from Inspector output.
T-03 uses the person navigation scenario without changing its goal; T-04 covers
the two pinned observations against these independent facts.

### Functional investigation goals

T-01–T-05 retain the intended user outcomes below. Real database/browser and
installed checks use independently intended fixture identities and facts;
wrong projection/derived equivalence, equal-text identity, merged revisions or
search-absence-as-nonexistence fails the applicable goal. Ordinary documentation
and actual operation are the user interface; no tutorial is required.

- **T-01:** Locate requested data categories and identify what each contains.
- **T-02:** Locate a record without a supplied ID, inspect its values, and follow a requested relationship.
- **T-03:** Use the text, vector, and graph views to inspect a selected record's representations and graph participation, follow verified associations, and identify their retrieval uses with documentation available.
- **T-04:** Compare two result sets, identify shared/distinct records and revisions, and handle one equal-content or ambiguous-identity case.
- **T-05:** Inspect a stored record absent from one result set and distinguish observed storage/readiness/eligibility facts from an undetermined selection cause.

High INS-07/08 additionally require source/dependency/revision tracing and honest
original versus later retrieval evidence. The former participant audience,
assistance rubric and four-of-five diagnostic remain historical/unperformed in
the Slice70 study pack. They were removed from0.2.0 by the owner ruling, not
satisfied by an automated or expert substitute.

## v0.2.0 action acceptance signals

These are additional required signals, not alternative ways to skip the
original INS criteria. Test names and commands are selected in detailed design
before implementation; no nonexistent executable test is claimed here.

| Scope ID | Required acceptance signal |
| --- | --- |
| V01-N01 | The adopted D-01 table and all eight combinations of `--no-show-inactive`, `--show-erased`, `--show-payloads` match real fixture outcomes across display/evidence/copy/links/export. Separate gates compose on overlaps; history/inactive default viewing has real positive cases. Retained-erasure policy/export has supplied typed-content contract cases; positive Engine end-to-end cases apply only if the qualified Engine legitimately exposes that readable state. Autonomous atomic-delete fixtures verify absence/unavailable, all flags and no reconstruction. Current-format ExplicitMaintenance fixtures also verify real queued exact-scope content display/export, default refusal, unrelated-content access and unchanged persisted policy under all eight flags. Queued intent alone does not imply inactivity or logical removal. Physically absent content stays unavailable. |
| V01-N02 | Current ordinary node/edge writes produce the recorded incomplete-provenance cases. Detail exposes role/completeness and available identity; missing links stay unavailable. No public migration ladder is assumed. |
| V01-N03 | A node excluded by its state and an edge excluded by temporal rules produce class-correct detail and retrieval observations. No edge node-state field is invented. Boundary instants are tested. |
| V01-N04 | Independently calculated distance facts are compared to the backend's named metric and declared population. Binary shortlist membership and f32 reranking scope are disclosed for the staged path; exhaustive-neighbor claims fail unless separately established. |
| V01-N05 | Every supported lexical request identifies the actual route, indexed fields and scoring family. Tests distinguish BM25/BM25F, indexed text from body, and verified match evidence from absent evidence. |
| V01-N06 | Current compiled schema attaches under other valid preconditions. Older/newer schemas refuse without mutation and report supported versus observed versions; no upgrade or recovery action runs. |
| V01-N07 | With network access denied, valid cached query assets, and reranker assets if that optional mode is used, work. Missing, partial, or corrupt assets produce an unavailable mode without network requests, cache creation or repair; Engine lifecycle effects remain separately attributed; content/text inspection remains usable where independent. |
| V01-S01 | Engine-owned bookkeeping is labeled; structural role and provenance survive navigation. Scoped counts and filters distinguish supported categories without assuming test-gated writers imply routine production records. |
| V01-S02 | A source/branch-qualified reuse decision records adopted and rejected candidates. Supported lexical execution uses qualified Engine operations or explicitly owned scratch work; no immutable/query-only restriction forces a parallel lexical implementation. Inspector introduces no extra product writes beyond the selected Engine operation/lifecycle. Cancellation interrupts only connections owned by the request/session; late model work obeys the documented bound. |
| V01-S03 | Initially nonempty WAL/journal refusals distinguish unsupported attachment state from empty data; WAL created by an admitted Engine is not itself a refusal. Messages and docs say preserve files/sidecars and investigate owners; no lossy verb or deletion is offered as an automatic fix. |
| V01-S04 | Backend tests validate exact graph node/edge IDs, direction, expansion, lexical evidence, vector values and identity-preserving cross-view references. Actual browser checks exercise keyboard selection, expand/collapse, scope/truncation, and return navigation. A table-only interface fails. |

Passing core functional tasks alone is insufficient: High INS-07/08, all technical
criteria and installed lifecycle/disclosure/resource checks remain required.
Release readiness also verifies local CI prerequisite/failure behavior, package
contents/checksums/source/build identity and safe promotion under Slice80;90
checks the actual final ignored dist artifacts. No registry or human-study pass
is implied.

## Slice-local and final acceptance

Historical [Slice 15](plans/0.1/slice-15/plan.md) established criteria and
verification planning; [Slice 16](plans/0.1/slice-16/plan.md) mapped them to the
architecture. The [current 0.2.0 plan](plans/0.2.0/plan.md) allocates foundation
20, lexical 30, vectors 40, graph/history 50, comparison/person interaction 60
and installed/integrated qualification 70. Each executor
completes the assigned needs/req+AC/design, real implementation, focused tests,
Sol-high design/code review and direct Terra verification. Final integration
in Slice 70 is additional evidence, not a substitute for slice-complete
lexical, vector, graph or record functionality. An unavailable dependency
requiring Engine/MCP changes is an unresolved scope blocker, not a passing
unsupported-mode case for a required fixture.
