---
title: ADR-0.8.28-vector-compatibility-open-policy
date: 2026-11-01
target_release: 0.8.28
desc: Close production provider gaps and reject open on actual verification refusal while preserving TC-68 cache behavior
status: accepted by owner seq-359; scope corrected by seq-357; implementation and separate code review pending
---

# Vector compatibility admission and deliberate access

## Authority and accepted decision

Owner `seq-355` excludes user-specified engine embedders before 1.x. Correction
`seq-357` preserves existing checks and cache settings/behavior and rejects the
expanded runtime/evidence-cache proposal. The owner accepted this ADR and implementation at `seq-359`: production
Default/None admission and a typed failed open when the existing checker actually
refuses. It does not make every open a fresh check or promise ongoing runtime
compatibility. Implementation lands on `dev/0.8.28` only after the requested separate
Astra-medium implementation review. D50-1/D50-2 remain held; Unit S remains separate.

The current [design and inventory](../plans/0.8.28/features/slice-50/built-in-provider-design.md),
[requirements/AC](../plans/0.8.28/features/slice-50/built-in-provider-requirements.md)
and [plan](../plans/0.8.28/features/slice-50/built-in-provider-plan.md) define this
narrow proposal. Prior Astra reuse/integration reviews are superseded historical
reviews. A separate [independent Astra-medium review](../plans/0.8.28/features/slice-50/built-in-provider-narrow-review.md)
returned APPROVE_FOR_OWNER_REVIEW with no P1/P2 findings; owner approval followed at `seq-359`.

Owner approval at `seq-359` supersedes the degraded-open portions of
`ADR-0.8.18-vector-equivalence-self-check.md` and
`ADR-0.8.27-engine-owned-runtime-topology.md`, and narrow the user-provider promise
of `ADR-0.6.0-embedder-protocol.md`. Retain engine-owned dispatch, deadlines,
cleanup and identity ownership. Existing accepted ADRs remain authoritative
except for the explicitly superseded clauses accepted here. TC-68 fingerprint/cache/cadence is not superseded.

## Preserve verification and cache behavior

| Existing admission outcome | Proposed open result |
| --- | --- |
| Existing checker runs and accepts | Return engine normally. |
| Existing validated persistent marker accepts | Return engine normally with zero probes; marker remains authoritative. |
| Existing gate does not engage (including no provider/current vectorless cases) | Preserve normal admission and existing first-enrollment residual. |
| Checker actually refuses due to mismatch or inability to verify | Return classified open error; no Engine. |
| Corrupt/invalid profile or reference state rejected by existing admission | Preserve rejection with structured detail. |
| Caller supplies another engine provider | No supported production admission route before 1.x. |

Preserve existing identity/dimension and structural checks, 45-probe fixture,
P1 zero binary flips and P2 maximum uncentered L2 `1e-5`. Preserve the registered
kind/durable prospective declaration gate; retained baseline alone does not add
a new gate when kinds/declarations are absent. Preserve existing fingerprint
inputs/recipe, marker validation/recording/error rules and timing of checks.
Normal clean first population uses 90 embeddings, fresh existing-baseline check
45, cached acceptance zero, and vectorless/no-provider paths zero. Process restart
or model reconstruction does not independently invalidate a valid marker.
MCP idle close and A/B/A switching continue to use this existing behavior.

The fingerprint omits actual backend/pooling/device execution facts. A valid
marker can therefore hide same-identity backend drift; closing caller injection
does not claim to fix that residual. The marker is not authenticated, probes are
finite and first-enrollment references do not prove historical vector provenance.
These are retained limits, not reasons to add new infrastructure to this change.

No new process cache, capacity/lifetime/LRU, retained vectors, runtime descriptor,
epoch revocation, fork qualification, immutable precision, owned asset snapshot,
context-health machinery, every-call checks, mean advisories or recovery reordering.
Pinned built-in Mean, device selection, loader/global precision and automatic
mean pin/recompute/recovery behavior stay unchanged. Existing fingerprint changes
such as a rewritten mean still have their existing cache consequences.

Source witnesses at `ae231e3c8`, under engine `src/`:
`vector_equivalence.rs:119–180` (gate), `:206–242` (first baseline),
`:347–375` (fingerprint), `:387–465` (residual and marker policy), `:650–652`
(cache shortcut). The accepted TC-68 design remains the cache authority.

## Close production provider escape paths

