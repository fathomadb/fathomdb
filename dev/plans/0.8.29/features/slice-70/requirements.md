---
title: Slice 70 — bounded degradation and parity requirements
status: PROPOSED
target_release: 0.8.29
slice: 70
---

# Slice 70 requirements and local acceptance

The [Slice9 Batch4 ruling](../../prework/slice-9/decision-log.md) approves bounded
corrections through the [direct execution gates](../../direct-execution.md).
This contract reconciles completed10, integrated20 Linux repairs and integrated
qualified partial60 subset5271 at assigned base589467. All22 historical source
pins match. Parent remains frozen589467. The same independent Astra baseline-fit
recheck passed before implementation; owned isolation, setup and staged runtime
pairs have executed. [Status](status.md) records exact evidence and remaining
qualification/review limits; whole20/60/F11 closure is not inferred.
IDs below are release-local; canonical `dev/acceptance.md` remains locked.

## Actual source-entry reconciliation

Actual entry589467 and integrated qualified partial60 subset5271 are recorded
in the [plan](plan.md#actual-source-entry-reconciliation) and [status](status.md).
All22 historical product/test pins match; approved scope, exact oracles and
runtime RED ordering stay unchanged. The same independent Astra baseline-fit
review passed with zero material findings before implementation; see the
[review log](review-log.md). Executed pairs and outstanding qualification are
recorded in [status](status.md) and the [execution map](execution-map.md).

## Need and authority

N29-A/K → R29-REL-A/K → R29-3P-12 / O3-12 / D29-SDK in the
[completed allocation](../../prework/slice-3/allocation.md) require truthful
per-call reports, supported remedies, nameable Rust types and byte-coordinate
parity. Preserve canonical REQ-052/053, AC-056/074/079, FFI string validation
AC-068a/b, error privacy AC-069 and accepted provider/open/runtime decisions.
No new canonical AC, operation, migration or provider-admission route is proposed.

| Local need → requirement | Falsifiable acceptance / negative control |
| --- | --- |
| N70-1 truthful requested-arm outcome → R70-1 | A70-1: each query-vector-attempting ordinary/frozen/evidence/expand route reports exactly `vector/embedder_absent` for NotConfigured and `vector/embedder_timeout` for StartedTimeout. Seeded lexical IDs survive, all hits have actual surviving-arm provenance, no fabricated vector hits. Direct text-only routes neither embed nor report vector absence. Healthy success leaves degradations empty. |
| N70-1 bounded existing outcomes → R70-2 | A70-2: Saturated and QueuedExpired retain only `embedder_overloaded`; Provider(Failed/Timeout) and InvalidOutput retain current sparse fallback without a newly invented reason. Three typed CUDA provider failures remain errors with exact payloads. A timely panic never becomes a successful degraded result; preserve ordinary unwind and frozen reader-owner Storage outcome. |
| N70-2 truthful shutdown → R70-3 | A70-3: actual shutdown DispatchError::Closing maps to existing EngineError::Closing, with no success result. Preserve dispatcher first-terminal/deadline arbitration and worker lifetime. Observe the corrected helper result before any later reader refusal; retain public shutdown controls. Non-close Cancelled keeps current unlabelled sparse fallback and is deferred unless a truthful accepted contract and reachable production condition are established. No cancellation API or broader Closing wording is added. |
| N70-3 supported remedy → R70-4 | A70-4: blocked readiness and immediate drain error carry the exact ordered remedies `configure_default_embedder`, `submit_non_embedding_input` through Rust/facade/SDK/PyO3/Python/NAPI/TS. Unsupported `configure_caller_embedder` is absent. Code/operation/state/documentation URL/body privacy and pending-work preservation remain unchanged. Ready/processing states keep their existing absence rules. |
| N70-4 usable public types → R70-5 | A70-5: an external facade consumer names SearchHit, IdSpace, IdSpaceKind, SnippetOptions, SnippetAnchor and SnippetV1 and consumes the actual search/snippet field types. Existing independent SDK consumer still resolves these types and snippet_for. No new facade helper or governed operation is needed. |
| N70-5 byte contract → R70-6 | A70-6: Rust/PyO3/Python/NAPI/TS retain all five existing snippet fields, defaults, ranges, native conversion and half-open UTF-8 `[start, end)` coordinates. Body windows/spans and rendered-text highlights decode their intended substrings for combining/astral text. Search rejects out-of-range values; pure selection clamps. Correct demonstrably closed-interval TS/NAPI prose without changing selection. |
| N70-6 truthful carriers → R70-7 | A70-7: new reasons survive Rust SDK and both native result conversions, Python/TS public unions and search/expand wrappers. Rust empty vector/Python None/TS absent remain. MCP emits the exact new words and a matching built degraded hint under both Slice60 formats without losing metadata or violating each format's budget. Any legacy adapter labels remain explicitly distinguished from per-call reasons. |
| N70-6 preserve healthy/result contract → R70-8 | A70-8: unchanged EP03 PINNED bytes pass, as do existing overload, typed-error, eligibility, snapshot and Unicode controls. For each changed reason case, independently seeded expected IDs/order/manual text-arm score expectations and unchanged soft_fallback/explanation meaning are checked. Only reviewed reason/remedy/export/documentation deltas are accepted. |

## Proposed outcome boundary

`embedder_absent` means no numerical runtime was configured for this call;
`embedder_timeout` means dispatch started its request and the absolute deadline
won. Provider-reported Timeout is a distinct Provider outcome, retaining current
unlabelled fallback. A label says why the query vector is missing, not that the
store is globally unhealthy, that all arms ran, or that any hit is weak.
Because the reason taxonomy stays partial, an empty list means none of these
reported causes occurred; it does not certify that every requested arm ran.

Actual failed-open verification returns no Engine; Default/None admission,
identity checks/cache, pinned built-in behavior and caller semantic subprocess
transport remain. Explicit CUDA failure cannot silently become CPU or text
success; automatic policy fallback remains only where the accepted device
policy permits it. No new caller numerical shim, calibration, threshold,
EmbedderFailed/eligibility/projection reason or total arm-completeness claim.

## Receiving boundaries and prerequisites

- Actual10 implementation104dcbc passed fresh independent Astra review and closed
  at d7d928. Actual20 Linux subset657edeb76caaf9c8d2aab433766bec9362e73217
  is integrated with independent final PASS; only engine-owned evidence hooks,
  their tests/census and related documentation changed. Preserve those handles,
  Engine arguments and transaction placement. A3/A5 policy and Windows A6 remain
  held;20 is incomplete. Closed10 is70's hard prerequisite; neither full20 nor30
  closure is a new prerequisite for these labels.70 needs its own tools/artifacts.
- Serialize shared engine/interface/MCP landings with40/50/60/100/110.50 owns
  identity/ranking changes;60 owns formats/default/paging/filter routes.70 owns
  reason vocabulary and minimal carrier/hint translation, with parent-coordinated shared-file landing. Retain the reviewed sampled `embedder_unavailable` epoch label; do not remove it via a wholesale EP03 adapter rewrite.
- New span/ref/scored-byte representation belongs80/90.80 approval consumes
  settled70 contracts;90 also requires actual Block1 follow-up exit after20–70.
  Final combined160 qualification remains separate.
- Slice120 owns selected-major CUDA installation remedies after its still-held
  artifact choice.70 preserves fatal CUDA error transport without freezing12-only
  advice as a universal contract or inferring13 selection/ABI/policy changes.
- Format37/no migration, fixed R580/primary12.6/no K620, fork140 A-only and
  deferred B/C .30 remain. No fault venue, paid run, optional study or publication
  authority is created here.

[Design](design.md) fixes precedence and public-carrier boundaries;
[test map](test-map.md) and [execution map](execution-map.md) stage all supported
native/installed assertions before shared GREEN. [Exact oracle successors](oracle-successors.md)
and the [historical successor](historical-oracle-successor.md) implement A70-7/8:
real None hybrid SDK list is exactly absent, MCP's canonical list is exactly
absent then sampled unavailable, and the first degraded hint names absent.
Direct-text/filter/manual controls stay empty/unchanged. This is the reviewed
bounded reason amendment, not a broader acceptance or fixture rewrite.
