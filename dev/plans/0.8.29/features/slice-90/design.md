---
title: "Slice 90: bounded residual design"
status: PROPOSED
target_release: 0.8.29
slice: 90
---

# Slice 90 bounded residual design

The [requirements](requirements.md) implement the
[full owner-approved80 contract](../slice-80/design.md), accepted at
`f2f4831554ae165420a31890cf102642ce28b21d` (seq384).
[Census](census.md) reconciles integrated source
`3b9e02c151dd7308def6cf199c71902106edd837`:50 identity/feedback and60/70
contracts are implemented inputs. The original vector-participant set, aggregate
coverage/cache, converters and new MCP fields remain90 work. Actual Block1 exit
and independent updated-design PASS still precede source implementation; no
reapproval of80 is requested.

## Omission, diagnostics and default store

Use Option plus skip_serializing_if for unavailable-only Envelope.state and
SearchOutput.verdict, populated None. Preserve real completeness, readiness and
degradations. Add required orient.not_in_this_release=[state,verdict,kinds,counts].
Keep structured kinds/counts placeholders; their text omission already belongs
to60. Coverage is supported with an availability result, not a fifth missing
capability. No freshness identity or calibrated verdict is manufactured.

Build one immutable sorted accepted-key list per tool from the same schemars
input properties as discovery; fail construction if a schema cannot supply that
object, rather than silently using a partial list. Share this source with edge
registration and typed argument precheck, not duplicated literals. Validate
original top-level keys before deserialization/store/admission. Least original
unknown key wins, sorted before truncation. If its byte length>64, use the
longest UTF-8 char-boundary prefix≤61 plus U+2026; otherwise retain it exactly.
Known-only serde failures stay bad_argument and nested attributes are not tool
arguments. No fresh schema/accepted Vec/detail String on a successful call.

Extend ToolError with optional argument, accepted, refs_not_in_response and
refs_omitted; omit on unrelated errors. Fixed messages are “unknown tool
argument” and “used and unused may only name references from that response”.
Never interpolate request strings into message/remedy or include record body,
SDK message, host path or SQL. Diagnostic copies are only caller keys/refs.
This governs the new diagnostic branches; broader generic serde-error sanitization is not added to90 scope.

| Tool | Fixed unknown_argument remedy |
| --- | --- |
| list_stores | use hints to control suggestions; list_stores takes no other arguments |
| use_store | pass store and optionally hints; use a listed store name |
| orient | pass store and optionally hints |
| search | use only the accepted search arguments; continuation calls take only store, continuation, max_tokens, hints and highlights |
| fetch | fetch takes no limit; pass fewer refs, or max_tokens to bound the size |
| expand | use only the accepted expand arguments; lower limit or max_depth to bound work |
| feedback | pass response_id, used and optionally unused, store and hints |

Preserve feedback's existing parse→resolve store→ensure slot→response lookup
(wrong_store or response_expired)→membership→epoch→telemetry→text mode→query ID
→acquire/recheck epoch→record ordering. The membership diagnostic iterates used
then unused, preserving duplicate offending occurrences, rather than serializing
the BTreeSet. Include first50 offenders with raw length≤128; omit overlong refs
whole, increment checked u64 refs_omitted for them and every remaining offender.
No new successful duplicate/overlap policy or total input-size cap is inferred.
Foreign unsupported-looking refs remain membership failures, not fetch parse errors.
No partial feedback labels are recorded. Preserve implemented50
Unique(cursor)/absorbing Ambiguous membership: named ambiguity refuses atomically
with existing invalid_request/bad_argument only after acquisition/final epoch
check on an otherwise-recordable call. Earlier membership/epoch/telemetry/text/
query-ID outcomes retain precedence; unmentioned ambiguity is inert. Do not
introduce first/last selection, fanout, a feedback-ref ABI or another identity fix.

Structured errors serialize typed fields. Text appends argument=<JSON string>
accepted=<JSON array> or refs_not_in_response=<JSON array> [refs_omitted=<n>]
after60's existing fields. Errors have no hints/success budget. The diagnostic
bound is finite:≤6400 raw ref bytes,≤38400 JSON-escape bytes plus fixed framing,
complete finite accepted names and count. It is not a256-token-fit promise;
never truncate an identifier into another identifier or drop accepted keys.

