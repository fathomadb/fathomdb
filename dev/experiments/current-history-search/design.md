# Formal implementation and hypothesis protocol

Status: COMPLETE — Astra reviewed; hypotheses tested and independently verified.
Implementation and outcomes: [results.md](results.md).

## Scope and intended contract

All three read meanings are mandatory:

1. Current authoritative facts: latest knowledge, valid at the selected now.
2. Corrected historical facts: latest knowledge, valid at a supplied past time.
3. Prior belief: knowledge as of K, valid at V, before later corrections.

An explicit archive query enumerates revisions. Archive search does not imply
that every stored assertion is true, and archive union is not a graph snapshot.
Application-supplied change intent and effective intervals carry semantics;
replacement order alone never infers a world change. Competing sources retain
separate claim identities. Current source authority is checked in every view.

The production target is a transactionally maintained current-search projection
plus explicit history retrieval, for node bodies, edge bodies and declared
property text. The POC models these with separate FTS tables for each family.
It uses the production `porter unicode61 remove_diacritics 2` tokenizer.
Default retrieval must be independent of excluded superseded revisions, including
scores, order, top-K and lexical graph-seed selection. An actual change to an
admitted current fact is allowed to change ranking. Default exclusion must not
be implemented as a recency penalty.

## Canonical model and writes

`assertions` stores a stable application claim ID, version ID, node/edge type,
body, scalar properties, source ID, endpoints where applicable, valid interval
[Vfrom,Vto), knowledge interval [Kfrom,Kto) and assertion status. A separate current
per-claim admission table gates every version; it is not historical truth.
NULL upper bounds mean infinity. Knowledge times are explicit monotonic logical
ticks, not inferred wall-clock event times. Each change has one transaction tick.
Application-supplied valid time may precede, equal or follow that tick.

Writes overlay an explicitly supplied valid interval on the currently curated
claim timeline. Old overlapping assertions receive a knowledge end; unchanged
left/right pieces are reasserted at the new knowledge tick. The replacement
covers precisely its requested interval. This supports late world changes,
backdated corrections and partial corrections without rewriting prior belief.
A retraction creates an explicit nonaccepted assertion for the affected interval.
A structural revision preserves the asserted valid interval. Reasons are data,
not new production lifecycle enum values. The POC accepts explicit intent
`initial`, `world_change`, `correction`, `revision`, or `retraction` and does not
perform natural-language contradiction or truth inference.

Canonical changes, historical projection insertion, and current projection
refresh commit atomically. An injected failure rolls back all three. A current
projection includes accepted, active, authorized assertions valid at its
materialization instant. No prior version resurrects after a retraction.
Rebuild derives both projections from the retained canonical assertions and
current authority; it never chooses truth from revision recency alone.
Source denial applies now even to prior-belief queries. Erasure physically
removes every version and every projection for the source and remains absent
after rebuild. The POC tests database contents, not OS memory zeroization or
physical WAL erasure; production at-rest erasure machinery is not reimplemented.

## Reads, statistical scope and graph semantics

A snapshot selects exactly one knowledge state and valid instant, plus current
source authority and active/accepted status. All selected canonical assertions
form its statistical corpus; requested property name restricts output only,
matching the engine's cross-property statistical baseline. Node, edge and
property FTS statistics remain separate. Content query terms never define N
or average length. Stable claim IDs and version IDs break score ties.

The POC query grammar is an OR of normalized ASCII word tokens. Phrase, prefix,
NEAR, column weights, arbitrary Unicode query normalization and production
query-compiler parity are deliberately unqualified. Bodies still use the real
FTS tokenizer. The custom comparator reads actual FTS vocabulary frequencies
and document lengths rather than a mock tokenizer. Tests include inflection
and diacritic bodies against supported normalized terms.

