# Follow-up POC protocol and execution record

Status: EXECUTED WITH LIMITATIONS — P1/P2 scoped evidence; P3 campaign PARTIAL; not production-qualified.
Owner request: apply design changes and propose any additional work needed.
Production design: [current-history-search.md](../../design/current-history-search.md).
Existing isolated POC remains complete at `4b6bc1053`; its
[results](results.md) support correctness, not production latency acceptance.

## Recommendation and scope

Run three bounded follow-ups in this order, reusing the existing tests,
fixture generators and pinned SQLite. These close unproved storage, certificate
and resource assumptions; they do not repeat the already-passed basic temporal
oracles. Keep the production engine unchanged except for a disposable/test-only
candidate seam needed to exercise real constraints and routing. No new release
slice, permanent public method or schema migration is authorized by this proposal.
The user subsequently authorized implementation and measurement. The protocol below
is retained as the planned scope; [follow-up results](followups/RESULTS.md) record
actual coverage, revised diagnostic controls, failures and unresolved boundaries.

## P1 — Temporal mapping against real identity/lifecycle constraints

Question: can the three temporal read meanings coexist with the shipped canonical
identity, G11 scope and lifecycle/evidence authorities without reinterpreting them?

Build a small candidate temporal ledger/adapter against a real fresh database
with the actual schema and writer constraints, not the POC's looser assertions
schema. Reuse deterministic providers and direct canonical fixtures. Keep the
accepted one-current-head unique index enabled. Demonstrate mapping an opt-in
governed claim to immutable revision/fragment/source identities while ordinary
and anonymous records retain their existing contracts.

Write RED tests first for partial A/B/A correction, nested retraction, late and
future world changes, strict V/K boundaries, independent source claims sharing
endpoints, current authorization, the complete existing read-view flag matrix,
source closure and exact artifact/evidence version mapping. Inject a failure in
the shared write transaction and reopen/rebuild; no prior belief or canonical
head may be silently changed. Existing `logical_id` uniqueness, `superseded_at`
meaning and G11 behavior must remain enforced. A needed successor is identified
explicitly rather than bypassing the constraint in the fixture.

Pass: all stated semantic oracles and unchanged identity/ordinary-read invariants
hold; exact candidate data ownership and public semantic changes can be written
as a successor ADR/interface proposal. Fail: fragment representation requires
silently relaxing the head index, fabricates old knowledge, conflates a current
access policy with historical assertion state, or loses erasure ownership.
Output: concrete schema/identity mapping, required successor decisions and scoped
test results. This step does not claim a full production schema is implemented.

## P2 — Cheap certificate, clock refresh and default read path

Question: can certified current queries avoid work proportional to excluded
history, while boundary refresh remains correct under real writer/reader races?

Replace the isolated prototype's ID-set comparison with the production candidate
certificate/frontier and current-index-first collector. Use engine-shaped writer
admission and real read-only pooled connections; do not rely on per-query current
membership scans or a global in-memory cache. Reuse production query compilation,
tokenizer/stop words and tie/prefix behavior. Record candidate work separately
from the shipping baseline, with identical current data and query sequence.

RED/GREEN scenarios include exact certificate bounds, backward time, a future
record outside the current index, owner/source/dependency boundary crossings,
closure in progress, a source denial during refresh, write between admission and
reader snapshot, refresh rollback, and close/erase while requests wait. Force
multiple readers to request one expired scope simultaneously. Verify coalescing,
release-before-wait, same-snapshot validation and bounded retries. No success
may use stale membership or an unannounced historical corpus. Include a burst
where many records cross together; do not hide this work in warmup.

Pin V1 immediately before a boundary and V2 immediately after it; force V2's
refresh to complete first, then resume V1. Shared refresh is valid only if its
certificate interval covers that request's unchanged V. An older refresh must
not roll publication back over newer canonical/policy state. The older request
must get a compatible snapshot within policy or explicitly fail. Also queue a
refresh before source denial/erase and before repair/rebuild publication, then
resume it afterward. Verify current generation, authority and visibility rather
than accepting the task's captured prequeue values.

Hold 10,000 current node rows fixed and vary excluded prior versions at 0/4/16
per row. Use a separate edge/property workload with explicit counts rather than
reusing the old N=nodes+edges label. Certificate validation must read bounded
metadata, with no corpus/vocabulary/body enumeration; verify query plans and
loaded-row/VM-step attribution. Measure the entire read, writer refresh, queue
wait and close/erase effects, not just native BM25. A singleton check may use
point lookups; no artificial constant wall-time claim is made.

For preintegration scouting, use the prescribed T-012 generator/query sequence
and >=1,000 samples in release mode, recording current reads and separately
labeled history-ratio/boundary workloads. The 20/150-ms target can reject an
obviously unsuitable candidate, but a standalone adapter cannot pass AC-076.
Actual acceptance requires the unchanged T-012 fixture through integrated
`Engine::search`. Exercise the unchanged AC-081a/b fixture on candidate real
reader routing where available, including all admission/certification time;
otherwise report NOT TESTED, not an inferred pass. AC-081c needs its distinct
real-reader progress witness, not merely a concurrent WAL writer.

Pass: no stale success, no history enumeration on a certified read, correct
bounded refresh, no reader/writer deadlock, and scoped cost evidence supporting
integration against unchanged budgets. A failure triggers design revision;
never hide failed/limited requests from the latency report or loosen ACs.
Output: certificate fields/frontier, bounded handoff policy and error semantics,
all measurement cells, source of remaining history-dependent work and concrete
risks. This is the highest performance uncertainty exposed by the first POC.

## P3 — Historical backend, memory and lifecycle qualification

Question: which correct selected-corpus mechanism is viable for historical and
relaxed views, and what explicit limits/lifecycle does it require?

