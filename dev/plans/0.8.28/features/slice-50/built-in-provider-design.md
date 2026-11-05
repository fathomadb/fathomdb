---
title: Built-in provider — narrow admission and failed-open proposal
status: implemented; separate Astra-medium APPROVE; verification recorded
target_release: 0.8.28
observed_on: 2026-11-01
---

# Built-in provider contract

## 1. Scope and authority

Owner scope `seq-355` excludes user-specified engine embedders before 1.x.
Correction `seq-357` requires preserving existing verification and cache behavior.
This proposal closes production provider escape paths and converts an actual
verification refusal into a typed failed open. It does not strengthen the checker
into a continuing runtime guarantee. The owner approved this narrowed contract and implementation at `seq-359`;
separate Astra-medium implementation review gates landing on `dev/0.8.28`; D50-1/D50-2 remain held, and Unit S is
unchanged. The previous bounded-evidence/runtime-stability proposal is superseded.

Read [requirements and AC](built-in-provider-requirements.md),
[plan](built-in-provider-plan.md), and the
[proposed successor ADR](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md).
The prior broad-scope reviews are historical. A separate Astra-medium reviewer
reviewed this revision after its author stopped writing: [APPROVE_FOR_OWNER_REVIEW](built-in-provider-narrow-review.md),
with no P1/P2 findings. Owner approval followed at `seq-359`; implementation review remains required before landing.

## 2. Preserve the existing checker and cache

Source witnesses below are relative to `src/rust/crates/`, inspected at
`ae231e3c8`. This document changes no source.

`fathomdb-engine/src/vector_equivalence.rs:119–180` owns the current gate:
no provider means no probes; a registered vector kind or durable prospective
dense declaration engages verification. Preserve the gate, including the
first-enrollment residual and existing behavior when no kinds/declaration remain.
A retained baseline alone does not become a new reason to probe. Do not add
synchronous probes to canonical writes or checks to every inference call.

Preserve the fixture, structural validation, identity/dimension rules, P1 zero
binary flips and P2 uncentered maximum L2 `1e-5`. Preserve the fingerprint recipe
and inputs, comparison algorithm, marker lookup and publication/error behavior.
The persistent TC-68 marker remains authoritative for cached acceptance.
`vector_equivalence.rs:347–375` hashes stored verification inputs, not the live
backend; `:425–465` implements persistent lookup, best-effort recording and
existing registered/prospective error invalidation differences. No incidental
fingerprint or marker correctness changes are bundled into this work.

For ordinary clean cases, first population embeds 45 references and confirms them
with 45 further embeddings (90 total); a fresh check of an existing baseline
uses 45; a valid cached open uses zero. Vector-less/no-provider opens use zero.
A process restart does not invalidate a valid persisted marker. MCP idle closure
and A/B/A reconstruction retain existing cached behavior without a new cache.
Missing/stale/malformed markers and changed existing fingerprint inputs retain
their current validation and recheck behavior. Marker-write failure remains a
successful verified open with possible rechecking later, not a new refusal.

The cache does not define validity or extend the check's assurance. The existing
policy accepts its known residual: a valid marker can hide same-identity backend,
pooling, device or library drift because those runtime facts are not fingerprint
inputs. Closing injection narrows supported choices; it does not eliminate that
residual. The marker is not authenticated and the finite probes do not establish
universal equivalence or historical vector provenance. See the explicit source
comments at `vector_equivalence.rs:387–424` and the accepted TC-68 design.

Pinned BGE/Mean, supported device selection, loader behavior, ambient Candle
precision behavior, mean pin/recompute/recovery ordering and context-error policy
remain unchanged. No immutable runtime descriptor, owned asset snapshot, new
Candle fork, context-health registry, retained probe outputs, process cache,
capacity/TTL/LRU, epoch, or diagnostic cache revocation is required.

## 3. Production boundary and retained seams