Reuse list_stores' existing single session default sample for new required
nullable default_store, existing row flags and default hint. First text line is
`stores: N default=<JSON string or null>` independently of hints. use_store
retains its action/quoted renderer. Add one concise guide sentence: calls
without store use the session default; list_stores/use_store choose/switch it.
Keep current1200-byte guide cap. No provider/session default changes follow.

## Aggregate public value and exact arithmetic

Implement the full80 accepted carrier: required
ProjectionRuntimeStatus.vector_coverage:StoredHeadCoverageV1, re-exported through
engine/facade/SDK and converted by PyO3/NAPI/wrappers. MCP orient moves this
member to top-level vector_coverage exactly once, outside projection_status.
No operation/flag or SearchHit/SearchResult member is added.

Both variants require population=stored_mapped_spans and profile with
key=default, stored name/revision/dimension. Available requires seven values:
records, known_records, unknown_records, truncated_known_records,
known_body_bytes, unrepresented_known_bytes, unattributed_heads; plus nullable
truncated_known_share and unrepresented_known_byte_share. Unavailable instead
requires exactly reason=cache_cold or state_changed and no numeric fields/shares.
Profile/token decoding/SQL failures are errors, never cache misses.

Count attributable canonical node/edge mapping-head tuples under the one stored
default profile, including retained superseded/inactive/out-of-window records
and no-longer-vector-targeted declarations. Do not filter by current view/source
eligibility/runtime availability. A passing mapping with no canonical owner,
exactly one NULL-body owner, or both node/edge owners increments only
unattributed_heads. First apply the unchanged span predicate: exclusions cannot
hide its existing failures. This is not physical-vector health or a census of
all canonical/unprojected records. Unmapped physical vectors add nothing.

For admitted heads let R/K/U/T/B/V have [acceptance](acceptance.md)'s meanings.
NULL end is unknown, including empty input. Known0/length0 is known-empty,
not truncated. R=K+U; T counts known end<length; B sums length over K only and
V sums length−end over K only. Shares are exact unreduced(T,K) and(V,B), null
iff denominator0. No SQL SUM/TOTAL/REAL, float ratio, inferred threshold,
saturation or rounding. Stream checked u64 counters≤i64::MAX and u128 bytes/
fraction components. Validated per-row length≤i64::MAX and row count≤i64::MAX
bound byte totals below2^126; defensive checked failures stay typed errors.

Only aggregate counts, byte totals and fraction components use canonical unsigned
decimal strings in MCP JSON and NAPI/TS transport/public values. Rust retains
typed u64/u128 values; PyO3/Python expose exact ints and frozen snake_case tagged
values. Profile.dimension retains its existing numeric carrier in both variants:
JSON/TS number, Python int and the existing Rust dimension type, validated by
the existing profile contract. For example dimension8 remains numeric8, not
string "8"; it is outside the decimal-string counter rule. TS public names stay
camelCase with a discriminated available/unavailable union. Decoders validate
tags/required variant fields/reason/range/canonical decimals and count/fraction relationships;
malformed native data yields ProjectionGenerationError with reason
projection_generation_corrupt and exact RFC6901 path under /vectorCoverage or
/vector_coverage. Do not coerce counts/bytes/fraction components through JS
Number or float, or lose0 through truthiness; numeric dimension and scored-byte
scalars retain their own existing/approved numeric carriers.

## Existing-validation capture and O(1) coverage read

Preserve scalar vector_spans::validate(Some(cursor)). Add a private full capture
path returning fixed-size summary, not mutating Engine cache. Replace the healthy
full EXISTS result with a streaming mapping result carrying existing invalid
predicate, owner-presence, body_length/end and original orphan anti-join in the
same statement. Do not transport body strings/vector BLOBs/per-record Vec to
Rust. Preserve current COALESCE(n.body,e.body) length/boundary semantics and
original errors. No new NULL-body, dual-owner or missing-physical-vector refusal
at open/aggregate capture. Existing completion independently retains its broader
cross-owner/physical checks; a later failure publishes no staged summary.

