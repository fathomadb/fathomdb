---
title: Slice 80 current source and authority census
status: PROPOSED
target_release: 0.8.29
slice: 80
---

# Slice 80 census

Read-only source boundary: `5e4b5b61765aa9ce2557b2349218f44e74803f05`,
branch llm/0.8.29-s80-data. Only this checkout's ten owned Markdown files change.
This census is source/test inspection, not runtime qualification. No database,
native import/build, model, benchmark, fault or product test was run.

## Integrated inputs and actual limits

| Input | Retained integrated evidence | Limit consumed by80 |
| --- | --- | --- |
|10 |104dcbcbe92a1eca730d146b99ab17d444df6217; closure d7d928f8e49d7af6a792385534f4e991c13232b3 | Preparation only. |
|20 |Linux subset657edeb76caaf9c8d2aab433766bec9362e73217 | Whole20 A3/A5 policy and Windows A6 open. |
|30 P |Runtime2fd772558e46d0b1645dce2ffea730d0bdd3305c; final docs0d8be5bb75516955ebf99b94913b9b54921bb3a4 |16cells/20000timed successes, provider-free direct text, zero vectors. Assembled qualification preserves original failures/resumptions/skips; not G/D/M/F, TC5 or whole30. |
|50 |Documentation/proposed identity ADR integrated at5e4b5b617 | Corrected design and different final DOC review passed; all identity/provenance/feedback/helper-pack/helper-export choices remain PROPOSED/HITL-required. No source handoff or runtime census; hard30 open. |
|60 |Formats/direct-filter/budget5271c1bf022f0ea22f95f7433e226feb2315043e | Qualified integrated subset, not an active sibling candidate. Assembled214/214 canonical suites and native subjects retain original307 wrapper failure and skips; whole60/F11 open. |
|70 |COMPLETE e875f36042f89c1d84756f78135db0e8926c1ed4 | Implemented bounded reasons, remedies, hints and native/facade parity. Assembled214/214 suites, actual native identity/lifecycle and distinct final review; original failures/resumptions/skips remain explicit. Not whole20/60/F11 or Block1 exit. |
|100 |Reviewed documentary checkpoint3fa16620ac30cbe3ed66f5bfe542f2f7a62b8982 | Three H/P/W post-crash predicates and its narrow coverage exception remain unanswered. No80/90 exception. |

Retained source of these qualifications: ../slice-30/results-p.md and execution.md,
../slice-50/status.md and review-log.md, ../slice-60/status.md and execution.md,
../slice-70/status.md and review-log.md, plus the tracked release-state JSON.
80 does not rerun or expand their subjects. G/D isolated49be is unqualified and
quiescent below16GiB heavy admission; no G/D source is used here. /mnt/data is
mounted; the exact frozen TC5 pack remains missing on a separate cold volume.
Runtime consumer census scope is approved, but consistent snapshot/method is
unresolved. Callsite inspection establishes no runtime prevalence or permission
to choose an access method.

Historical bounded Phase A, aggregate-v2 PASS (proposal SHA-256
`a5e4851df2fedcbcce3e7d1b93138980c25e172d0ceb802792d218940bca0e53`)
and consolidation consistency PASS remain in [review log](review-log.md) and
release state. Bounded-v1 design PASS and different fresh FINAL DOC/OUTPUT PASS
on parent diff709240de are recorded in [review log](review-log.md), with actual
parent scoped checks and78-pin audit. V2 changes records/citation only, with no
separate v2 PASS or full80/source/runtime/resumption GO. No tracked root
manifest.json exists. A historical temporary manifest is not repository evidence.

## B2 implementation census

Paths below start at src/rust/crates/ unless stated otherwise. Relevant complete
functions and named test bodies were read, including capped-output remainders.
The frozen task manifest pins source/test blobs and whole-file digests; it is
an inspection receipt, not evidence that any suite ran here.