Compare native private materialization with a bounded-statistics custom
candidate only if it avoids the original all-vocabulary scan. Keep the old
custom implementation solely as a correctness reference; do not optimize it
without a measured reason. Use common selected-corpus statistics, full production
query-compiler behavior (including literal punctuation/quotes, Unicode, stop
words and stemming), all property names and zero-token rows. Do not add raw
FTS syntax that the engine's safe compiler does not expose.

Use 0/4/16 excluded revisions per versioned claim, one/eight callers, and fixed
128-byte / 4,096-byte indexed payloads (UTF-8 bytes after generation, not character
counts). Use separate statistical-family fixtures with explicit baseline counts:

| Family fixture | Current node owners | Current edge owners | Current indexed documents | All-version documents before diagnostic overlays |
| --- | ---: | ---: | ---: | --- |
| Node body | 10,000 | 0 | 10,000 node bodies | 10,000 x (H+1) node bodies |
| Edge body | 10,001 endpoint nodes | 10,000 | 10,000 edge bodies; endpoint nodes are a separate family | 10,000 x (H+1) edge bodies; nodes are not versioned in this cell |
| Property | 10,000 | 0 | 20,000 scalar documents, two names per node | 20,000 x (H+1) scalar documents |

Node/edge fixtures do not declare unused properties. Property owner bodies stay
fixed while both scalar payloads follow the size axis. Diagnostic corrections,
fragment splits and source restrictions report exact postchange canonical,
projected and admitted counts separately. Include corrected history, prior belief,
archive and each supported lifecycle relaxation. Distinguish admitted corpus,
query matches and winners. Test resource refusal/error cleanup, receiver drop,
close, source denial and erasure during population, including evidence's stricter
authority. Use the same generated input for every backend.

Predeclare experimental controls: exactly 1,000 scheduled requests per latency
cell, at most 600 seconds wall time per cell, a two-hour total campaign budget,
and an 8-GiB process-RSS stop budget. These are bounded experiment values, not
production limits or changes to AC thresholds. Confirm the selected host can
support that budget before execution; any revised budget is recorded before the
run, never chosen from observed outcomes. RSS monitoring may overshoot its stop
threshold and is not a hard allocation-cap guarantee. Record requested versus
actual payload sizes, execution order, all attempts and peak memory.

Do not retry a refusal/error as a replacement observation until 1,000 successes
appear. Backend-internal bounded refresh retries belong to the original request
and its measured time. Report success/refusal/error counts and their durations
separately. Resource/time-exhausted or unfinished cells remain INCOMPLETE; pending
cells are listed explicitly. No qualified p99 is reported without >=1,000 valid
successful samples and the required request-success behavior. Acceptance fixture
failures cannot be filtered out of an AC verdict.

Measure end-to-end p50/p99 for qualified distributions, peak process RSS plus clearly attributed
corpus allocation where available, SQLite temp-file behavior, queue/close time,
writer contention and `ErasureIncomplete` retry. Include corpus construction,
ranking, winner hydration and drop; do not borrow the previous small benchmark's
memory/latency numbers as bounds. Save rerunnable tools in this experiment;
raw per-query logs remain scratch artifacts, with concise outcome tables.

Pass: exact identities/order and numerical parity for the supported compiler,
no unauthorized/erased resurrection, memory-only private storage and deterministic
cleanup. Choose explicit corpus-byte/active-call/queue/deadline policy and typed
limit behavior from the measurements and intended envelope. No value is claimed
validated before this run. Over-limit work must not silently truncate statistics
or produce incomplete ranked success. Choose backend and cancellation behavior
only after reporting all cells. AC-076/081 are not automatically historical API
budgets; a new historical latency SLO, if needed, is a separate owner decision.
Output: backend recommendation, measured cost envelope, proposed resource values
and lifecycle/error semantics; unresolved tradeoffs stay explicit.

## Qualification after candidate production integration

Run the unchanged canonical protocols on the actual integrated paths:

| Gate | Required evidence |
| --- | --- |
| AC-076 / T-012 | Binding 10,000-row fixture, >=1,000 measured queries, end-to-end p50 <=20 ms and p99 <=150 ms |
| AC-081a | Exactly 1,600 sequential searches <=500 ms; warning >=200 ms |
| AC-081b | Exactly 1,600 total searches with eight callers <=100 ms; warning >=80 ms |
| AC-081c | Real distinct reader completes before a held peer snapshot is released |
| AC-072/073/075 | Preserve independently binding affected vector/stress/fidelity verdicts; lexical success is not a substitute |

Use tests appropriate to each change; do not rerun full regressions for each
finding. Run the required final production gate once on the completed candidate,
then scoped repairs if necessary. Astra reviews design/code and interpretation;
an independent tester verifies results. Any GitHub CI inspection/action uses gh.
P1/P2 scoped experiments have executed; P3 resources are PARTIAL under the recorded
combined budget. Final secure-delete P2 fixtures preserve healthy read isolation
but expose about 4.6-second full boundary refresh. See [results](followups/RESULTS.md)
for executed coverage and failures. No integrated production acceptance result is claimed.

## Next evidence: secure bulk refresh

Incremental maintenance can avoid full reconciliation for an ordinary single
write, but does not eliminate 10,000 removals at a simultaneous expiry boundary.
The observed secure-delete cost therefore needs a separate bounded bulk-refresh
and repair probe before production strategy selection. Compare transactional
fresh-projection rebuild/swap or a valid bulk-clear mechanism with the current
reference while preserving certificate publication, pinned snapshots, erasure
ownership, rollback and WAL completion semantics. Measure the complete burst,
including construction and publication; do not hide it in warmup or widen request
budgets to make it qualify. No such optimization is implemented or qualified here.
