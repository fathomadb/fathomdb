---
title: Slice 80 bounded residual design
status: PROPOSED
target_release: 0.8.29
slice: 80
---

# Slice 80 design

This is the concrete contract for owner acceptance, not an implemented API.
[Census](census.md), [requirements](requirements.md), [acceptance](acceptance.md)
and [test map](test-map.md) identify the source delta and observable oracles.

This revision uses integrated source
`e1cb1bd5311f68da8be98bbe133b9cecd5ab56d4`, selected identity/packing
seq-374 and feedback seq-375, and Slice50's independently reviewed
`10a72e5a9` carrier design. Selected design is distinguished from implemented API.
The earlier aggregate-v2 and consolidation consistency reviews passed; they are
not repeated. The updated contract needs its required Astra-medium review and
full80 owner acceptance. No implementation or90 source entry follows.

Integrated60 supplies format-conditional discovery, text omission and final
budget accounting; integrated70 supplies embedder_absent, embedder_timeout and
embedder_overloaded across bindings/hints. Preserve these built behaviors.
Selected50 identity and feedback policy are settled; its carrier and atomic
refusal remain implementation work. No new public occurrence-label ABI follows.

## Informative responses, errors and default store

Use Option plus skip_serializing_if for envelope state/search verdict, populated
None. Orient lists [state,verdict,kinds,counts] in that fixed order. Keep existing
structured kinds/counts placeholders as accepted in hints §6;60 already omits
those two text rows. Preserve runtime/embedding/projection readiness and all
degradation entries. Missing capabilities are not hint reasons. Neither state
identity, freshness, calibrated confidence nor weak-result classification is
implemented by this presentation change.

Build accepted top-level argument names once from the same schemars input
schema as discovery; store a sorted immutable list per tool. Precheck original
keys using bytewise UTF-8 lexical order before serde parsing. Select the least
unknown original key, then shorten only its diagnostic copy: if raw len>64,
take the longest char-boundary prefix of at most61 bytes, append U+2026 (3 bytes).
Never sort by truncated text. Nested attribute maps are not top-level arguments.
Known-key-only type/required-field validation remains bad_argument; an unknown
key wins over those typed failures and all store/admission work. Request-level
JSON/object/protocol faults remain rmcp faults. Details allocate only on error;
valid calls perform membership checks against the prebuilt list without
constructing per-call accepted Vecs/schema objects.

ToolError gets optional argument, accepted, refs_not_in_response and refs_omitted
typed fields, omitted on unrelated errors. unknown_argument has fixed message
"unknown tool argument"; refs_not_in_response has fixed message
"used and unused may only name references from that response". No caller text
is interpolated into these messages or remedies. Generic serde-error sanitizing
beyond this unknown-key branch is not silently broadened into80 scope.

| Tool | Fixed unknown-argument remedy |
| --- | --- |
| list_stores | use hints to control suggestions; list_stores takes no other arguments |
| use_store | pass store and optionally hints; use a listed store name |
| orient | pass store and optionally hints |
| search | use only the accepted search arguments; continuation calls take only store, continuation, max_tokens, hints and highlights |
| fetch | fetch takes no limit; pass fewer refs, or max_tokens to bound the size |
| expand | use only the accepted expand arguments; lower limit or max_depth to bound work |
| feedback | pass response_id, used and optionally unused, store and hints |

Feedback retains parse→resolve store→ensure slot→response lookup (wrong_store
or response_expired)→membership→epoch→telemetry→text mode→query ID→acquisition
and epoch recheck→record. Build offending data by iterating used then unused;
do not reuse the sorted BTreeSet as diagnostic order. Repeated offenders remain
repeated occurrences. Include the first50 offenders whose raw UTF-8 length≤128;
skip whole overlong refs and count them plus every later nonincluded offender.
refs_omitted is absent when0, else an exact checked u64 count. No new input-size
limit or successful duplicate/overlap-label policy is inferred. Malformed or
unsupported-looking strings are membership failures here, not fetch parse_ref
errors. No labels reach record_feedback on membership failure.