Graph traversal uses only edges and endpoint nodes admitted by the same V/K
snapshot and current source authority. Archive traversal is refused because a
union of revisions has no single temporal interpretation. Traversal groups
logical targets before its result cap; revision multiplication cannot consume
current seed or result slots. Time-respecting journeys across different times
are a separate operation outside this POC.

## Three ranking mechanisms

- `partitioned`: native BM25 over a maintained current FTS projection for the
  certified current view. Historical/other scopes use a selected-corpus path.
- `custom`: recall from all-version FTS; compute BM25 N, phrase/term document
  frequency, average length and term frequency over the selected snapshot.
  It scores every eligible match before top-K. This bounded implementation
  scans FTS vocabulary and is a correctness comparator, not an optimized kernel.
- `materialized`: copy exactly the selected scalar/body projection rows into a
  private per-query SQLite `:memory:` corpus, with `temp_store=MEMORY`, and use
  native BM25. Statements and the connection drop within the read call.

Raw BM25 scores from separate temporal corpora are never merged. Archive search
uses one explicitly selected combined corpus. Version-level archive results
include supersession/retraction intent, valid bounds, knowledge bounds and
source. Default, corrected-history and prior-belief queries return snapshot
facts, not audit revisions.

A maintained index is certified only while its membership still equals the
selected scope. Time boundaries can invalidate that certificate without writes.
The POC compares selected IDs with materialized membership; on a mismatch,
`partitioned` falls back to the custom scorer under the same held read snapshot,
without mutating a reader or copying a corpus. This O(N) certification is an
explicit prototype cost. Production needs a next-boundary/generation certificate
and a policy for refreshing under the writer; that optimization is not presumed
proved. Explicit historical queries and request-varying authority also bypass
an incompatible current certificate.

## Hypotheses, assumptions and predetermined tests

| ID | Hypothesis or assumption | Falsification / required observation |
| --- | --- | --- |
| H1 | Excluded history cannot affect default retrieval | Hold current rows fixed; add/change many superseded node, edge and property rows; exact native scores, order, top-K and graph seed identities remain equal |
| H2 | World change differs from correction | Late move preserves the earlier true address; correction changes the answer for the same past interval; prior belief still reproduces the original assertion |
| H3 | Temporal graph snapshots avoid invented paths | Nonoverlapping edges never form one snapshot path; positive simultaneous path does; endpoint revisions and source denials obey the same view |
| H4 | Three rankers agree within the supported grammar | Same selected corpus yields identical identities/order and scores within 1e-12 absolute tolerance; archive uses common statistics; all valid and knowledge boundaries are half-open |
| H5 | Current partition alone survives clock-only eligibility changes | Negative control must disprove this assumption; certified fallback must restore parity without a write |
| H6 | Atomicity and projection lifecycle are deterministic | Write fault leaves canonical/index output unchanged; reopen/rebuild round trips; inactive, denied and erased sources never leak or resurrect |
| H7 | History storage has cost but default native ranking avoids its corpus | Report all sweep cells; current FTS row counts remain fixed as excluded revisions grow; measure write/disk/query cost without assuming zero overhead |
| H8 | One request can observe one coherent snapshot across a concurrent write | Two real WAL connections preserve prewrite snapshot; later request observes postwrite replacement |

Assumptions tested explicitly: callers supply claim identity, intent and valid
intervals; knowledge ticks are strictly increasing; conflicting independent
sources are not auto-superseded; source authority is current, not time-traveled;
archived errors are assertions, not accepted facts; every projection family is
removed on erasure; no query limit is applied before semantic eligibility.
Malformed intervals and nonmonotonic knowledge ticks must fail atomically.

Property tests generate bounded legal update intervals, then verify curated
timeline nonoverlap, rebuild equivalence and serialization/reopen equivalence.
Hand-authored examples establish the semantic oracle; no generated golden
answers or LLM truth judgments are used.

## Execution and decision rule

1. Astra reviews this design, hypotheses and expected tests before GREEN.
2. Stage/commit a genuinely failing behavioral test per increment, with observed
   RED evidence; implement only that increment and rerun its scope.