Only production `EmbedderChoice::Default` and `None` remain. User providers may be
considered at 1.x or later, with a separate contract; they are not promised for 1.0.
Device choice is not provider choice. Retained low-level helpers need not become
private or move features merely to enforce a boundary already closed at admission.

| Existing path | Disposition and minimum containment |
| --- | --- |
| Engine `open.rs:1999–2026`, caller dispatch `:525–557` | **Close** `Caller` and `CallerWithDeviceResolution` production variants. Feature unification must not restore them. Keep `Default`/`None`; remove misleading caller-Noop recovery advice at `:1647`. |
| Engine `projection_registry.rs:121–123`, stored selector comparison `:1188` | **Close** new provider selection: new declarations accept omitted/null `embedder` only; reject every string, including `default`, empty and model names, before mutation. Keep old stored strings verbatim for inspection; do not relabel them or execute a substitute BGE provider. Unsupported historical read-to-configure is intentionally refused. No vector-config identity strings are introduced. |
| Embedder API `lib.rs:79–96` trait | **Retain** low-level abstraction for dispatch, tests and evaluation; annotate that implementation is not production engine admission. No generic stability/descriptor API. |
| Embedder `candle_bge.rs:539–697` constructors and `with_pooling`; `loader.rs:148` paths | **Retain** existing standalone/test/evaluation construction and loader semantics. Production Default retains its existing pinned Mean construction; no caller object can be attached. No new provenance capability or owned-byte constructor is needed. |
| Embedder `lib.rs:206–210`: Nomic and optional ONNX | **Retain deferred** evaluation backends and current feature gates; annotate no supported production engine attachment before 1.x. Preserve offline evaluation data and accepted ONNX boundary. Do not gratuitously reorganize features. |
| Embedder `lib.rs:219–240`: Noop | **Retain** low-level test fixture with explicit non-production annotation. Production caller closure removes its engine route; no new feature gate solely for cosmetic hiding. |
| Engine `open.rs:696`; Python/Node `test_support.rs` | **Retain** distinct hidden test/evaluation openers where needed, behind existing appropriate non-default support gates. They may inject fakes for failure tests; normal admission stays closed even when those features unify. Preserve current checker behavior in test engines rather than inventing a second verification system. |
| Engine `vector_storage.rs:643–645` `write_vector_for_test` | **Retain** existing test/test-hooks gate and shipping-feature exclusion; no production vector-import route. |
| Embedder loader hooks (`loader-test-hooks`) and `tc5-benchmark` constructors | **Retain** current non-default evaluation boundaries and attestation purpose. No production open accepts their caller-created providers. |
| SDK `standalone.rs:88,138`; Python `embedding.rs:179,190`; Node `embedding.rs:19,30` | **Retain supported** standalone CLS helper, explicitly distinct from engine Mean. It returns vectors; it does not select or inject an engine provider. No helper removal or new cache separation machinery. |
| SDK `engine.rs:56–58`, Python/Node opens and MCP SDK open | **Retain** default/none selection, handle ownership and lifecycle; never automatically retry without a provider. |
| Device policy, reranker and shared Candle settings | **Retain** current behavior and qualification. No new device/provider grammar, precision freeze or per-call health instrumentation. |
| Persisted profiles, probe references and kinds | **Retain** data and schema. Well-formed historical unsupported profiles remain inspectable without a provider; no implicit migration, relabeling or BGE substitution. |

Update interface, crate and relevant design annotations where this boundary
supersedes old promises. In particular, the accepted embedder-protocol ADR's
user-provider promise needs a successor scope clause; retain its dispatch rules.
The ONNX ADR remains historical evaluation authority, not a production escape.
Test feature closure at the actual shipping artifacts and ordinary API, without
requiring all public low-level symbols to disappear. No pre-1.0 compatibility shim.

## 4. Actual refusal becomes failed open