Selected50 seq-375 uses Unique(cursor)/Ambiguous response membership.
After acquisition and the final epoch recheck, but before resolving labels or
calling record_feedback, reject any named ambiguous ref with existing
invalid_request/bad_argument and zero writes. Preserve earlier membership
correction and existing epoch/telemetry/text/query-ID/acquisition not_recorded
precedence. Repeated same-cursor refs are unique; unmentioned ambiguity is inert.
This is selected planned50 behavior, not a new80 error code. A ref can belong
to the page while naming several records. No feedback_ref, first/last choice or
all-occurrences attribution is introduced. Logical fetch remains latest-eligible
resolution under its own view; h:/p: support and helper/pack contracts stay as
selected in the identity ADR.

Both new errors render structured fields verbatim as compact JSON and text
argument=<JSON string> accepted=<JSON array> or refs_not_in_response=<JSON array>
[refs_omitted=<n>] after60's existing error fields. Errors have no hints/budget
envelope and remain outside success max_tokens, as existing tool errors are.
This is an explicit finite diagnostic bound, not a promise to fit256 tokens:
at most6400 raw ref bytes, at most38400 JSON escape bytes plus fixed framing,
accepted key names and the bounded numeric count. Never silently drop accepted
keys or truncate a ref into a different valid identifier to fit a success budget.

ListStoresOutput.default_store is required nullable, independent of hints.
Reuse the one sampled session value for flags/hint/value; text is
`stores: N default="name"` or `stores: N default=null`. Use JSON encoding even
though valid store names are ASCII and bounded. Keep use_store's existing
action and60 quoted rendering. Guide: calls without store use the session
default; list_stores/use_store are needed to choose/switch. Preserve1200-byte cap.

## Aggregate reporting: reviewed bounded contract

The following concrete specification incorporates exact aggregate v2 SHA-256
`a5e4851df2fedcbcce3e7d1b93138980c25e172d0ceb802792d218940bca0e53`,
with its bounded independent Astra-medium PASS. It selects the documentary
carrier/architecture for this packet; it is not new ABI/policy/source approval.

### Selected population and public carrier

Propose **stored mapped-span coverage for attributable canonical records**,
not live-query coverage or physical-vector health certification.
Accepted long-record §7.1.2 requests record/byte shares cached with projection
state with no extra orient scan; it does not prescribe live E/P/K/U/M. The
following is the concrete narrow interpretation submitted for normal80 review,
not an assertion that the earlier owner approved these particular fields.

The denominator population is canonical node/edge records with an unambiguously
attributable committed mapping/head-span tuple in the read snapshot, under the
one stored `default` embedder profile. These are facts recorded in mapping/span
metadata; availability does not certify that every physical vector exists.
It includes retained superseded/inactive/out-of-window records and records
whose current declaration no longer targets vectors. It ignores current
ReadView, source eligibility and runtime-provider availability. There is no
per-projection allocation: current code uses one vector_default partition.
It does not claim every canonical record has a vector, or that any query scored
these records. Profile identity comes from stored metadata, never the attached
runtime name/device. Configuration affects membership only through actual
committed stored tuples; no reproject or new generation is requested.

Head uniqueness is grounded in vector_spans::validate: r.rowid=r.write_cursor,
s.vector_rowid=r.rowid, s.write_cursor=r.write_cursor, window_number=0 and
byte_start=0. Both mapping rowid and span vector_rowid are primary keys;
UNIQUE(write_cursor,window_number) rules out a second0. Therefore each admitted
canonical cursor counts once, independently of logical refs/body identity or50.
Preserve the existing validation predicate **exactly**, including its
COALESCE(n.body,e.body) body-length/boundary semantics. A missing/orphan span
or any tuple/length/end condition that currently fails remains the same typed
error. Apply exclusion classification only after that predicate succeeds; an
excluded row must not hide a pre-existing validation failure.

Count the following passing mappings as `unattributed_heads`, outside R/K/U/T/B/V:
no canonical owner (including write_vector_for_test), exactly one canonical
owner whose body is NULL (bodyless edge), or owners in both node and edge tables
(ambiguous attribution). Do not reject any of these merely for reporting or
treat a known end on an excluded mapping as a canonical known/unknown record.
The field means a span could not be attributed to exactly one canonical body,
not merely that a cursor is absent. A NULL end on an attributable body remains
unknown; a NULL body is excluded, not an invented zero-length body.