Aggregate capture makes **no vector_default join, rowid/EXISTS probe, vector
column read or new physical schema check**. Pinned sqlite-vec point lookup loads
vector data even for rowid-only requests; an indexed plan is not a no-payload
proof. Per-hit validation is a separate bounded point path, described below.
Capturing adds new O(N) Rust row-transfer/arithmetic within an already-required
full pass; it is not free work. No second store-wide traversal or additional
full validation is introduced. Ordinary/explained reader searches use the
existing noncapturing path, not implicitly expensive new accounting.

At open.rs's existing full-span and immediately following frozen validation,
retain order/error maps inside one Deferred read transaction; capture token/
profile on that snapshot, commit and carry an optional seed through the existing
open tuple/guard to Engine. A genuinely not-yet-initialized fresh profile means
no seed; SQL/decode failures do not. Later open repairs/profile/generation/mean/
backfill work can change state. Compare the seed again on the same writer
connection at Engine construction before runtime workers; discard unequal seeds,
never retag old totals or rescan. Stable reopen normally seeds; fresh/repaired
open may stay cold.

Existing writer full generation-status refresh misses may capture via private
status result. Stage summary/token alongside completion; publish only after
successful transaction commit in its actual owning caller, including mutation
status. Generation-cache hits may reuse only independent already-committed
matching coverage entry; a miss does not force a scan. Reader/explain direct
status callers cannot seed writer cache. Add no refresh API, task or explain=true.

Engine owns one Mutex<Option<CachedStoredHeadCoverage>>, scoped to its writer
connection. Entry holds summary/profile/token only, no body/ref/record list.
Lock order: writer connection then coverage cache; clone/release cache lock
before SQL. No reverse-lock path or worker mutable handle. Token loads on one
pinned Deferred transaction: database ID; user_version37/schema_version;
stored default-profile name/revision/dimension; visibility generation/state_nonce
and exact clean coalescing0; same-writer main.data_version and total_changes.
Validate scalar domains. Direct singleton/key/PRAGMA reads only: do not call
frozen binding/current_generation/registry_digest for this token. Connection-local
data_version is never compared across engines/readers and never substitutes for
nonce/total_changes. Retain the accepted trigger-governed external-write model.

read_projection_status uses a mutable writer guard and Deferred transaction,
loads current token/profile plus existing status facts, then clones matching
summary or returns cache_cold/state_changed; commit before returning. Added
coverage reads are O(1) in record/vector count, plus profile string length.
Existing registry/readiness work and orient's other two SDK reads are not
claimed O(1) or one shared snapshot. Serialization does no database work.

DML/external commits/projection tuple writes/profile/schema changes/rebuild/
erase/recenter invalidate conservatively. Rollback may advance total_changes,
so state_changed is truthful even with unchanged committed facts. Uncommitted
candidate/commit failures publish nothing; post-capture commit is caught at next
token comparison. Erased stale numbers never serve new state. Reopen gets a new
cache. Time-only validity transitions do not change this stored population.
Cold/state-changed may remain unavailable indefinitely until existing full
status validation or reopen; no always-current availability guarantee is inferred.

## Per-hit scored extent and conversions

The accepted field is SnippetV1.scored_bytes:Option<u32>, never SearchHit growth.
Reuse existing50 FusionHit/FusionOwnership and private RecordKey, distinguishing
CanonicalNode(cursor), CanonicalEdge(cursor) and OwnerlessNodeFts(rowid). Collect
original vector_results keys only when snippets are requested, before the first
consuming branch (vector-stage-only, ordinary, deferred hydration or nested graph
fusion). After final ranking/cuts, attach extent only when that same canonical
selected key belonged to the original vector arm. Entry.in_vector is only
current-input tie preference; nested graph fusion cannot supply this proof.
Body/ref, TextEdge branch, explanation ranks or CE do not prove participation.
No query-provider/tokenizer recomputation or later logical fetch is allowed.

Attach metadata after snippets_for_hits has selected the positional snippets,
before capture.finish strips ownership in NoEvidenceCapture and before tx.commit.
Existing evidence requests intentionally return snippets=None; preserve that
separate API. Use existing SearchReaderError::Evidence for EngineError mapping.

