---
title: Built-in provider — narrow requirements and acceptance criteria
status: proposed; scope corrected by seq-357; independent review APPROVE_FOR_OWNER_REVIEW; owner approval pending
target_release: 0.8.28
observed_on: 2026-11-01
---

# Built-in provider requirements

Scope: owner `seq-355`, corrected by `seq-357`. This replaces the broader evidence
cache/runtime-stability proposal. The [design](built-in-provider-design.md) is
proposed, not implementation authority. Feature-local IDs below do not amend
locked `dev/acceptance.md`. Unit S remains separate; D50-1/D50-2 remain held.

## User needs

| Need | Outcome |
| --- | --- |
| N1 Supported configuration | Production engine uses pinned BGE/Mean or explicit no provider; no user embedder selection before 1.x. |
| N2 Existing correctness policy | Preserve identity/integrity/numeric checks, persistent cached acceptance and explicitly documented residuals. |
| N3 Useful failure | Actual verification refusal returns a classified error and no unusable handle; caller deliberately chooses valid no-provider access. |
| N4 Predictable cost/lifecycle | Preserve existing probe cadence, cached reopen behavior and engine-owned cleanup, including MCP idle/A/B/A. |
| N5 Bounded scope | Close escape paths without new cache/runtime infrastructure, mean policy, diagnostic command or repair promises. |

## Requirements

| ID | Contract | Needs |
| --- | --- | --- |
| BIP-R1 | Production admission exposes Default/None only. Close Caller/CallerWithDeviceResolution and reject all string projection embedder selectors before mutation; omitted/null remains valid. Distinct contained test/evaluation openers may inject fakes. Retain appropriate public low-level/standalone helpers without gratuitous feature cleanup. | N1, N5 |
| BIP-R2 | Preserve existing probe gate, structural validation, identity/dimension checks, 45-probe fixture, P1 zero flips and P2 maximum uncentered L2 `1e-5`. Preserve first-enrollment residual and current no-arm/no-provider behavior. | N2 |
| BIP-R3 | Preserve persistent TC-68 marker authority, fingerprint inputs/recipe, cache validation, recording/error behavior and cadence. No process cache, key changes, restart refresh, retained outputs, TTL/LRU/epoch or diagnostic revocation. A valid marker can still hide same-identity backend drift. | N2, N4 |
| BIP-R4 | Preserve built-in Mean, device policy, loader construction, global precision behavior, context-error handling, automatic mean pin/recovery and explicit recompute semantics. No new immutable runtime or continuing numerical guarantee. | N2, N4, N5 |
| BIP-R5 | Convert actual checker refusal to typed failed open/no Engine, preserving causes and subscriber/binding reporting. Accepted cached opens and inapplicable checks remain accepted. Later runtime errors retain existing operation behavior; no automatic fallback. | N3 |
| BIP-R6 | Establish existing guard ownership before refusal return. Release database/readers/callback/lock ownership and preserve existing bounded dispatch cleanup and late-result discard; retain original error. | N3, N4 |
| BIP-R7 | Stage and confirm newly collected registered-arm references before publication, using existing prospective-path discipline, so rejected first population cannot seed later admission. Preserve existing references and marker-error rules; no whole-open rollback or mean-recovery reorder. | N2, N3 |
| BIP-R8 | Explicit no-provider admission validates existing stored profile metadata without fictitious BGE identity or corruption bypass. Preserve canonical/text/lifecycle/erasure/deferred-work behavior. Do not certify existing provider-free dense rebuild safe. | N3, N5 |
| BIP-R9 | Update superseded provider promises and inventory annotations with source witnesses. Defer diagnostic CLI, rebuild defect/orchestration and stronger runtime guarantees for separate owner disposition. Prior broad reviews are historical; require a separate review of this revision before owner contract/release-placement approval. | N1, N5 |

## Proposed acceptance criteria

| ID | Requirements | Evidence required after approval |
| --- | --- | --- |
| BIP-A1 | R1 | Consumer/API tests reject production Caller/CallerWithDeviceResolution even with unified support features. Needed test/evaluation injection remains distinct and absent from shipped normal admission. Standalone CLS, low-level helpers and CPU/CUDA selection remain usable under their existing contracts. |
| BIP-A2 | R1, R8 | Each binding rejects every string projection selector, including empty/default/model names, before durable mutation. Omitted/null declarations round-trip. Historical strings/profile metadata remain truthful on no-provider reads and cannot silently select BGE. |
| BIP-A3 | R2, R3 | Existing checker/cache regression suite retains thresholds, structural-validation ordering, exact fingerprint/recipe and marker semantics. Ordinary clean first population performs 90 embeddings, fresh existing-baseline check 45, valid cached open zero; zero on current vectorless/no-provider paths. |
| BIP-A4 | R2, R3 | A valid persisted marker still accepts across process restart and unchanged MCP idle/A/B/A reopen with zero probes. Missing/stale/malformed marker and changes to existing fingerprint inputs keep current behavior. Marker-write failure does not turn a verified pass into failure. Same-identity backend-drift residual test remains explicit and passing; no new baseline-only/no-kinds probe gate. |
| BIP-A5 | R4 | Existing loader/device/global-precision and mean pin/recompute/recovery tests retain behavior; source/dependency review confirms no fork, owned-asset snapshot, descriptor, retained outputs, cache capacity/epoch, context-health machinery or mean advisory/reorder. No every-call numeric check. |
| BIP-A6 | R5, R6 | Separate failing-first cases for actual mismatch, provider error, panic, timeout and malformed result produce typed causes, no Engine, and subscriber/binding parity. Cached acceptance is not forcibly rechecked. Database ownership is reacquirable; original error and bounded cleanup survive slow/late provider completion. |
| BIP-A7 | R7 | Registered first-population confirmation failure leaves no newly rejected references or success marker; prospective behavior stays unchanged. Existing references survive refusal, registered/prospective marker handling remains as before, successful population retains 90 calls. Storage-failure tests prove atomic reference publication; do not assert whole-open byte identity or undo earlier recovery. |
| BIP-A8 | R8 | Real database tests cover well-formed supported/historical profiles under None, corruption refusal, valid canonical/text/lifecycle/erasure and visible deferred dense work. Existing mean/recovery behavior stays intact. Tests/docs do not imply provider-free dense rebuild is repaired. |
| BIP-A9 | R9 | Independent Astra-medium review follows author's completed revision; all retained seams have appropriate annotation/containment evidence. Record RED/GREEN and scoped checks, independent code review and final strict full gate for later approved code. Documentation-only review runs Markdown validators, not a full code gate. |

These criteria are not pass claims. Use the existing characterization and timing
records as evidence, not a new campaign. The owner's probability below 3% is a
qualitative planning assumption, not a measured failure rate. See the
[plan](built-in-provider-plan.md) for review and implementation gates.
