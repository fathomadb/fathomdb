---
title: Vector compatibility failure at open and explicit recovery access
date: 2026-10-31
status: proposed; owner review required; no implementation authorized
target_release: 0.8.28
desc: Replace automatic degraded-open while retaining vector compatibility checks
---

# Proposed vector compatibility open contract

## Recommendation and authority

**Reject provider-enabled open when required vector verification fails.** Keep
explicit no-provider access for valid stores, and keep the compatibility check.
Do not automatically retry the failed open without its provider. This proposal
implements no behavior and supersedes no accepted decision until approved.

Owner investigation direction: steward ledger `seq-350`. Evidence:
[108 baseline observations](../experiments/slice50-refusal-baseline/analysis.md).
The replacement contract, recovery behavior and implementation scope below are
submitted for review together. Release placement remains subject to the owner.
D50-1/D50-2 remain unimplemented; approval of investigation did not select them.
Proposal record: steward `seq-351`, measurement `seq-352`, lifecycle follow-up
`seq-353`; pending release-state decision
`vector-compatibility-open-contract`. This gates this policy implementation,
not publication authorization or unrelated release work.

Upon acceptance, supersede only degraded-open clauses R-VEQ-4/6 in
[ADR-0.8.18](ADR-0.8.18-vector-equivalence-self-check.md) and the corresponding
probe-failure clauses in
[ADR-0.8.27 runtime topology](ADR-0.8.27-engine-owned-runtime-topology.md).
Retain identity/dimension gates, thresholds, async writes, resource ownership,
provider deadlines and bounded cleanup. Any revised verification cadence must explicitly address
the TC-68 cached-acceptance rule (the 0.8.20 plan/design). Cadence and reuse
remain undecided pending the measured startup cost and SDK/MCP lifecycle review. Respect the existing
[corruption-on-open policy](ADR-0.6.0-corruption-open-behavior.md).

## Proposed open contract

| Situation | Outcome |
| --- | --- |
| Attached provider; stored or prospective vector arm requires verification; accepted verdict | Return a usable engine. |
| Attached provider; required numeric comparison fails | Return a typed open error; no engine handle. |
| Required verification cannot run or complete | Return a typed open error preserving the cause; no engine handle. |
| Verification metadata is structurally invalid | Refuse ordinary admission with structured corruption/state detail; do not reinterpret as a numeric mismatch. |
| Caller explicitly selects no provider; store passes applicable integrity checks | Open for existing canonical/text operations, with no vector provider or verification claim. |
| Attached provider; no stored/prospective vector arm | Preserve existing probe-not-required behavior and first-enrollment rules. |

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

Do not select unconditional fresh-per-open verification from these measurements
alone. Verification reuse/invalidation and the MCP resource lifecycle must be
reviewed together before implementation. A process/session cache would need to
bind acceptance to the actual runtime and current store verification inputs;
the current persistent hash omits live backend/device/library/pooling changes.
Retaining a provider across closes also changes the engine-owned resource
contract. Neither alternative is designed or approved here.

Retain exact thresholds and the accepted initial-enrollment residual. Vector-less
and no-provider opens remain outside the live-probe requirement. No repeated
45-probe check is proposed on each engine call. No cache retention/removal,
idle-timeout change or runtime-sharing change has been implemented or approved.

An absent baseline is not automatically corruption: it is legitimate during
first enrollment. An unreadable arm-registration query must propagate failure,
not be treated as an empty arm. Validate applicable stored metadata independently
of model inference; selecting no provider must not bypass known corruption.

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
  its storage classification. An invalid cache marker alone triggers a fresh
  comparison, rather than proving reference corruption.

Precise enum/class names are implementation design; the distinctions above
are the proposed public contract. Initial subscribers must receive the error,
including rejection after `open_locked` succeeded. SDK mappings, error docs and
contract tests change in the same implementation. Remove obsolete refusal-only
live-engine branches/report fields across surfaces; add no compatibility shims.

Failed verification preserves existing reference vectors, stored vector data,
pinned mean and declared identity. It may invalidate a stale success marker;
it must never publish a success marker or bless the failing backend. Stage a
new complete baseline in memory and publish it only after acceptance on both
registered and prospective-arm paths. Partial or rejected baselines are not
published; late inference results cannot alter references or verdicts.

This is **not** whole-open rollback: ordinary startup maintenance/recovery and
supported migration can precede verification. Their existing durable effects
remain governed by their own contracts. Do not promise byte-identical files
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
provider. This is a required correction for custom-identity stores. When a
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
Engine admission or reference/cache mutation. Reuse locked, WAL-aware diagnostic
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

A read-only diagnostic does not itself revoke admission. Current TC-68 can
still accept a cached verdict after fresh diagnosis fails. Under a future fresh
admission rule, ordinary open would compare again; any future verification-reuse
rule must specify invalidation and treatment of that known failed diagnosis.
The lifecycle/cadence decision remains pending; do not imply this gap is fixed.

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
| Verification | Identity gates, numeric probe, cached acceptance | Classified rejection; preserve limits; settle verification reuse and SDK/MCP reopen cost before selecting cadence. |
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
5. CLI fresh diagnosis despite a cached verdict, immutable reference/cache state,
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

Owner review is requested for the failure/recovery contract and the unresolved
verification-reuse/lifecycle design before implementation. An approved runtime-keyed
verdict cache or unconditional fresh cadence is not assumed. In-place backend rotation,
whole-open rollback, verification of vector-less opens, and removing healthy
store-wide drain barriers are not implied by approval. Existing accepted ADRs
remain authoritative until a successor is approved.
