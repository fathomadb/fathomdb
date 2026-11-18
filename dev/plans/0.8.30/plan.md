---
title: FathomDB 0.8.30 release plan — correctness, performance, latency and robustness
date: 2026-11-17
status: PROPOSED
target_release: 0.8.30
---

# FathomDB 0.8.30 release plan

Find and fix wrong results and unsafe lifecycle behavior, improve measured
end-to-end latency, and reduce the time developers wait for actionable failures.
Make ordinary publication finish through the same trusted mechanism that worked
in recovery. The two investigations below have bounded exits even when their
first-wave hypotheses yield no confirmed product defect.

The [testing and robustness findings](testing-robustness-findings.md) cover
0.8.27 and 0.8.28. The [0.8.29 retrospective](../../notes/0.8.29-retrospective.md)
and [engineering recommendations](quality-performance-robustness-recommendations.md)
identify the more recent observations. This draft follows an independent
`gpt-6.2-astra` medium review of the findings and existing allocations. Execution,
real faults, GPU or paid campaigns, and publication retain their existing
authorization boundaries.

## Baseline and order

At activation, verify current `origin/main` against the published 0.8.29 tag
`b64d32f80` and closure `569652e61`. Start from current qualified repairs and
store format 37. Reuse a result when its source behavior, features, artifact,
fixture and protocol still support the claim. Test changed native packages
through installed consumers. Preserve failed and skipped historical results.

Supplemental Tegra CUDA wheel, addon and Pages delivery remains open in 0.8.29
under seq-398. Resolve that work in .29 before accepting the final .30 Tegra
baseline. Independent .30 CPU and design work can proceed.

| Slice | Priority and dependency | Decision at exit |
| --- | --- | --- |
| 0 | Confirm baseline, first-wave selectors and existing evidence on activation. | Identify changed claims, sample limits and relevant accepted thresholds. |
| 5 | Start delivery repairs alongside 10 and the baseline for 15. | Failures survive cancellation, the normal promoter is corrected and the expensive CI path is measured. |
| 10 | Correctness bug hunt and remediation. | Classify the bounded cases and fix confirmed defects. |
| 15 | Profile before optimizing. Consume relevant 10 repairs before promotion. | Adopt a measured improvement or retain the baseline with its result. |
| 20 | Concurrency, lifecycle and resource bug hunt and remediation. | Resolve or bound the selected schedules and resource behavior. |
| 30 | Begin fork research after 0. Integrate qualified prerequisites into 20. | Qualify the selected dependency cohort and affected installed consumers. |
| 40 | Qualify stable integrated source and final artifacts. | Required checks and consequential open dispositions are visible. |
| 45 | Publish only after 40 and separate production authorization. | Verify the real registries, installed consumers, release and documentation. |

Use separate worktrees for concurrent writers and serialize shared source and
pin integration. Review substantive design and final implementation in coherent
batches. Maintain one state and board when execution starts. Finalize consumed
fork/native inputs before final performance and artifact qualification; later
changes invalidate only the claims that actually depend on them.

## Slice 5 — actionable failures, CI cost and normal finalization

- Exercise a real wrapper failure beyond 200 diagnostic lines followed by
  cancellation of a later child. Preserve the first exit and full useful
  diagnostic before continuing, completed outcomes, and the unrun tail. Bound
  capture and redact sensitive data. Check both collect-all and fail-fast callers.
- Use monotonic elapsed time in the remaining wall-clock wrapper. A clock
  adjustment must leave nonnegative durations and preserve the original exit.
- Share the qualified pinned OIDC CLI promotion mechanism between normal and
  recovery workflows. Check trust for the actual normal workflow and exact tag.
  Only the main npm package advances to `latest`. After a successful mutation,
  print expected and observed versions and poll within a fixed bound. Two stale
  reads followed by the target succeed. Persistent mismatch fails without
  republishing. Check real normal finalization in 45.
- A missing optional npm binary must fail the installed smoke with the actual
  platform, resolved dependency, native path and install diagnostic. Check
  matching workers/services before production access or approval.