On that same reader transaction, perform at most one indexed statement per
final canonical vector participant. Use both mapping/span cursor and rowid
identities to expose misplaced/extra heads, without LIMIT1/best-head selection.
Validate exactly one canonical owner/non-NULL body matching the selected
carrier, exact mapping rowid=write_cursor=selected cursor, exact span identities,
head0/start0, integer nonnegative body_length matching original stored bytes,
physical vector identity/presence and nullable integer end at a UTF-8 boundary.
Do not compare representative edge_fact kind with stored kind. Missing mapping/
span/physical vector, ambiguous owner, extra/misplaced/nonzero head, mismatched
body, malformed end, invalid boundary or known end exceeding u32::MAX returns
the existing typed Storage/projection_generation_corrupt error. Validate the
whole tuple before treating NULL end as unknown. No partial successful sidecar,
repair, full-store validation, clamp, wrap or unknown fallback is allowed.
This bounded physical check must not leak into aggregate capture. Validated provider NULL or nonparticipant givesNone; known0 staysSome(0).
Ownerless FTS makes no span read, even for a numeric rowid/cursor collision.
Standalone snippet_for and text-only path giveNone without storage work.
Snippets-off adds no participation set/coverage query/sidecar allocation.
Frozen validation/refusals remain before retrieval; reader snapshot owns facts.

Rust re-exports preserve opt-in Some([])/None sidecar rules. PyO3 getter/dataclass
scored_bytes:int|None=None maps explicitly; NAPI optional integer and TS
scoredBytes?:number preserve0, validating dynamic integer0..u32::MAX. MCP copies
aligned sidecar to optional Hit.scored_bytes; text adds scored_bytes=<n> before
pipe, omitted atNone. No fetch/expand scored field or new switch. Re-snippeting
may use pure selector then restore paired trusted metadata without storage reads.
Role markers remain part of original stored UTF-8 body coordinates. Windows/spans
are body-relative, highlights snippet-relative; CR/LF flattening changes neither
byte length nor endpoint. Selector anchors/shape/ranking remain as currently built.

Orient text adds one `coverage: <compact JSON>` line using the exact structured
wire object. No auto advice/hint/weak classification follows. Extend60 final
format census/schema/accounting; mandatory metadata may cause its existing typed
refusal, never omitted coverage to fit. Search shaping drops whole hits without
changing endpoints and recomputes kept handles/continuation. Preserve hint fallback,
zero-progress and no-oversize-success contracts. Keep actual integrated70 words.

## Contract CRUD and fit review

Same implementation PR updates maintained Rust/RustSDK/Python/TypeScript/MCP
interfaces, MR-024/031.1/050/071, EP17/snippet and long-record guard reporting
wording. Correct historical “new table/generation bump” guidance in maintained
prose to existing37 spans; do not rewrite historical ADR/storage SQL. No storage
migration, rebuild/backfill, new governed operation or embedder ABI bump.

Use quality-roadmap boundary/property/anomaly/reference methods at token,
UTF-8, serialization and commit seams. Defer global MC/DC, generic SQLite fuzz,
allocation campaigns and chunking studies. All new measurement/operator helpers
must be committed, rerunnable with args/env and indexed before their evidence
is used; only logs belong in scratch. This design refresh creates no helper or
runtime proof. Local pinned serde/schemars/sqlite-vec source should answer
any concrete implementation uncertainty before dependency/network research.

## Final-source qualification dependencies

130E and independent160 platform/consumer qualification can proceed in parallel
with the actual Block1 prerequisites.90 consumes neither whole160 nor130E as
a source-entry gate. Final90 must join the assembled-source qualification: its
Rust/native fields/converters affect distributed binaries and installed Python/
Node/strict-TS consumers (160 Q14/Q15), and its default MCP success/error/schema/
budget output affects Q11 and Q12 smoke/item4 observations. Pre90 Block1 results
remain real prerequisite evidence for their baseline, not automatically proof
of the changed final output. After implementation, assess exact source/artifact/
format/workload applicability and rerun only invalidated scopes, including
new-field/budget/paired-smoke assertions where affected. Retain applicable
unchanged prior results; neither a blanket full replay nor paid-call permission
is inferred. Parent controls any required model run under its actual approved
models, episode/budget limits and current technical readiness.

The current completion route is local qualification, review CI, then run CI.
Registry publication, publishing tags and post-publication smoke remain outside
this assignment.