| B2 row | Built source and existing tests | Missing or unqualified residual |
| --- | --- | --- |
|1 |mcp output.rs requires Envelope.state and SearchOutput.verdict; tools.rs constructs unavailable placeholders. Integrated60 edge.rs removes outputSchema/structuredContent in text mode; renderers omit unavailable state/verdict and orient kinds/counts. formats.rs and field_census.rs pin this actual split. | Structured optionality/absence and orient.not_in_this_release are still missing. Text omission is built and a regression control, not a new RED. Both formats must preserve real readiness/degradation. |
|2/3 |engine hints.rs is pure; mcp hints.rs/tools.rs build empty/degraded/cut hints. Full hints.rs tests pin empty_not_ready, ordered absent then sampled unavailable, first-cause embedder_absent hint, bounds/precedence/budgets/quoting. schema.rs admits absent/timeout causes. | No demonstrated new hint producer gap. Preserve70's bounded taxonomy and both-format mappings. No weak_results, new readiness inference or calibrated verdict. |
|4 |mcp tools.rs parse uses deny_unknown_fields and generic invalid_request/bad_argument; edge.rs input<T> uses schema_for_input. schema.rs checks unknown inputs but has no typed argument/accepted detail oracle. | Deterministic bounded unknown_argument, full schema-derived sorted accepted list and fixed remedies. Nested maps retain current semantics. |
|5 |tools.rs feedback validates visible membership before epoch/telemetry/text/query-ID guards and rechecks epoch after acquisition. session.rs stores BTreeMap<ref,u64>. Full feedback.rs pins unique attribution, concurrency, store/ref/epoch/sink/no-result-effect. | Typed ordered bounded offending refs are missing. Current map collapses repeated refs and cursor aliases;50's corrected P-ref/R-cursor guard remains a proposal, separate from this presentation change. |
|6 |list_stores samples default once, sets row flags and default_store_set hint; use_store and store-less routing are built. stores.rs/hints.rs pin session isolation, switching, hints on/off; formats.rs pins quoted names/use_store and current first line. | Required nullable top-level default_store, quoted first-line default even with hints=false, and clearer guide sentence. Do not rebuild60 quoting. |
|7 |registry.rs/bridge.rs retain FIFO wait, permit ownership, run-to-completion/abandonment. Complete admission.rs covers32waiters, queue_full/deadline, overload schema, progress/cancel/shutdown/stuck and no retry_after_ms;60 error renderer preserves busy detail. | Both-format regression qualification only; no demonstrated new queue defect or retry field/action. |
|8 |schema format35 span table survives37; vector_spans.rs validates tuple/length/UTF8 with COALESCE and orphan check; projection_commit.rs publishes mapping/span/vector/terminal transactionally. Complete slice60_store_format.rs pins unknown/known offsets, invalid/corrupt spans, rebuild/recenter/batch fallback/erasure/rollback. | No StoredHeadCoverageV1/vector_coverage/scored_bytes exists. Reporting and exact per-hit attribution are net-new conditional work, not span-storage rebuild or format change. |

70's actual engine query_vector_or_degradation reports exactly absent for
NotConfigured, timeout for StartedTimeout and overloaded for Saturated/QueuedExpired.
Provider(Failed/Timeout)/InvalidOutput preserve unlabelled sparse fallback; Closing
is an error and timely panic retains its existing propagation. Complete
slice70_query_degradations.rs pins ordinary/frozen/evidence/expand absence and
started-timeout values, seeded lexical IDs/score, slot retention, text-only zero
provider calls and provider-error controls. An empty degradation list does not
certify all arms ran. Preserve sampled adapter embedder_unavailable separately.

## Aggregate source boundary

Current ProjectionRuntimeStatus contains runtime_embedder_available,
runtime_unavailability_reason, projections and vector_unsupported_kinds only.
read_projection_status holds the Engine writer connection and loads registry/
readiness; it does not call the full generation-status validator. Orient makes
three separate SDK reads. Existing whole-envelope snapshot atomicity is absent.

vector_spans::validate(None) uses SELECT EXISTS over the exact invalid predicate
and orphan anti-join. open_locked calls it before frozen_read::validate_on_open;
physical_completion calls it before provenance/cross-owner/completion checks.
The validator does not inspect physical vectors, reject NULL canonical bodies,
or itself apply completion's cross-table owner refusal. Preserve that exact
COALESCE predicate and each caller's error scope. Reporting must not add a
physical-vector health claim or blanket open refusal.

The historical aggregate-v2 contract remains concrete: stored_mapped_spans;
seven counters records/known_records/unknown_records/truncated_known_records/
known_body_bytes/unrepresented_known_bytes/unattributed_heads; nullable unreduced
rational shares; profile.dimension numeric u32 (example8), count/byte/fraction
components decimal strings in JSON/TS and exact ints in Rust/Python. Passing
ownerless/bodyless/dual-owner mappings increment only unattributed_heads.
Unknown-empty and known-zero stay distinct; no-vector/pending/failed without a
committed mapping are outside the population. No zero/full inference.