Run the existing admission/check/cache flow. If it accepts or is inapplicable,
return the engine normally. If it actually refuses, return a classified error
and no Engine, rather than `dense_disabled` degraded-open. A cache hit remains
acceptance; do not force an uncached check to justify this new error policy.

Distinguish numeric mismatch, cannot-verify causes (provider error, panic,
timeout/admission failure, malformed output), and invalid reference/profile/mean
state. Preserve underlying typed causes through the checker and bindings rather
than parsing today's reason string. These plumbing changes must not alter which
inputs pass, fail or skip. Initial subscribers receive the error. Remove only
obsolete refused-open state/report branches; later runtime errors keep their
existing operation behavior. No every-call compatibility guarantee or automatic
provider fallback.

At `open.rs:867–878`, the current probe precedes construction of
`OpenPostProbeGuard`. Establish cleanup ownership before the new early return.
Preserve engine-owned dispatch, existing single bounded cleanup budget, original
error, SQLite/readers/callback/lock release and late-result discard. A still-running
provider can retain only existing provider/request/accounting resources, never
database authority. No new cleanup topology or renewed retry budget.

### Minimal first-baseline atomicity

`vector_equivalence.rs:206–242` stages and confirms prospective references before
publication, but registered-arm first population persists before confirmation.
Propose applying the existing stage/check/persist ordering to registered first
population too: a failed open must not leave newly created, rejected references
as the next open's authority. This is directly necessary failed-open failure
atomicity, not a new enrollment or verification regime. Preserve 90 ordinary
first-population calls, checker inputs, thresholds and subsequent marker behavior.
Preserve existing registered versus prospective marker-error handling. Existing
references are never replaced on failure. Test confirmation failure and storage
failure explicitly; make no whole-open rollback promise. Ordinary earlier mean
recovery/migrations keep their current durable effects and ordering.

## 5. Deliberate no-provider access and deferred work

Use existing None/default-off selectors. Correct no-provider admission to validate
an existing stored profile as metadata rather than compare it with fictitious BGE
identity (`open.rs:557`, profile validation near `:2125`). Preserve integrity and
corruption refusal; do not promise arbitrary damaged databases can open. New-store
profile creation stays under existing defaults. This is ordinary mutable access,
not forensic mode: canonical/text/lifecycle/erasure behavior and deferred dense
work remain governed by existing rules. No-provider startup mean recovery and
explicit recompute retain existing behavior, with no numerical verification claim.

No-provider access is not a repaired dense backend. Existing doctor identity checks,
structural diagnostics, export and rebuild do not establish numerical compatibility.
The known provider-free rebuild invalidation defect remains disclosed: current
operator rebuild can invalidate dense state without a usable provider, and CLI
recovery currently opens without one. This proposal neither fixes nor certifies
that path safe. Root/owner must separately disposition it; do not expand this
change into dense rebuild admission, orchestration or a new diagnostic CLI.

| In scope | Deferred / unchanged |
| --- | --- |
| Production default/none boundary; selector rejection; retained-seam annotations | General user-provider support, feature cleanup not needed for closure |
| Typed refusal/no handle; existing bounded cleanup; minimal rejected-baseline atomicity | D50-1/D50-2 drain changes, general runtime validity enforcement |
| Necessary no-provider stored-profile validation correction | New diagnostic command, cache revocation, dense rebuild orchestration and the known rebuild defect |
| Existing checker/cache/mean/loader/device behavior and explicit residuals | New cache/fingerprint/cadence, precision fork, asset snapshots, mean advisories/reordering |

The owner's less-than-3-percent probability is a qualitative planning assumption,
not a measured supported-production incident rate. Existing timing/characterization
remain historical evidence; no new measurement campaign or pooling study is needed
for this scope. Owner `seq-359` authorizes this narrowed implementation; excluded redesigns
remain outside scope. Separate Astra-medium implementation review precedes landing.

Implementation completed under owner `seq-359`: see the
[implementation and verification record](built-in-provider-implementation-record.md).
