---
title: Built-in provider verification, open failure and explicit recovery access
date: 2026-11-01
status: proposed; owner review required; no implementation authorized
target_release: 0.8.28
desc: Replace automatic degraded-open while retaining vector compatibility checks
---

# Proposed vector compatibility open contract

## Recommendation and authority

**Support only the pinned built-in BGE-small/Mean engine provider before 1.x,
and reject provider-enabled open when required vector verification fails.**
Keep deliberate no-provider access to valid stores, retain the compatibility
check, and use bounded process-local numerical evidence to avoid repeated
inference for unchanged relevant opens. No automatic fallback after failure.

Owner scope ruling: steward `seq-355`; user-provider support is a long-term goal,
not a pre-1.x feature. The owner commissioned the revised
[built-in-provider contract and extension inventory](../plans/0.8.28/features/slice-50/built-in-provider-design.md),
[user needs/requirements/AC](../plans/0.8.28/features/slice-50/built-in-provider-requirements.md)
and [work plan](../plans/0.8.28/features/slice-50/built-in-provider-plan.md).
Astra-medium developed and reviewed the design. This ADR is proposed; detailed
mechanisms, public API removals, fork/dependency work, open/recovery behavior
and release placement require owner review before implementation.

Evidence and history: [108 baseline observations](../experiments/slice50-refusal-baseline/analysis.md),
[fresh-open timing and lifecycle findings](../experiments/fresh-open-verification/findings.md),
and steward `seq-350` through `seq-356`. Release-state decision
`vector-compatibility-open-contract` remains unruled. D50-1/D50-2 are historical,
unselected options; Unit S's scan optimization remains separate.