- Time the existing Windows WAL source checker and tool-only bootstrap on
  matched inputs. The .29 observations were 856.43 seconds and about 8 min 50 s
  respectively. Remove recursive rescans or duplicate builds where measured.
  Tool-only setup starts no native wheel build. Invalid workflow controls and
  the real Windows WAL test still run in their appropriate jobs.
- Use job durations to distinguish queue, setup, execution and repeated builds.
  Report before/after elapsed time and affected coverage. Keep a reduction
  only with its original oracle intact. Reuse built inputs only with matching
  trusted identity. Keep ordinary CI proportional to changed paths; no new merge
  queue, required-check expansion, nightly campaign or soak period is selected.

## Slice 10 — correctness bug hunt and remediation

Start with the saved S50 case and two small real-store sentinels. Inspect existing
integration and in-crate tests before adding cases. Fix each confirmed defect
behind its own observed failing behavioral regression and rerun the affected
route. Distinguish product, fixture and operator failures.

1. Inspect the preserved natural-write SDK-hybrid sample 860/query 360/op 9971.
   Both calls used reference routing. Ordered IDs and text agreed, but two scores
   differed. Start from [the retained evidence and allocation](release-followups.md).
   For one bounded reproduction on the affected route, capture candidate IDs,
   distances, ranks, ties, cursor/snapshot and route. Compare a fixed-state
   control exactly. Different reader states need positive evidence. Preserve the
   original failure and seq-393 disposition if the cause remains unresolved.
2. Write distinct records with identical bodies and exercise repeated public refs
   through real feedback. Verify stored identity, exact attribution on accepted
   input, and zero feedback writes on a mixed valid/ambiguous request. Review the
   allocated occurrence-label contract, including response/store/epoch lifetime,
   paging, fetch, both formats and validation before any new ABI is selected.
3. Exercise coverage publication across a write and a pinned reader with a UTF-8
   boundary. Check exact known/unknown populations, `R=K+U`, byte spans and
   publication after rollback. Add a malformed native/open case where a changed
   boundary or observed error makes the next probe discriminating.

Stop after the named cases and one targeted follow-up per ambiguity unless a
specific new state predicts a different outcome. Record what ran, the observed
result and residual consequence. Expected behavior and bounded non-reproduction
are valid exits. Critical wrong results or data loss require repair or explicit
owner disposition. Fixed-state exactness and existing score criteria apply.

## Slice 15 — performance and end-to-end latency

- Profile one current 100k hybrid SDK workload and a relevant text or lifecycle
  control. Keep provider-free, SDK, cold/warm, snippet and output-format cases
  distinct. Use matched state, data, configuration, instrumentation and
  interleaved baseline/treatment runs on the ordinary CPU host.
- Measure caller-observed p50/p95/p99, throughput, refusals, timeouts and
  resources. Attribute queue wait, service, eligibility, retrieval, hydration,
  reranking and serialization on the selected path. Retain sample counts,
  outliers and host load. The .29 roughly 48–49 ms eligibility scan is a lead.
- Pick one demonstrated bottleneck. Fix the comparison rule before changing
  implementation. Compare cost and outputs on the same snapshot and include
  natural writes if reuse or invalidation is involved. Time-dependent
  eligibility can change without a generation change.
- Promote a treatment only if it improves the selected measure under that rule
  and preserves accepted correctness, tail, lifecycle and resource criteria.
  An inconclusive or negative result closes the experiment with its scope.

## Slice 20 — concurrency, lifecycle and resource bug hunt and remediation

1. At existing barriers, cancel a queued call and time out a started call, then
   close and reopen. Exercise independent engines or path aliases where they
   share an owner. Require positive work before the transition, correct capacity
   ownership, no stranded waiter, completion within the fixture watchdog and
   successful subsequent use. Preserve database quiescence before the shared
   provider-drain deadline; add no total-close latency guarantee.
2. Use the existing Node sampler under shipping defaults. Separate JS heap,
   live native ownership and allocator reservation. Compare matched ON/OFF
   cases within the selected run bound. Preserve .29 default FAIL/OFF_NOT_RUN
   and the separate semispace4 result. Fix reproducible leaks with an ownership
   regression. State remaining uncertainty when the sample cannot identify one.
