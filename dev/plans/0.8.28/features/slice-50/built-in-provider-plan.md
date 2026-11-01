---
title: Built-in provider — revised narrow work plan
status: implementation approved by owner seq-359; separate Astra-medium implementation review gates landing
target_release: 0.8.28
observed_on: 2026-11-01
---

# Built-in provider work plan

Owner `seq-355` excludes user providers before 1.x; `seq-357` explicitly preserves
the existing checker and persistent cache. The previous evidence-cache, immutable
runtime and mean-policy package exceeded scope and is superseded. The owner approved the narrowed
contract and implementation at `seq-359`, with landing on `dev/0.8.28` gated by
separate Astra-medium code review. Unit S is unchanged;
D50-1/D50-2 stay held, not alternative implementation instructions.

Separate [independent Astra-medium review](built-in-provider-narrow-review.md)
completed: APPROVE_FOR_OWNER_REVIEW, no P1/P2 findings. The revised contract is
approved for implementation at `seq-359`. The [implementation handoff](built-in-provider-implementation-brief.md)
names the isolated worktree and TDD/review requirements.

## Authorized design work

1. Astra-medium revises [design](built-in-provider-design.md),
   [requirements/AC](built-in-provider-requirements.md), successor proposal and
   current-direction records. Preserve historical evidence/reviews with a scope
   supersession banner; do not change source, tests, dependencies or release state.
2. Author stops writing. A separate Astra-medium reviewer checks the complete
   revised package against `seq-357`: existing cache/gate/counts/residual,
   production boundary closure, minimal failed-open atomicity and honest recovery
   limits. The author cannot supply its own independent verdict.
3. Root integrates review corrections, runs scoped Markdown validation and brings
   the concrete proposal to the owner. No new measurement campaign is required.
4. Obtain owner approval of contract and release placement before implementation.
   Record the known provider-free rebuild defect for separate owner disposition;
   it is neither fixed nor declared safe by this proposal.

## Later implementation, only if approved

Delegate isolated implementation worktrees under repository discipline. Use
behavior-specific failing-first tests, staged for review, then implementation:

1. Close production caller variants and projection selectors; preserve contained
   test/evaluation injection and supported standalone helpers. Update interfaces.
2. Preserve checker/cache/gate/mean behavior with existing tests and focused
   cadence/residual assertions. Do not revise fingerprint inputs or cache policy.
3. Carry typed verification causes to open; stage registered first-baseline
   confirmation before publication as the minimal rejected-open atomicity change.
4. Route actual refusal through existing guard/dispatch cleanup; update subscribers
   and binding errors. Correct no-provider stored-profile validation only where
   necessary for deliberate access; no diagnostic CLI or rebuild orchestration.
5. Independent code review and final strict `agent-verify.sh` after appropriate
   scoped development checks. Existing checkout lacks its own `.venv`; setup is
   prerequisite to full verification, not permission to borrow another checkout's.

Intentional supersession of old degraded-open/provider-surface tests must be
identified explicitly; do not change tests merely to make fixes pass. Preserve
existing TC-68 residual tests, normal cached-open counts, first-enrollment and
mean semantics. Code review must reject incidental stronger guarantees or feature
cleanup. Owner `seq-359` authorizes implementation commits and the existing repository
workflow handles landing/push after independent review; no tag/publication or
later ladder work is authorized.

Existing cache regression anchors in `tc68_probe_fingerprint_cache.rs`:
`verified_reopen_performs_zero_probe_embeds`,
`a_rewritten_pinned_mean_reruns_the_full_probe`,
`an_unreadable_cache_entry_falls_back_to_running_the_probe`,
`a_missing_cache_entry_falls_back_to_running_the_probe`, and
`residual_same_identity_backend_drift_is_not_caught_on_a_cached_open`.
Preserve their cache outcomes; intentionally adapt only assertions that formerly
expected a returned refused handle to the approved typed failed-open contract.