3. Build all three rankers and the temporal graph prototype, then run the full
   focused POC suite and property cases. An independent agent reruns verification.
4. Run the fixed sweep (128/512 records, 0/4/16 excluded revisions per record,
   15 samples, warm serial queries, top 10). Record all cells and environment.
5. Astra reviews implementation and interpretation. Report supported, rejected
   and untested assumptions separately. Do not claim production performance,
   complete query-grammar parity, authorization deployment or full physical
   erasure based on this POC.

Correctness failures reject the implementation. If H1-H4/H6/H8 pass and H5's
negative control plus fallback behave as specified, prefer the current index
architecture for default ranking; use the measurements to expose its costs.
H7 is observational: no arbitrary latency threshold is invented after results.
Production adoption additionally requires design/ADR/API review, existing engine
integration, all arm/cursor/evidence paths, scheduler/rebuild/erasure integration,
and production-scale measurements. Those are follow-on implementation steps,
not accomplishments of this isolated experiment.

## Research basis

- [SQLite BM25](https://www.sqlite.org/fts5.html#the_bm25_function): table-wide statistics.
- [Wikidata ranking](https://www.wikidata.org/wiki/Help:Ranking): errors versus true historical values.
- [XTDB time](https://docs.xtdb.com/about/time-in-xtdb.html): valid time and system time.
- [Datomic database API](https://docs.datomic.com/javadoc/datomic/Database.html): archive versus snapshot.
- [Neo4j versioning](https://neo4j.com/docs/getting-started/data-modeling/versioning/): temporal nodes and edges.

## Recorded Astra refinements before GREEN

- Stable claim source, type and endpoints are immutable; incompatible writes
  fail. Every temporal fragment has a distinct immutable version ID, retains
  its original assertion intent and provenance, and cannot overlap another
  accepted fragment for the same claim at one knowledge time.
- Partial correction oracle: A[0,100) at K10 corrected to B[30,60) at K20
  produces A/B/A. At K19 the whole interval is A. Nested retraction[35,45)
  produces no accepted fact there and cannot resurrect A. Test exact boundaries.
- Query terms pass through SQLite's actual Porter/unicode61 tokenizer using a
  tiny scratch connection; lowercase ASCII is not treated as an equivalent
  stemmer. Duplicate and stem-colliding OR phrases preserve native BM25
  multiplicity. Test zero-token documents, IDF floor, ties and cross-property
  statistical rows. Scratch tokenization is not a copied search corpus.
- Source authority is current as of the pinned read transaction. An earlier
  reader may finish after denial commits; the next request must deny, including
  prior-belief queries. Graph endpoints name node claim IDs; independent source
  claims remain distinct. Temporal coherence does not imply conflict-free truth.
- Each backend timing includes certification/statistics/build/ranking/hydration
  and private-connection drop. Report native rank-only attribution separately.
  Add corrected-history, prior-belief and clock-fallback cells to the fixed
  sweep. Shared disk and write totals cover all coexisting POC indexes and do
  not estimate an individual backend's footprint.

## Implementation review corrections

Astra's four direct witnesses require current per-claim admission to gate all
versions, exact accepted-fragment validity for structural revisions, both
eligible endpoints for graph seeds, and source validity for archive access.
Admission is a current policy, like source authorization: prior-belief reads
reconstruct assertions but never restore former authorization/admission.
First ingest may be labeled Revision when importing an externally revised
record; Initial is only legal for a new claim. Existing Revision must match
one accepted current fragment exactly and cannot fill retracted or disjoint
gaps. Other interval changes require explicit world-change/correction intent.
Archive source validity is evaluated at the experiment's explicit current
instant (`config.now`) inside the same held read snapshot. Snapshot source
validity is evaluated at its selected V; current allowed/active/admission
policy applies in both. Erasure removes the claim admission record too.