Physical completion retains its separate existing policy: member_completion
checks physical vectors for eligible members; dense_member_kind_at excludes
bodyless/expired edges. physical_completion independently refuses cross-owner
cursors today. Those callers retain their errors and cannot publish a staged
summary if their later checks fail. The earlier open-time span validator does
not gain those completion checks. Broader integrity validation is outside this
reporting proposal; a missing physical vector is not newly probed or certified.

Add required `vector_coverage: StoredHeadCoverageV1` to existing Rust
ProjectionRuntimeStatus and its SDK/facade/native/public conversions. No new
operation, option, SearchHit/SearchResult field, schema or persisted counter.
MCP orient gets top-level vector_coverage by moving this member out of its
existing projection_status conversion, so the wire carries it once.

Wire shape, with all fields required in their applicable tagged variant:

```json
{
  "population": "stored_mapped_spans",
  "profile": {"key": "default", "name": "example", "revision": "r1", "dimension": 8},
  "status": "available",
  "records": "4",
  "known_records": "3",
  "unknown_records": "1",
  "truncated_known_records": "1",
  "known_body_bytes": "16",
  "unrepresented_known_bytes": "6",
  "unattributed_heads": "0",
  "truncated_known_share": {"numerator": "1", "denominator": "3"},
  "unrepresented_known_byte_share": {"numerator": "6", "denominator": "16"}
}
```

```json
{
  "population": "stored_mapped_spans",
  "profile": {"key": "default", "name": "example", "revision": "r1", "dimension": 8},
  "status": "unavailable",
  "reason": "state_changed"
}
```

Unavailable reasons are exactly `cache_cold` (no complete seed for this open)
and `state_changed` (a retained seed does not match). Neither is a storage-health
diagnosis. Unavailable contains no counts, shares or stale snapshot numbers.
The profile is the current snapshot's valid stored identity in both variants.
Missing/malformed profile or authority metadata is an error, not cache_cold.
Open's existing fresh-profile initialization is handled before serving any call.

Rust enum: StoredHeadCoverageV1::{Available{profile,summary},
Unavailable{profile,reason}}; StoredHeadCoverageSummaryV1 has the seven numeric
members above and two Option<ExactCoverageFractionV1> members. Counts are u64
bounded by i64::MAX; byte sums and fraction components are u128. Fractions are
exact pairs, denominator>0 and numerator≤denominator; do not reduce them,
because the explicit denominator is useful. There are **no floating ratios**,
NaN, infinity, rounding or thresholds. A share is null exactly when its
denominator is0. The population/key literals are converter constants.

MCP JSON and native TS transport encode all counts/bytes/fraction components as
canonical unsigned decimal strings (`0` or nonzero digit followed by digits),
with checked u64/i64-count or u128 limits. Profile.dimension stays u32 in Rust,
an exact Python int, and a JSON/TypeScript number (example8); it is outside
the decimal-string domains. This follows existing NAPI projection
cursor/count string transport and avoids JavaScript precision loss. TS public
vectorCoverage is a discriminated union with camelCase names and string
integers; share null stays null. PyO3/Python expose exact Python ints and frozen
tagged values (snake_case), never float/int coercion. Native/wrapper decoders
reject malformed tags, missing variant fields, unknown reasons, noncanonical or
out-of-range decimals and contradictory count/fraction relationships. Re-export
the returned types through existing engine/facade/SDK roots and maintained
Python/TS type roots. No governed-operation allowlist exception is needed.
Malformed native payloads use the existing ProjectionGenerationError reason
projection_generation_corrupt with the exact RFC6901 field path below
/vectorCoverage (TS) or /vector_coverage (Python), as existing projection native
decoders do. Do not misclassify them as caller-invalid arguments or cache misses.

### Arithmetic, omitted populations and examples

Let R=records, K=known_records, U=unknown_records, T=truncated_known_records,
B=known_body_bytes and V=unrepresented_known_bytes. For each admitted canonical
head, NULL end means unknown, even if its body is empty. Known end e satisfies
0≤e≤length and is a UTF-8 boundary. R=K+U, T≤K, 0≤V≤B.