Upon acceptance, explicitly supersede degraded-open clauses R-VEQ-4/6 of
[ADR-0.8.18](ADR-0.8.18-vector-equivalence-self-check.md), corresponding
probe-failure clauses of [ADR-0.8.27](ADR-0.8.27-engine-owned-runtime-topology.md),
and TC-68's persistent cached-acceptance rule. Narrow the public provider promise
in [ADR-0.6.0 embedder protocol](ADR-0.6.0-embedder-protocol.md), and make the
[embedder design's](../design/embedder.md) automatic mean pin/recovery
compatibility-conditioned on a proposed mean. Retain finite thresholds,
identity/dimension/integrity gates, async canonical writes, dispatch ownership,
deadlines and bounded cleanup. Respect the
[corruption-on-open policy](ADR-0.6.0-corruption-open-behavior.md) and
[embedder-owned vector identity](ADR-0.6.0-vector-identity-embedder-owned.md).

## Proposed open contract

| Situation | Outcome |
| --- | --- |
| Attached provider; stored or prospective vector arm requires verification; accepted verdict | Return a usable engine. |
| Attached provider; required numeric comparison fails | Return a typed open error; no engine handle. |
| Required verification cannot run or complete | Return a typed open error preserving the cause; no engine handle. |
| Verification metadata is structurally invalid | Refuse ordinary admission with structured corruption/state detail; do not reinterpret as a numeric mismatch. |
| Caller explicitly selects no provider; store passes applicable integrity checks | Open for existing canonical/text operations, with no vector provider or verification claim. |
| Built-in enabled; no stored/prospective dense arm and no baseline | No probes; preserve explicit unbaselined first-enrollment residual. |
| Built-in enabled; baseline retained after removing all dense kinds | Check or reuse exact evidence before later reenrollment can serve dense work. |
| Caller requests another engine provider | Unsupported production surface before 1.x; no caller-provider admission route. |

Success is **not** a guarantee against subsequent provider failure, nor a claim
that all projections have caught up. No live engine is returned in the
attached-but-refused state. A provider error never becomes implicit permission
to serve vectors or silently open in a different mode.

The [fresh-open measurement and lifecycle follow-up](../experiments/fresh-open-verification/findings.md)
found added median cost of 0.16–0.18 s on RTX 3090 and 2.37–2.56 s on the tested
CPU. SDK handles can stay open across the application lifecycle. MCP-managed
stores on dev/0.8.29 instead default to idle close after 60 seconds and enforce
one open store; switching can reopen stores on sequential requests. Thus a
fresh check on every open can become a recurring query-path cost.

The revised contract chooses a first relevant live check per process/state/runtime,
then bounded reuse across engine/model close and reconstruction. Retire on-disk
marker authority. The cache holds at most 16 positive entries and 2 MiB accounted
payload; each contains the successful check's 45 uncentered live outputs
(69,120 bytes), fixed digests and metadata, without models or database resources.
Each active baselined engine holds one evidence set independently of LRU eviction.
No ordinary call repeats the 45 probes. Exact keying, context validity, epoch
fencing, bounds and state transitions are specified in the companion design.

This depends on net-new sealed construction from actual owned verified asset
bytes and immutable per-runtime strict-F32 Candle execution. A constructor
snapshot of mutable global settings cannot prove stability. Fork work must be
independently qualified before adoption; no fork change is implemented here.

Automatic pin/recovery compares the proposed mean against retained live outputs
and stored references without provider inference inside a transaction. On a
rejected optional candidate, retain the accepted current mean and continue valid
projections, with one queryable/event advisory and suppressed automatic retries
per accepted-state token. Explicit recompute returns a typed rollback error.
For existing baselines, check current state before optional open-time mean
recovery; do not commit a rejected candidate by reopening. This explicitly
changes the old unconditional automatic pin/recovery policy on rejection.
Baseline-bearing ordinary no-provider open defers automatic mean recovery;
explicit no-provider recompute makes no numeric acceptance claim.

A relevant provider-enabled open has either a registered/durable dense arm or
an existing probe baseline. Truly vector-less and no-provider opens do no live
probes; retained baselines cannot evade checking by temporary kind removal.
Retain legitimate initial-enrollment absence of references and its explicit
historical-provenance residual. An unreadable arm-registration query propagates
failure; it cannot masquerade as an empty arm. Structural validation is separate
from inference and never bypassed by no-provider selection.

## Astra-medium design and review

The earlier [reuse review](../plans/0.8.28/features/slice-50/verification-reuse-astra-review.md)
returned SOUND_DIRECTION_NEEDS_DESIGN. At the owner's request, Astra then
proposed and [reviewed the integrated design](../plans/0.8.28/features/slice-50/built-in-provider-astra-review.md). It proposed the [built-in-only contract](../plans/0.8.28/features/slice-50/built-in-provider-design.md)
and audited extension paths. The proposal closes production injection, contains
retained evaluation seams, specifies actual-construction provenance, immutable
execution, finite numerical evidence, bounded ownership/concurrency, context-loss
invalidation and transactional mean decisions. It is a concrete successor
proposal, not acceptance or a claim that those mechanisms exist today.

## Error and durable-state contract

Expose stable, structured open failures across Rust, Python, TypeScript and CLI:

- **Numeric mismatch:** compatibility-specific open error, with reason code,
  probe stage and measured flip/L2 values versus limits. Do not call it missing
  configuration or a scheduler timeout.
- **Cannot verify:** distinguish provider error, panic, deadline, admission
  failure and malformed output; preserve typed dispatch causes. No string
  parsing of the current human-readable refusal reason.
- **Invalid reference/profile/required mean:** structured invalid-state or
  corruption detail with locator and remediation hint. SQL I/O failure retains
  its storage classification. A malformed legacy on-disk cache marker is ignored and proves neither
  acceptance nor reference corruption; the new process evidence rules decide
  whether fresh comparison is required.

Precise enum/class names are implementation design; the distinctions above
are the proposed public contract. Initial subscribers must receive the error,
including rejection after `open_locked` succeeded. SDK mappings, error docs and
contract tests change in the same implementation. Remove obsolete refusal-only
live-engine branches/report fields across surfaces; add no compatibility shims.

Failed verification preserves existing reference vectors, stored vector data,
pinned mean and declared identity. Legacy success markers have no admission authority;
failure must never publish process acceptance or bless the failing backend. Stage a
new complete baseline in memory and publish it only after acceptance on both
registered and prospective-arm paths. Partial or rejected baselines are not
published; late inference results cannot alter references or verdicts.

This is **not** whole-open rollback: ordinary startup maintenance and
supported migration can precede verification. Baseline-bearing automatic mean
recovery follows the prospective compatibility ordering above; other recovery
effects retain their own contracts. Do not promise byte-identical files
after failure or invent a schema migration for this policy change.

## Cleanup contract

Reject after `OpenPostProbeGuard` owns the connections and admission lock,
before boot graft, projection-runtime startup, profile installation or returning
an engine. Existing guards already provide the required unwind foundation.

Before error return, release all SQLite owners, callbacks and the admission
lock. Close dispatch admission and apply the existing single 30-second provider
drain budget. A provider still running after that budget may retain only its
provider/request/accounting state, never database authority. Preserve the
original open error; report incomplete provider cleanup as secondary detail.
No reaper/replacement thread, renewed drain budget or unbounded join.

A probe deadline does not bound total failed-open latency: normal startup and
resource cleanup are additional. Recovery access may reacquire database
admission once SQLite ownership is released; it does not require a stuck
provider to exit. Repeated retries against a permanently stuck provider can
retain additional workers: the supported advice is fix/restart its host, not
an automatic retry loop. Preserve the established late-result discard rule.

## Explicit access and recovery path

Reuse the existing explicit selectors: Rust `EmbedderChoice::None`, Rust SDK
`OpenOptions.use_default_embedder = false`, Python `use_default_embedder=False`,
and TypeScript `useDefaultEmbedder: false`. Do not add an `allow_failed_check`
flag, automatic fallback or recovery verbs to the governed SDK.

For an existing store, no-provider admission must validate and use the stored
profile identity/dimension as metadata, not compare it with an invented BGE
provider. This is a required correction for well-formed historic unsupported-identity stores. When a
provider is actually supplied, strict identity/dimension matching still applies.
New-store profile creation remains governed by existing defaults.

No-provider admission is **mutable ordinary admission**, not a forensic or
read-only mode: startup maintenance and governed canonical operations remain
possible. Use explicit `search_text_only`/`searchTextOnly` for FTS without
embedding, vector, graph or CE work. Inspection and valid metadata/lifecycle/
erasure operations remain available under their existing invariants. Sparse
fallback in ordinary search does not promise dense completeness. No-provider
pending work remains visible as blocked/deferred; zero pending work does not
mean an attached provider was verified.

When either rebuild would invalidate or regenerate **dense state**, require a
usable provider, even at zero backlog, before mutation. Determine dense state
from authoritative declarations/enrollment and affected materializations, not
pending count alone. Keep provider-free `rebuild_projections` for stores with
only text/attribute projections; do not break pure-text repair. This is a new
operator admission rule, not D50-2's metadata drain change. A projection rebuild
is not backend replacement or verification.
Ordinary canonical writes may still create explicitly deferred projection work;
that existing no-provider contract is not silently made read-only here.

| Cause | Supported recovery |
| --- | --- |
| Transient provider error, panic, timeout or malformed output | Repair the provider/environment; retry normal open against the original references. A stuck provider may require host restart. |
| Genuine numeric incompatibility | Restore the original compatible backend/environment. Until then, explicitly use no-provider access if store integrity permits. |
| Invalid verification metadata or required mean | Preserve evidence, diagnose offline, and restore a consistent trusted backup. Do not delete references or mark the backend verified. |
| Intentional new embedding function | Reconstruct a separate store from authoritative source/provenance, or design a separate complete offline replacement workflow. No existing safe in-place rotation/rebaseline command is promised. |

`doctor verify-embedder` currently checks identity/dimension only. `recompute-mean`
uses retained vectors; rebuild preserves the verification references;
`safe-export` copies the database including its vector/reference state. None is
a numeric compatibility repair or a way to authorize a changed backend.

Add a CLI-only **proposed** `doctor check-vector-compatibility` diagnostic that
can explicitly run a fresh comparison despite the cache, without ordinary
Engine admission or database/reference mutation. Reuse locked, WAL-aware diagnostic
admission; never ignore uncheckpointed WAL to obtain an apparently clean view.
Use an explicitly selected supported provider (initially the pinned default);
return unsupported-provider detail for a custom identity the CLI cannot load,
while still permitting structural inspection. Do not fabricate a numeric verdict
from identity-only comparison. Report mismatch, provider failure and invalid
metadata separately. It must not
require successful provider-enabled open to diagnose why that open fails.
Existing `doctor data-plane-integrity` remains the immutable structural diagnostic;
its current checks must not be advertised as full compatibility diagnosis.
Report `baseline_not_established` when first-enrollment references legitimately
do not exist; a read-only diagnostic must not populate them or label that state
corruption. Report whether ordinary admission would reuse a cached verdict.

A failed fresh diagnostic clears in-process positive evidence and increments
the publication epoch without changing the database. Its store admission lock
excludes a concurrent live engine of that store. A coherent successful fresh
comparison may repopulate bounded evidence; an absent baseline cannot. Context
loss also invalidates the affected live runtime at the boundaries in the design;
clearing cache entries alone cannot revive or validate an existing runtime.

No-provider access is not a corruption bypass. If ordinary integrity admission
fails, use CLI diagnostics and the supported offline recovery/backup route;
do not return an SDK handle to known-corrupt state. Source re-ingestion is not
a promise to preserve cursors, revision identifiers or complete provenance
without an application-controlled reconstruction plan.

### Keep CLI rebuild recovery usable

The existing `wire_recover` always opens without a provider. Pair the admission
rule above with a **new explicit** provider-enabled CLI selection for dense
rebuild, using the same compatibility checks and typed refusal as normal open.
A proposed `--use-default-embedder` on the two rebuild actions selects the
pinned default provider; explicit model loading follows existing device policy.
A binary without that feature gives a typed feature-unavailable refusal.
Wire the CLI's `default-embedder` feature through the facade to the engine:
its current Cargo feature enables the direct loader only, so selecting
`EmbedderChoice::Default` in a CPU-only CLI would otherwise still fail.
Other recovery actions stay provider-free; do not attach a model to erasure.

The selected provider must match the stored identity; never substitute BGE for
a custom profile. Custom-provider CLI loading is not implemented today. Until a
separately specified provider integration exists, custom dense rebuild remains
unavailable through this CLI; inspection/erasure still use the corrected
no-provider route. Do not represent this limitation as an automated backend
migration or add a model plugin framework to this change.

For a valid provider-enabled rebuild, settle pre-existing work while dispatch
is unfrozen, then preserve the established freeze/worker-quiescence rules around
invalidation. After regeneration resumes, report vector repair complete only
when its readiness/barrier confirms completion; timeout must retain a truthful
pending/error outcome. CLI shutdown must not turn queued work into a claim of
completed repair. These are operator orchestration requirements; optimizing
healthy metadata's store-wide barrier remains separate. Add CLI end-to-end
coverage so the new admission rule cannot make both rebuild commands unusable.

## Code-grounded delta and validation

| Area | Exists today | Required change after approval |
| --- | --- | --- |
| Verification | Identity gates, numeric probe, cached acceptance | Classified rejection; preserve limits; bounded built-in evidence reuse, immutable runtime and explicit invalidation. |
| Cleanup | Post-probe guard and bounded provider-only retention | Wire new rejection through guards and initial-subscriber reporting. |
| Baseline publication | Prospective arm stages before acceptance; registered arm persists first | Make first-baseline publication acceptance-conditional on both paths. |
| No-provider access | SDK selectors and CLI doctor/recover | Validate stored profile without fictitious provider identity; retain corruption refusal. |
| Vector rebuild | Can invalidate at zero backlog without usable provider; CLI is always provider-free | Require usable provider for dense state; add explicit provider-enabled CLI path and completion reporting; preserve pure-text repair. |
| Diagnosis | Identity-only doctor check; structural offline checks | Fresh, non-mutating CLI numeric diagnostic; no SDK bypass. |

Source witnesses (relative to repo root, inspected at `5d0dacea5`):
`src/rust/crates/fathomdb-engine/src/open.rs:13` (outer guard), `:121`
(database unwind), `:557` (None identity), `:773` (independent device policy),
`:866` (probe), `:877` (post-probe guard), `:928` (worker start), `:1055`
(error event), `:2125` (profile check); `vector_equivalence.rs:77` (cause loss),
`:161` (arm-query error suppression), `:199` (baseline paths); engine
`src/embed_dispatch/core.rs:507` (drain), `src/projection_rebuild.rs:60`
(admission), `src/operator.rs:269` (export), `:306` (identity-only doctor).
CLI `src/rust/crates/fathomdb-cli/src/lib.rs:1896` and `:1918` open without a
provider. SDK selectors are in Rust SDK `src/engine.rs:56`, PyO3
`src/engine.rs:178`, N-API `src/engine.rs:17` under their respective crates.

None-mode does not currently bypass independent CUDA pool/reranker device-policy
validation. A guaranteed GPU-independent forensic path therefore uses offline
CLI admission, not an undocumented promise that ordinary None-mode never
consults device configuration.

Before landing implementation, require behavior-specific RED/GREEN tests for:

1. Mismatch/error/panic/timeout classification and no returned handle; immediate
   lock reacquisition, no retained SQLite owners, bounded provider-only retention,
   original error preserved and no late publication.
2. Old reference/mean/vector preservation on comparison failure, no partial or
   rejected initial baseline, accepted retry and startup-maintenance boundaries.
3. Valid no-provider access for default and custom identities, corruption refusal,
   FTS results, lifecycle/configuration/erasure postconditions, pending visibility,
   close/reopen, and no model inference in the no-provider path.
4. Both no-provider rebuilds rejected before dense invalidation at zero/nonzero
   backlog; pure-text projection repair preserved; explicit provider-enabled CLI
   rebuild completes from a healthy backlog and survives close/reopen, preserving
   existing concurrency guards. Unsupported custom provider/feature combinations
   refuse clearly before mutation; cover CPU-only default-provider feature
   wiring as well as CUDA-enabled builds.
5. CLI fresh diagnosis despite a cached verdict, immutable database/reference state,
   absent-baseline result, WAL-aware admission, error mapping and SDK parity;
   vector-less, no-provider and first-enrollment behavior explicitly covered.
   Test the selected verification-reuse/invalidation contract across MCP idle
   closure and A/B/A switching. Repeated calls on an accepted engine must not
   repeat the 45-probe check.

These are proposed tests, not passing claims. The existing real-error unwind
unit test was rerun unchanged: 1 passed, 111 filtered out (0.36 s). It establishes
the cleanup foundation, not the new rejection path. Its output is preserved in
[open-policy-cleanup.log](../experiments/slice50-refusal-baseline/open-policy-cleanup.log).
The earlier 108-case matrix
and 40 controls remain the behavioral baseline. No production source, tests,
accepted ADR or live release ladder was changed by authoring this proposal.

## Options and decision requested

Recommend the contract above: explicit failed open plus deliberate no-provider
access and CLI diagnosis. It avoids the measured impossible-progress state while
retaining useful text/canonical access. Keeping automatic degraded-open would
require capability restrictions across every mutation/rebuild path and preserve
the attached-but-unusable state; that is a wider and more fragile contract.
Accepting failed verification or resetting references is outside the safety
constraint and is not proposed.

Owner review is requested for the revised built-in-provider contract, its
extension-path dispositions, compatibility-conditioned automatic mean policy,
bounded evidence cache and fork stability prerequisite before implementation.
The user-provider exclusion is ruled; these mechanisms and release placement
are proposed. In-place backend rotation, whole-open rollback and removing
healthy store-wide drain barriers are not implied. Existing accepted ADRs remain
authoritative until the successor is approved.