Remove production `Caller` and `CallerWithDeviceResolution`, leaving Default/None.
No feature combination restores them to normal admission. Distinct appropriately
contained test/evaluation openers remain usable, including fake providers needed
to test ordinary persistent-cache behavior. Public low-level traits, constructors,
Nomic/ONNX evaluation and Noop fixtures need only the containment/annotations
necessary to prevent production engine attachment; no gratuitous feature cleanup.
Supported standalone CLS helpers remain distinct from engine Mean, and device
choice remains supported under existing policy.

For new projection declarations, accept omitted/null `embedder` only; reject all
strings before durable mutation. Preserve historical stored strings for truthful
no-provider inspection, never substitute BGE or relabel data. Unsupported historic
read-to-configure is intentionally refused. Keep identity out of vector configs.
The linked inventory gives source witnesses and per-seam dispositions.

## Typed refusal and cleanup

Distinguish numeric mismatch, cannot-verify causes (provider error, panic,
deadline/admission failure, malformed output), and invalid-state/storage causes.
Carry typed causes through the checker rather than parse reason text. Update
initial subscriber, Rust, Python, TypeScript and CLI error reporting together.
No actual refusal returns an attached-but-refused Engine, and no automatic retry
selects None. A cache hit remains acceptance; later operation errors retain their
existing behavior, not a new guarantee that an opened provider never fails.

Establish `OpenPostProbeGuard` ownership before the new rejection return
(`open.rs:867–878`). Preserve release of SQLite/readers/callbacks/admission lock,
existing single bounded provider cleanup budget, original primary error and
late-result discard. A provider still running may retain only existing
provider/request/accounting state, never database ownership. No new reaper,
provider replacement, repeated budget or indefinite join is introduced.

### First-baseline publication

Registered first population currently persists references before confirming;
prospective preflight already confirms staged references first
(`vector_equivalence.rs:206–242`). Propose reusing that stage/check/persist
ordering for registered first population so an unsuccessful new open cannot
seed later admission with its newly rejected references. This is the minimal
failure-atomicity adjustment directly supporting failed-open behavior. Preserve
ordinary 90-call population, numeric checks and existing registered/prospective
marker-error handling. Existing references are not replaced by a failing backend.
Test confirmation/storage failure, accepted retry and atomic publication.

This is not whole-open rollback. Ordinary startup maintenance, migrations and
mean recovery can already precede verification; preserve their ordering and
durable effects. Do not promise byte-identical files after failed open or add a
schema migration/enrollment redesign for this error-policy change.

## Explicit no-provider access and recovery limits

Use existing Rust None and SDK default-off selectors. For an existing store,
validate its stored profile identity/dimension as metadata rather than compare
with fictitious BGE identity. Preserve structural/integrity rejection; no-provider
access is not a corruption bypass. New-store profile creation remains unchanged.

No-provider access stays ordinary mutable admission, not forensic mode. Preserve
valid canonical/text/lifecycle/erasure operations and truthful deferred dense
work under existing rules. Mean recovery and explicit recompute retain current
behavior without a numeric verification claim. None-mode is not guaranteed to
bypass independent CUDA pool/reranker policy validation; offline structural
inspection remains the existing alternative when ordinary admission fails.

For actual compatibility refusal, a caller can restore the compatible environment
and retry against original references, or deliberately use valid no-provider
access. Existing identity-only doctor, structural checks, export and rebuild are
not numerical compatibility repair. Do not reset references, claim safe in-place
backend rotation, or invent a diagnostic command in this proposal.

**Known deferred defect:** provider-free operator rebuild can invalidate dense
state without a usable provider; current CLI recovery opens without one. This
proposal does not fix that behavior or certify it safe. It needs separate owner
disposition, not an implicit expansion into dense rebuild admission/orchestration.
No claim that no-provider access repairs dense state or completes dense backlog.

## Evidence and review gate

Existing [characterization](../experiments/slice50-refusal-baseline/README.md)
and [fresh-open measurements](../experiments/fresh-open-verification/findings.md)
remain historical evidence. The fresh-check cost is not a reason to replace a
cache the owner explicitly retains. The owner's probability below 3% is a
qualitative planning assumption, not a measured incident rate. No new measurement
campaign or pooling experiment is required.

After independent design review and owner contract/release-placement approval,
implementation requires behavior-specific RED/GREEN for boundary closure, typed
actual refusal, existing cache/gate/counts/residual, minimal first-baseline
atomicity, cleanup and no-provider profile validation. Retain current mean,
loader/device/global precision behavior. Independent code review and the final
strict full gate apply to approved code work; Markdown-only design work uses
scoped validators. The narrowed implementation is authorized at `seq-359`; excluded redesigns remain unauthorized.