- T counts known e<length. Share is (T,K), or null at K=0.
- B sums body_length over K only; V sums body_length−e over K only. Byte share
  is (V,B), or null at B=0. Unknown body lengths never enter either byte sum.
- Known-empty(length0,end0) increments R/K, not T/B/V. Unknown-empty increments
  R/U. No NULL→0/full conversion, including empty input.
- No-vector, pending and failed-terminal canonical records without a committed
  head are outside R. No count of those records is asserted. Existing readiness
  pending_count keeps its own meaning; it is not an inferred missing-head count.
- Unattributed heads have their explicit separate count; they never enlarge K,
  U, B or V. Bodyless and ambiguous-owner passing mappings join the same
  explicit exclusion counter. Unmapped physical vectors cannot enlarge this
  mapped-span population. A missing physical vector does not erase its stored
  span fact here. This summary makes no physical-presence/completeness claim.

Hand oracles: (length,end)=(10,4),(6,6),(0,0),(8,NULL) produce the JSON above.
Adding a pending10-byte canonical record leaves every coverage number unchanged;
adding an ownerless, bodyless or ambiguous-owner mapping that passes the existing
span predicate increments only unattributed_heads. Each such row counts once.
Existing completion errors still apply when that distinct operation runs. A known-empty
singleton gives R1/K1/U0/T0/B0/V0, shares0/1 and null. An unknown-empty singleton
gives R1/K0/U1/T0/B0/V0, both shares null. A validated empty mapped store gives
all seven counters0 and both shares null, even if it has many unprojected
canonical records. Do not label that result “empty database” or “fully covered.”

Runtime absent with persisted heads still reports available stored facts if the
cache matches. Profile/config changes invalidate the seed. Time alone does not
change this population; source/node/edge validity boundaries do not expire its
cache or change these ratios. This is not a silently weakened live denominator.

### Capture with existing full validation

Concrete code ownership changes proposed:

1. vector_spans.rs: retain scalar validate(Some(cursor)) and its scoped checks.
   Add crate-private validate_full_with_coverage(&Connection) returning the
   numeric summary, not an Engine cache mutation. Replace the healthy full
   validation's EXISTS output with a streamed mapping-row result carrying its
   existing invalid predicate, owner presence, body_length and nullable end.
   Preserve the existing orphan-span anti-join check in that same SQL statement.
   Carry owner-presence/body-NULL classification as scalar columns solely to
   select denominators/exclusions after the unchanged invalid predicate. No
   body String, vector BLOB or per-record Vec is transferred to Rust; update
   seven checked counters as rows stream. Preserve existing typed validation
   errors, without adding NULL-body or dual-owner refusal to this path.
2. Perform **no vector_default join, rowid/EXISTS probe, column read or extra
   physical-schema check** for aggregate reporting. Pinned sqlite-vec0.1.9
   sqlite-vec.c:7363–7400 vec0Filter_point iterates every vector column and
   calls vec0_get_vector_data even for a rowid-only request. N indexed probes
   would therefore read corpus-wide vector payloads; EXPLAIN index access does
   not prove metadata-only work. Leave existing completion's physical queries
   where they already run, outside this aggregate validation/capture path.
3. Stream arithmetic in Rust, not SQL SUM/TOTAL. A validated body_length is a
   nonnegative i64, converted to u128 before checked addition/subtraction.
   At most i64::MAX positive record cursors and i64::MAX bytes per row imply
   B,V<2^126, within u128; checked operations remain defensive failures. Counter
   increments check i64::MAX. No SQL REAL conversion or saturated total.
4. Engine owns one Mutex<Option<CachedStoredHeadCoverage>>. Cache stores only
   the summary, profile and internal token; no body/ref/per-record data. A
   staged seed has no global visibility until its read transaction commits.
   The cache is tied to the single Engine-owned writer Connection, never shared
   between engines or populated from a reader-worker connection.
   Lock order is writer Connection then coverage cache; copy the entry and
   release the cache lock before SQL. No cache-only caller subsequently takes
   the writer lock, and no projection worker receives a mutable cache handle.