3. If a changed commit/checkpoint path requires crash proof, use the existing
   child/observer seam only within its approved fault bounds. Require delivered
   interruption at the named point, then compare canonical survivors, projection
   and queue state on reopen. Record undelivered or unrun cases as such.

Fix confirmed defects with observed RED then GREEN. Stop after the selected
schedules, one matched resource comparison and any affected recovery boundary.
Expand only for a concrete missing interleaving or observed ownership discrepancy.

Review frozen native residue, no growth after known loss, opaque host bytes and
typed refusal under the [reset allocation](release-followups.md). Healthy, OOM
and software-loss results retain their scope. Orin teardown needs Orin proof.
A 3090 run serves its own claim. Actual reset needs concrete venue, resource
and session authority. Prior one-use approvals are spent.

B30-01 proposes explicit opt-in Rust/Python/TypeScript module-model release.
First qualify the cudarc post-reset drop repair before exercising or shipping
the unsafe drop path. Then test concurrent use, repeated release, typed errors,
use after release and reload with clearable ownership. Preserve engine close
semantics and select no automatic idle release. If the prerequisite remains
open, record the blocked allocation and continue independent work.
[Module-model release contract](../../ROADMAP.md#0830-backlog) remains authoritative.

## Slice 30 — allocated fork maintenance

Retain F30-REBASE, F30-MARKER and F30-MULTIARCH under seq-367. Compare K1–K9
with a justified upstream revision, preserve required .29 repairs, and qualify
one coherent cudarc family before pin integration. Check the marker separately
from the private-pool primitive. Select native/PTX targets before implementation.
Inspect actual kernel bytes, run supported and unsupported controls, and compare
cold start/JIT, size and build time. Qualify in each fork, then exercise affected
installed product consumers on the final pins. [Fork allocations](fork-maintenance-plan.md)
retain their acceptance criteria and stable identities.

Use the [dated CUDA review](research/cuda-version-adoption.md) and
[standing policy](../../design/cuda-version-support-review.md) for target choices.
Comparable usage shares remain unknown. Retain existing driver/toolkit policy.
CUDA13 delivery belongs to .31. The [overflow tripwires](../../design/overflow-chunking-reevaluation.md)
govern future chunking review. Physical erasure receives concrete defect fixes
only. This draft selects no migration, new precision, backend, schema, cache or
default. A cache treatment needs measured cost, invalidation and snapshot proof
before separate adoption; chunking and ANN are not selected.

## Testing choices and final verification

Apply the [quality roadmap](../../notes/fathomdb-quality-testing-roadmap.md)
inside the owning slices. Use real databases, small stateful properties for
projection/recovery and round trips, independent same-state reference cases,
first/persistent failures and positive concurrency progress. Use a few
wrong-order, swallowed-error or missing-cleanup mutations where an oracle is
in doubt. Keep one deep semantic owner and thin installed-binding checks.
For an expensive test, identify its timing, claim and defect caught or missed.
Move or consolidate it only when the replacement still catches that failure.

The .27 row-error repairs and .28 cursor fixture show how covered paths can
pass with wrong or empty results. The .29 score ambiguity, default Node growth
and lost failure output determine the first cases. Defer global MC/DC, general
VFS/allocator fuzzing, broad platform/paid campaigns and new test frameworks.
Their cost has no corresponding changed claim in this plan. No database mocks,
regenerated goldens, weakened thresholds or new canonical acceptance numbers.

Run scoped checks while editing. At 40 run the required full source gate on
stable clean source once per coherent code phase and the applicable security,
long, performance, platform and artifact checks. After a narrow fix, cover
failed, affected and unrun scopes. A broad change can require fresh verification.
Retain original failures and skips. Update affected interfaces, ADRs, changelog,
docs/navigation and strict docs checks. Canonical acceptance and thresholds stand.

At 45 use explicit publication authority and the immutable shipping tag. Verify
all required publishers, exact-version fresh registry installs and runtime
open/close/exit, persisted write/read and separate-process reopen, co-tagging,
normal finalization, and public Pages/downloads. Partial publication stays open.
Reuse equivalent qualification and close with one concise result identifying any
owner disposition or remaining limitation. No .30 implementation or release
activation has started through this planning task.
