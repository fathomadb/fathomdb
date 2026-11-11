---
title: Slice 70 — bounded outcome and public-carrier design
status: PROPOSED
target_release: 0.8.29
slice: 70
---

# Slice 70 bounded design

Scope follows [requirements](requirements.md); this is source-grounded design,
not a product verification receipt. Initial behavior witness was product
`6edc1155f039f87cd0a37660cb27ae4373bbce71`. Reconciliation witness: clean parent `2f6913c1d59ac8ff3a30de835b57290a8635998c`,
including completed10 source104dcbc/closured7d928 and independently reviewed20
Linux subset657edeb76. Parent reports independent Astra PASS for actual20 fit, strict historical
successor and actual60 candidate fit. Consolidation is proposed for consistency
recheck, with no source authorization. Actual60 product/test pins at307c remain
identical at candidate7038; only reviewed non-product operator packaging differs.
See [source pins](source-pins.json) for exact objects and exceptions.

## Actual source-entry reconciliation

Actual entry589467 and integrated qualified partial60 subset5271 are recorded
in the [plan](plan.md#actual-source-entry-reconciliation) and [status](status.md).
All22 historical product/test pins match; approved scope, exact oracles and
runtime RED ordering stay unchanged. Same-reviewer baseline-fit recheck precedes
implementation; only owned setup may run while that review is pending.

## Built behavior versus proposed delta

Paths below are repository-relative. This pass reread EP03 byte/overload, readiness engine/Python/TS, foreground/CUDA
and native degradation module bodies, wrapper EP03 bodies, relevant snippet
entry/conversion bodies, facade/SDK exports and exact20 source delta. The prior
EP17/selector suite census is retained as inherited inspection, not a fresh run
or claim of rereading every wider test. Large source owners were traced at the
relevant helper/entry/conversion bodies.

| Owning source | Built fact | Bounded delta |
| --- | --- | --- |
| engine `embedding.rs` | Typed CUDA kinds precede the helper; panic resumes; saturation/queued expiry label overload; remaining dispatch errors become None. Remedy list still includes caller injection. | Add absent/started-timeout reasons, make actual Closing an error, defer non-close Cancelled unchanged, remove unsupported remedy. |
| engine `search_api.rs::search_reranked_view_internal`; `search.rs::read_search_in_tx` | Ordinary query embeds before reader dispatch; frozen query embeds after snapshot validation inside the reader. Exactly one side embeds; frozen degradations are appended and expanded results clone the list. | Keep one reason from the common helper, preserve snapshot/filter/argument precedence and existing expansion early-return propagation. |
| engine `search_types.rs`; py/napi `read_search.rs` | One enum reason; native conversion uses as_str and omits empty lists. Real gated-provider overload conversions already tested. | Extend closed reason words and real new-outcome conversion controls; no result-field redesign. |
| facade `src/lib.rs`; SDK `src/lib.rs` | SDK already exports the six needed hit/identity/snippet types and snippet_for; facade lacks those six types. | Export exactly six types through facade. Keep SDK operation surface independent. |
| py/napi `errors.rs`, `projection.rs`; Python `read.py`; TS `errors.ts`, `read.ts` | Typed blocked remedies travel intact through error/readiness payloads. | Fix the central list, assert end-to-end native-installed readiness/drain payloads. |
| engine/native snippet owners; Python `types.py`; TS `search.ts` | Five options and Unicode selection/byte slicing already implemented; TS/NAPI docstrings say `[start, end]`. | Correct half-open prose and explicit canonical interface wording. No duplicate selector implementation or invented missing field. |
| MCP `tools.rs::search_degradations`, `hints.rs::cause`, `output.rs::HintCause` | Adapter copies engine words, adds soft_fallback and sampled embedder_unavailable, canonicalizes; cause conversion lacks new words. | Minimal exact new cause/hint translation; parent coordinates60 shared files. Retain reviewed sampled epoch label; apply only linked exact oracle successors. |

## Outcome table and precedence

| Winning dispatch outcome | Search outcome / metadata | Direct embed outcome retained |
| --- | --- | --- |
| Ok(vector) | Existing hybrid result, empty new labels. | Vector. |
| NotConfigured | Sparse result, exactly Vector/EmbedderAbsent (`embedder_absent`). | EmbedderNotConfigured. |
| Saturated or QueuedExpired | Sparse result, existing Vector/EmbedderOverloaded. | Overloaded. |
| StartedTimeout | Sparse result, exactly Vector/EmbedderTimeout (`embedder_timeout`). | Embedder. |
| Provider(CudaPoolExhausted / CudaContextLost / CudaPrivateBuildRefused) | Existing exact typed EngineError; no result/degraded success. | Same exact typed error. |
| Provider(Failed / Timeout / other non-CUDA), InvalidOutput | Existing sparse fallback and empty new reason list. Broader failure taxonomy is unallocated. | Existing Embedder mapping. |
| Closing from actual dispatch shutdown | Proposed EngineError::Closing; no result. | Existing Closing. |
| Non-close Cancelled | Unchanged unlabelled sparse fallback; deferred from70 correction. | Existing direct-embed mapping unchanged; not authority for a broader search contract. |
| Timely Panic | Resume panic at current owner: ordinary caller unwinds; frozen reader unwinds and public call returns Storage. | Existing caller unwind. |

Keep dispatcher `ReplyState::resolve/start/wait` arbitration unchanged: a finished
outcome wins once; deadline uses queued versus started phase. A close/provider
completion that loses to the deadline does not override its outcome. Late panic
or result is accounted by the existing worker and does not retroactively replace
the timeout. Started work retains its provider slot until it really finishes;
no replacement worker, cancellation preemption or new timeout budget.

Keep existing `search_query_vector` as the Result boundary and handle actual
Closing before the sparse helper. A cfg(test) module in `embedding.rs` can clone
its real Engine's existing `embed_dispatch` Arc and invoke this existing helper
while actual Engine.close resolves its held request. Baseline compiles and
returns Ok(None), so no production factoring or new test helper is needed to
obtain genuine runtime RED. See the exact sequence in [test map](test-map.md).
Keep taxonomy centralized. Frozen errors already travel through
SearchReaderError::Evidence without erasing their EngineError type.
Actual engine close can independently refuse reader dispatch; tests must reach
the changed mapping rather than pass because a later close check already fails.
Cancelled originates in EmbedReply::cancel/Drop; synchronous search consumes
its reply into wait. Reply abandonment can happen while the Engine stays open,
so it is not evidence of shutdown. Current Engine/PyO3/NAPI Closing messages say
"engine is closing" and MCP maps that category to not_ready/draining. The accepted
runtime ADR authorizes close cancellation only; direct embed's current mapping
does not authorize calling every internal cancellation shutdown.

Therefore non-close Cancelled remains unchanged: the query helper returns None
without a new reason, as before. Defer any new cancellation mapping until a
truthful accepted contract and reachable production condition are established.
No broadened Closing wording, new cancellation API or private mapping fixture
is used to assert public reachability. This deferral does not block the bounded
actual-Closing correction, absent/timeout reasons or the remaining70 scope.

## Public contract details

- Extend DegradationReason and Python/TS unions with exactly two words. Rust
  Vec remains always present; native empty→None and wrapper None/absent rules
  remain. Ordinary/frozen/evidence/search-expand use the same meaning.
  Correct interface claims that an empty list certifies every requested arm ran:
  this deliberately partial taxonomy still leaves ordinary failures unlabelled.
- Remedy order becomes `configure_default_embedder`, then
  `submit_non_embedding_input`; code/operation/blocked state/URL stay. Configuring
  the built-in runtime is a supported next-open choice, not live reconfiguration.
  Supplying non-embedding input avoids new embedding work; it does not clear
  already-pending work. Semantic extraction/consolidation transport is separate.
- Exact facade delta: SearchHit, IdSpace, IdSpaceKind, SnippetOptions,
  SnippetAnchor, SnippetV1. No pure helper addition is needed to name fields.
  Existing facade/SDK governed-surface tests and recovery absence proofs stay.
- Snippet defaults `(300,1,0,false,20)` and ranges remain: max_bytes40..4000,
  max_windows1..3, head_bytes0..floor(max_bytes/2), bool highlights,
  max_point_queries0..255. All ranges are `[start,end)` UTF-8 bytes;
  windows/spans index body, highlights index rendered text including ellipses.
  Python uses encoded bytes; TS uses Buffer slicing, never UTF-16 string indexes.
- Same implementation PR updates maintained Rust/RustSDK/Python/TS interfaces,
  affected MCP vocabulary and TS/NAPI/native public docstrings. Interface updates
  record this bounded enum/error/remedy/export change; EP03 stays partly proposed.
  Correct active EP03 built-state claims only for the landed subset, retaining
  its broader proposal. No implied ADR acceptance for its remaining reasons.

## MCP engineering selection for independent review

Proposed70 carrier edits add EmbedderAbsent/EmbedderTimeout to engine HintCause,
MCP output HintCause and cause parsing/from_engine.60 renders supplied words
in its own reviewed formats and retains budgets/continuations.70 tests both
formats on the actual integrated60 build. No SDK hints option/default flip.

Retain the current sampled `embedder_unavailable` label on a None open. It states
an open-epoch capability fact; the new embedder_absent states this query attempt
failed for absent configuration. Both may occur, with their different provenance
explained in the interface. Removing the built label would widen the adapter
contract change beyond these bounded reasons. This retains existing behavior,
not a compatibility shim or a new provider admission route. New words sort
canonically; with both labels, embedder_absent sorts first and the built degraded
hint names that first entry. Neither epoch readiness nor a hint certifies runtime
health. This bounded selection and its exact oracle successors received parent-reported
independent Astra PASS; no broader engine/provider semantics are inferred. No `dense_disabled` production path returns.

## Fit and limits

Apply [quality-roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
provider anomaly/cleanup controls and existing Unicode properties. Defer general
protocol fuzzing, resource campaigns, GPU reset/soak and automatic weak-result
classification. Positive CPU/native checks do not qualify GPU faults/platforms.
Fresh source/native identity, reviewed design and isolated owned setup are hard
execution prerequisites;50 final identity/60 final carrier integration must be
reconciled without changing the healthy EP03 human-intent byte oracle.

## Source-backed gap classification at actual20

| Item | Current body / reachability | Minimal action |
| --- | --- | --- |
| Missing absent label | `embedding.rs::query_vector_or_degradation` catch-all consumes NotConfigured; public None hybrid routes call the helper even on a vectorless store. | New reason from the common helper; text-only remains separate and empty. |
| Missing timeout label | Same catch-all consumes actual StartedTimeout; overload fixture's held search explicitly expects empty labels. | Separate observed-start RED, including that approved predecessor expectation change before GREEN. |
| Swallowed close | `runtime_lifecycle.rs::close` invokes real dispatcher.close before quiescence; stop resolves active replies Closing. Helper consumes it. Ordinary search may later fail reader dispatch regardless. | Test existing helper against that actual shutdown; add only Closing arm at existing Result boundary. |
| Non-close cancellation | EmbedReply.cancel/Drop resolves Cancelled; synchronous search consumes its reply into wait. No newly established public non-close cancellation path. | Defer unchanged unlabelled fallback; never infer shutdown from abandonment. |
| Unsupported remedy | `embedder_required_for` centrally emits three actions including caller injection; real public readiness/drain tests pin all three. | Exact two-value remedy list, same operation/privacy/pending-state semantics. |
| Native/wrapper carriers | Native conversion stringifies reason.as_str; wrapper maps cast/copy strings. No separate transport defect demonstrated. | Stage real new-outcome native/installed tests before reason GREEN; extend public unions. No duplicate passing stand-ins. |
| MCP hints | `hints::cause` rejects new strings; output HintCause lacks variants and from_engine mappings. Existing `degraded` accepts string DTOs and returns None today. | Two separate runtime mapping pairs for new words. Real MCP None case joins absence pair after integrated60 recheck. |
| Six facade types | Exports missing in facade, present in SDK. This is compile reachability, with no runtime reflection of export aliases. | Retain separately approved compile RED; do not label it runtime evidence or invent a policy waiver. |
| Snippet parity | Five fields/defaults, checked native search conversion, clamped pure selector and UTF8 slicing exist. TS/NAPI prose alone falsely uses closed interval. | Docs-only correction; reuse authored fixture/properties. No demonstrated selector behavior defect or new passing coverage pair. |

## Actual predecessor fit and execution limits

Actual20 adds engine-owned sidecar/resolver hooks to Engine/open, evidence-only
request/capture, and test_hooks with required Engine arguments. Read its exact
diff before editing shared search bodies; preserve handles, mutex release before
callbacks and existing transaction positions. Current root census48 and boundary
map account for those two fields. Query helper, DegradationReason, native/wrapper
conversions, facade exports, snippet selectors and PINNED bytes remain unchanged.
20 is partial: A3/A5 policy unanswered and Windows A6 held. No20 closure or30
prerequisite is inferred for70.

Use completed10 ownership modes and exact canonical candidate/child identity;
provision only parent-created own checkout after source GO. Original10/20 failures,
reruns and skipped cells do not qualify70.60 is active elsewhere with sole native
window; this set consumes pinned actual candidate source/tests and committed evidence,
not its mutable qualification outputs. Actual integrated60
recheck precedes70 source and fixes the both-format carrier test placement.50
identity and80/90 gates remain separate. Preserve F11's unanswered feedback choice.
120 alone owns selected-major CUDA remedy; retain exact hard CUDA payloads without
freezing current12-only installation advice as a universal contract.

## Canonical actual60 oracle amendment

The [oracle inventory](oracle-successors.md) and [pinned illustrative diff](oracle-successors.proposed.diff)
are part of this design. They change exactly five current oracle owners:
search.rs real None SDK/control and authored mandatory-envelope expectations;
hints.rs canonical list/first cause under both formats; tests in tools.rs retaining
real absence before synthetic overload; closed-cause schema validation; and the
strict historical comparator's guarded row4 amendment. Production mapping remains
exactly the reviewed two words plus existing sampled label. No blanket reason
ignore, old/new transitional acceptance or regenerated JSONL is permitted.

The [historical successor](historical-oracle-successor.md) fixes the independent
row4 reference:2735 JSON bytes, used1353, two-copy1368<4000. Six hits/ranks/scores,
trace/handles/completeness and cut=false remain exact. The mandatory-overflow
oracle independently adds the same pair only to hybrid; real SDK None control
requires absent alone and retains40 text hits/unchanged trace. It never derives
its reason oracle from actual MCP output or calls the product shaper.

Stage these exact successors and real engine/native/installed reason assertions
in the absence RED before the shared GREEN. [Execution map](execution-map.md)
splits absent and timeout closed-cause/schema runtime pairs before engine labels.
Generic wrappers already copy strings; no new passing stand-ins. Direct text,
F12 route/filter counters, synthetic explicit no-degradation shaping specimens
and all14 durable manual metadata cases retain their actual existing assertions.
Historical307 qualification outputs/provenance are immutable. No GPU/model rerun
or broad gate is created by this reconciliation.