The full stream still visits all mapped rows on a healthy store, plus existing
orphan-span validation. Added Rust row transfer/arithmetic is **new O(N) work
within that existing pass**, not free work. It adds no second store-wide
traversal and no aggregate-driven physical-vector reads. Inspect the actual
query/statement work, not an indexed-plan label, to preserve that boundary.
This proposal adds no physical-vector census or integrity policy.

Open capture: in open_locked, enclose the existing vector_spans full validation
and the immediately following frozen_read::validate_on_open
in one Deferred read transaction, retaining their current order and error maps.
Capture the coverage token/profile in that same pinned transaction after those
checks, commit, and return an optional seed through the existing open-result
tuple/OpenPostProbeGuard parts into Engine construction. If the default profile
has not yet been initialized by the existing fresh-open path, carry no seed;
do not turn an unexpected SQL/decode failure into this bootstrap case.

Later open work can change state: profile initialization, generation bootstrap,
prune/registry/attribute repair, mean recovery, equivalence probe and historical
edge/backfill graft. At Engine construction compare the seed token again on the
same writer connection before starting ProjectionRuntime workers. Keep an
equal seed; discard any changed/incomplete seed without retagging old numbers.
No relocated validation, extra full validation or “repairs preserve counts”
assumption. Stable quiescent reopen normally seeds numeric reporting; repaired
or fresh opens may legitimately start cold. rederive_projections_on_boot already
returns without writes when persisted and expected caches match.

Refresh capture: physical_completion currently calls full vector_spans validation
before completion aggregation. Return its computed coverage alongside the
existing CompletionSummary through a private status result when a private
capture mode is enabled. Enable it for writer-side cache-refresh misses; keep
ordinary reader/explanation full validation in its existing noncapturing mode
so reporting adds no counters/row-transfer/probe work to those searches.
On a full miss in
Engine::cached_status_in_snapshot, stage that coverage and token. Publish only
after successful tx.commit in read_projection_generation_status_at or the
existing mutation-status caller. A later completion/commit error publishes
nothing. Existing generation-status cache hits may reuse only the independent
Engine coverage cache's already committed matching entry; they do not save or
publish an uncommitted candidate inside the generation-status cache. An empty
or unequal coverage entry does not force a coverage scan on a generation-status
cache hit. Direct status_in_snapshot callers in explained reader searches never
seed this cache; their connection-local tokens are inapplicable.
Do not add explain=true, a scheduler refresh task or a public refresh operation.

### Token, orient reuse and lifecycle behavior

Token fields, all read while one Deferred transaction is pinned:

- database ID from the existing `_fathomdb_open_state` singleton key;
- schema/user_version37, schema_version cookie and exact stored default-profile
  name/revision/dimension;
- read-visibility generation plus state_nonce and coalescing==0 from its singleton;
- writer-connection PRAGMA main.data_version and Connection::total_changes().

Read the existing singleton/key rows directly and validate their scalar domains
(lowercase ID/nonce hex lengths, nonnegative generation, exact clean integer
coalescing). Do not call frozen binding_for_snapshot, current_generation or
projection_registry_digest for this token: they do additional history/registry
work. No open epoch, timestamp, current-generation ID or process counter alone
is authority. total_changes is an existing SQLite connection counter, not a
new persisted counter; it conservatively detects local DML, including writes
that bypass visibility triggers. schema cookie covers local DDL.

PRAGMA data_version is compared only on the same live writer connection. The
pinned libsqlite3-sys0.39.1 amalgamation implements it as a read-only header
value backed by pager/Btree data-version state; own commits are compensated,
so it cannot replace nonce/total_changes. It supplies additional detection of
commits from projection/runtime connections or external connections. Never
compare another connection's numeric value or promote a reader snapshot seed.
This retains the existing trigger-governed external-write model; malicious
file replacement, disabled triggers or deliberately forged metadata is not a
newly promised integrity model.