Writer-only staged capture is proposed during already-required full validation,
committed before publication. Only same writer connection and matching database/
profile/schema/clean visibility generation+nonce/data_version/total_changes may
reuse numbers. Rollback after attempted DML invalidates conservatively. External
commits invalidate; old reader snapshots never seed/consume the writer cache.
Cold/changed output carries no numbers. Stable reopen or existing full status
refresh may seed; orient adds fixed token work, not another validation scan.
These are proposed implementation requirements, not a cache that exists today.

Pinned sqlite-vec0.1.9 vec0Filter_point's payload-reading behavior is retained
historical source evidence from aggregate-v2; this census does not run it or
borrow a mutable dependency cache. No aggregate or per-hit reporting-driven
vector_default rowid/EXISTS/payload probe is authorized by an indexed plan.

### S80-F1 — canonical attribution multiplicity needs bounded review

Mapping/span uniqueness proves one physical head per cursor, not one canonical
row. Schema migration step_id:2 declares canonical_nodes/edges.write_cursor NOT NULL; ordinary
cursor indexes are nonunique. Step33's unique node (kind,cursor) index applies
only WHERE logical_id IS NOT NULL, not globally to cursor. The current span
validator joins n/e by cursor and can admit same-length, valid-boundary bodies
from more than one same-table owner. This is a source-derived construction,
not a runtime reproduction or product-defect finding.

A naive streamed joined-row aggregate can therefore double-count one mapping.
[Design](design.md#canonical-attribution-multiplicity-source-hold) records the
candidate correction/hold: preserve validation for every joined row, classify
one physical mapping once, and prove exactly one canonical owner/body before
including it in R/K/U/T/B/V. Broader ambiguity belongs outside those denominators;
its grouped MIN/MAX owner-cardinality stream requires current
bounded review. Managed writers allocate distinct cursors per batch row;
that is not a schema/validator guarantee for all admitted rows. No new validation/refusal rule, schema or silently chosen row.
The historical bounded PASS is not erased or extended to this newly found case.

## Selected-record sidecar and actual carrier work

SearchResult.snippets is opt-in and positional; SearchHit has no FTS locator,
artifact class or vector-participation bit. SnippetV1 is non_exhaustive and has
text/anchor/windows/spans/highlights/cut only. search.rs constructs the sidecar
on the same transaction before commit; snippet_query.rs bounds lexical work to
2*max_point_queries. Complete snippet_search.rs includes alignment/default-off,
Unicode, node/edge/vector, ordinary/frozen/query_only and exact38-query controls.
Complete snippet_cost_receipt.rs separates measurement-correctness controls from
its ignored sweep; search_result_shape.rs checks existing explanation/fallback.
None asserts scored extent. No timing or existing test pass is newly claimed.

Current fusion.rs still keys equal bodies; explanation rank/score maps are body
based. Actual loop order is vector→text→graph despite an older contrary comment.
Vector edges can carry TextEdge. filter.rs returns both NodesFts rowid and stored
write_cursor; snippet_query::FtsRows is HashMap<cursor,(table,rowid)> and can
lose the locator under aliases.50's proposed private RecordKey/locator/membership
handoff and helper-export retirement are not built/accepted. A bare cursor,
body, ref or display branch cannot lend another selected record a span/source.

The corrected50 feedback proposal requires one ref occurrence in visible P and
one cursor occurrence in full captured R for exact query_id. Off-page same-ref/
different-cursor is safe; same-cursor/different-ref is unsafe. Current telemetry
still stores cursor result_ids/arm_of and cursor labels; it gains no new identity.
No occurrence ABI or public carrier is selected here.

Facade and SDK already export SnippetV1 after70. PyO3/NAPI snippet.rs copy each
existing field; read_search.rs carries the aligned list. Python types.py and
_sdk_search.py build dataclasses; TS search.ts mapNativeSnippet and binding.ts
explicitly declare/copy fields. MCP search_page copies snippet values before
whole-hit shaping. Complete native snippet modules and Python/TS EP17 test files
pin original selector/golden fields; preserve their human fixture bytes and
compare the original projection when adding a future optional field.
Coverage requires explicit changes at every boundary; adding only Rust is incomplete.

Aggregate runtime-status conversions in Python read.py/TS read.ts are separate
from generation-status decoders. Coverage tag/numeric validation is net-new;
existing generation-status enum error convention can be reused, not misreported
as an existing coverage decoder. Existing CS64, format37, W0.8.29/E0.8.0,
ranking/defaults, compact text default and embedder-owned vector identity stand.
Physical erasure remains DONE except actual defects. SDK Stage2 placement/default
is conditional150; full80 and actual pre90 Block1 exit stay held;160 is downstream.