In Engine::read_projection_status, change its existing connection lock to a
mutable guard and open one Deferred read transaction. Load the token (first
table read pins the snapshot), existing runtime-status facts and default profile;
clone an equal cached summary, or return unavailable/cache_cold/state_changed.
Commit before returning. All added coverage work is fixed singleton/key/PRAGMA
reads plus cloning the fixed-size numeric summary and stored profile: no span/vector/owner scan or
call to read_projection_generation_status. Existing readiness queries are not
claimed O(1), and the other two orient SDK calls remain separate reads. Only
the coverage value/profile/token are one-snapshot facts; no whole-envelope
state identity is invented. MCP serialization performs no database work.
Here O(1) means independent of record/vector count. Profile string read/clone/
escaping still costs its byte length; no new identity-length cap is invented.

| Event | Cache behavior / truth |
| --- | --- |
| Ordinary owned DML commit | Visibility nonce/generation or total_changes differs; unavailable/state_changed until an existing full validation refreshes. No incremental counters or per-write coverage work. |
| External/projection connection commit | Trigger-governed changes alter nonce/generation; same writer data_version adds protection for other committed changes. The next status transaction cannot serve an unequal seed. |
| Commit after capture snapshot, before publication | Seed still bears its old token; next current read sees mismatch. An old pinned reader never imports this writer cache. |
| Rollback after attempted DML | Committed nonce/generation stay unchanged, but total_changes can advance. Conservatively report state_changed, never publish uncommitted candidate counts. No persisted fact changes. |
| Projection tuple commit | Current projection_commit atomically inserts mapping/span/vector/terminal; reporting reads only its committed mapping/span facts and new nonce/data_version invalidates. Partial rollback cannot seed or partly adjust numbers. |
| Vec0/full rebuild, reshape, mean/recenter | Mapping/span/profile/schema/connection changes invalidate conservatively even if the mathematical totals stay equal. Existing refresh may republish exact equal totals; no guessed retag. |
| Erasure | Mapping DELETE cleanup removes spans transactionally; old cache cannot be served for the new state. No excluded/purged body-derived values are emitted from the stale object. |
| Reopen | New Engine/cache/connection; only this open's complete committed seed is eligible. No cache persisted to database or process-global map. |
| Time-only validity transition | Coverage key/population unchanged; the stored-head summary remains usable. Live search eligibility can change independently. |

SQL/metadata/invariant failure while loading token, validating spans, streaming
rows or committing returns the existing Engine Storage/ProjectionGeneration
typed error (existing open error mapping on open), then existing SDK/native/MCP
mapping. Poisoned cache lock is an error too. Only successful token reads with
no/equal-vs-unequal seed choose unavailable/available; no catch-all fallback.

### Availability limits, rendering and acceptance delta

This implements actual numeric reporting on stable reopened stores and after
already-required full generation validation. Cold or state-changed stores may
remain unavailable indefinitely if the host neither reopens nor invokes an
existing full status read. That is an explicit availability limit, not a
permanently absent implementation and not permission to make orient scan.
The current accepted wording does not promise always-current numbers after
each write. If that stronger guarantee is required, this design does not meet
it; it would need reviewed maintenance across every mutation boundary or a
scan-policy change. No such stronger guarantee is inferred or asked here.

Keep orient.not_in_this_release exactly [state,verdict,kinds,counts]. Coverage
is now a supported capability with an availability result, not a fifth missing
capability. Text emits one `coverage: <compact JSON>` line using the exact wire
object above. It is deliberately the same typed object as structured output,
so quoted identity strings, decimal integers and null fractions cannot diverge.
60 final accounting includes this line/object; fixed-metadata overflow follows
its refusal contract, never omits counts to fit or yields an oversize success.
No hint, retrieval-mode advice, weak threshold or extra flag is generated.

R6/A6 and O80-10–12 carry this exact carrier/population/availability contract.
The future90 runtime RED oracles are: real existing
projectionStatus/orient calls return the hand fixture's values and exact
fractions; absent new field on baseline is a runtime semantic failure. Keep
cache-cold and refreshed behavior in the same RED scenario. Existing engine
test-hook providers/real files and dynamic native/JSON reads avoid compile-only
missing-helper failures. Use native Python conversion of a real engine status
for deterministic partial/unknown fixtures, then installed TS/MCP boundaries
with approved existing local-model fixtures as in PhaseA. Preserve source-bound
TDD and goldens; bounded aggregate v2 PASS does not replace the current
updated-design review, full80 owner acceptance or later source reviews.

Proof controls: two corpus sizes confirm no full validation/owner traversal on
warm or cold orient; actual capture SQL/trace contains no new vector_default
access, including indexed probes. Known0/unknown0/no-vector/pending and all
three excluded attribution cases match the oracles. Existing invalid-predicate/
orphan-span failures, token corruption and forced SQL/commit failures retain
typed errors; missing physical-vector and dual-owner failures remain controls
of existing completion operations, not new reporting/open refusal oracles;
concurrent external commit, rolled-back coalescing branch, rebuild/erase/reopen
and profile/config mutations cannot reuse a mismatched seed. Counts and ratios
round-trip at u64/i64-count/u128 limits through both native boundaries without
JS Number conversion. Record real full-pass overhead before any performance
claim. This Slice80 documentary consolidation executed none of these database/native
checks. The separately reported60 manual checks do not qualify80 behavior.

## Per-hit scored extent through the existing sidecar

Proposed implementation field: `SnippetV1.scored_bytes: Option<u32>`.
It extends the existing opt-in sidecar; SearchHit/SearchResult gain no fields,
flags or public ownership/participation bit. Some(e) asserts that this exact
returned canonical record participated in the original vector arm and has a
validated stored head extent [0,e), relative to its original stored UTF-8 body.
It does not assert that all those bytes affected a model score, a lexical/CE
score, the displayed window or this query's semantic relevance. Some(0) is a
known-empty stored extent. None asserts no known scored extent, encompassing
validated provider-unknown and no vector contribution; consumers cannot infer
which from arm. No additional discriminant is selected.

### Ownership and participation seam

Use reviewed50's planned `FusionHit { hit, ownership }` and selected private
RecordKey: Canonical(Node|Edge,cursor), or Ownerless(NodesFts,rowid).
Before the **first** fusion, retain keys of actual original vector-arm
candidates in a request-local HashSet only when snippets are requested. Membership
survives both fusion stages, reweighting, CE and final truncation via these same
keys. Entry.in_vector is merely current-input tie preference: nested graph
fusion passes a two-arm result as its vector input, so that bit is not proof.
A graph-only or text-only record with an existing stored vector remains None.
Ownerless FTS makes no coverage read, even if its numeric rowid collides with a
canonical cursor. Neither raw cursor alone, ref, body, display branch, CE nor
explanation is a key or participation authority.

50's `SearchOriginCapture::finish` is planned to receive final carriers for
every request, independently of evidence=true. Compute snippets and attach
coverage before NoEvidenceCapture strips those carriers into SearchResult;
evidence-off and explain-off calls get the same metadata. Existing selected
representative-first source/body/ownership stays attached. Do not add a second
ownership carrier or reconstruct one from final bare hits. This is a reviewed
integration seam, not a claim that50 has implemented it.

### Point read and refusal contract

For each final canonical participant, on the same pinned reader transaction
before commit, perform at most one indexed metadata/physical-presence statement.
Select the exact chosen canonical table/cursor and mapping/span tuple; use the
existing OR identities (vector_rowid/write_cursor) or equivalent indexed probes
to detect misplaced/extra heads rather than silently selecting LIMIT1.
Validate exactly one canonical owner/body, mapping rowid=write_cursor=selected
cursor, span vector_rowid=write_cursor=selected cursor, window_number=0,
byte_start=0, nonnegative integer body_length equal to the returned stored-body
byte length, and the existing physical vector identity/presence rules. Do not
compare representative edge_fact kind to stored kind as if they were identical;
preserve existing vector ownership semantics. The selected stored body must
match the carrier; no borrowing a co-cursor edge/node, equal body or newer
logical version. The full-store validator is never called per hit.

| Read outcome | Result |
| --- | --- |
| Snippets off, nonparticipant or ownerless FTS | No coverage query; None if a snippet exists. |
| Exactly one valid tuple, byte_end NULL | None, including an empty body; provider-unknown is not zero/full. Validate the rest of the tuple first. |
| Exactly one valid tuple, integer end at UTF-8 boundary within [0,body_length] | Checked u32::try_from(end), Some(e), preserving0. |
| Missing mapping/span/physical vector, extra/misplaced/nonzero head, ambiguous canonical owner, body mismatch, malformed end or invalid UTF-8 boundary | Existing typed Storage/projection_generation_corrupt refusal, never None or partial successful results. No repair, full scan or best-head selection. |
| Known valid end exceeds u32::MAX | Typed invariant/corruption refusal; no clamp, wrap or unknown fallback. |
| SQL/read/transaction commit failure | Existing typed error; no metadata downgrade or successful partial sidecar. |

There is one stored head under accepted format37. Multihead is corruption in
this release, not a range-selection policy or chunking implementation. Full-body
length may exceed u32 when end is unknown or fits; only the exported known end
has the checked u32 domain. Existing snippet coordinate limits still apply.
Frozen authority validation remains before retrieval; later mutation keeps its
existing state_drifted/state_unavailable refusal. A later writer's span cannot
enter this pinned search snapshot. Runtime-provider availability is not span
truth. Physical-presence reads may load one vec0 payload internally; unlike
aggregate capture this is bounded by final participant count, not corpus size.

At most one extra coverage statement per final canonical vector participant
(limit≤100), separate from existing2*max_point_queries lexical work. Snippets
off adds no participation set, coverage query or sidecar allocation. No tokenizer
recomputation/provider call, whole-store scan or new model qualification.
Start/Match/Whole selection, windows, matches/highlights and ranks remain under
the existing selector contract; vector-only long hits use Start, short hits Whole.
Standalone snippet_for is pure and sets None without storage.

## Binding and format mapping

| Layer | Candidate mapping / absence |
| --- | --- |
| Rust engine/facade/SDK | Existing SnippetV1 plus scored_bytes:Option<u32>; SearchResult.snippets None when off, Some([]) for requested empty results.70 facade exports/reexports are integrated; extend their returned snippet type. |
| PyO3 / Python | Native getter Option<u32>; dataclass scored_bytes:int\|None=None; `_sdk_search` maps exact integer/None. Never coerce missing0 via truthiness. All three existing search surfaces share it; standalone helper None. |
| NAPI / TS | Native optional integer; public scoredBytes?:number, present only for Some including0. Declare native binding.ts field and mapNativeSnippet; validate any dynamic conversion as integer0..2^32−1, never wrap. Entire snippets property absent when off. |
| MCP structured | Hit.scored_bytes?:u32, omitted for None; copy from the aligned snippet. No new input flag. It means exactly the same scalar, not snippet length. |
| MCP compact text | Add optional scored_bytes=<n> before the pipe; omit for None, retain0.60 owns final byte accounting, whole-hit removal/zero-progress and schemas conditional on format. |
| Orient aggregate | Required vector_coverage:StoredHeadCoverageV1 on orient; native/public projection status exposes the same tagged contract. population=stored_mapped_spans; available has seven exact counters plus nullable exact rational shares, unavailable has only profile/population/status/reason. Count/byte/fraction components use checked decimal strings in JSON/TS and exact ints in Rust/Python; profile.dimension stays numeric (example8). Text coverage line is the same compact JSON. All details are fixed above; no new operation or fifth missing-capability entry. |

MCP shaping/budget cuts cannot alter e or translate it into rendered coordinates.
Re-snippeting calls the pure selector then restores this trusted metadata from
the same paired result; no storage read during rendering. Fetch/expand have no
scored field. Their body cuts are unrelated to a search's represented head.
60 text CR/LF flattening preserves byte length; stored-body offsets still refer
to original bytes, including role markers, while highlights refer to displayed
snippet text. Bytes are never UTF-16 character positions.

## Canonical amendments and verification boundary

Implementing90 updates MCP/interface/error/schema/guide examples and the exact
field census in60; amend EP17/match-centred-snippets and long-record §7.1's
historical generation-bump guidance to preserve existing truthful spans. Do not
reproject merely to obtain reporting or edit format35/37 storage history.
Stage2 SDK hints placement/default stays150; no new flags/operations/manifest
exception. Prior goldens are not regenerated. Apply quality-roadmap Unicode,
boundary/anomaly/real-snapshot and bounded-work methods; defer broad MC/DC,
allocation-failure campaigns and chunking experiments. No performance/quality
numbers or test passes are claimed by this source-only design.
