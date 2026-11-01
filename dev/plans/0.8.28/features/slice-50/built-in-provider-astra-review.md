---
title: Built-in provider design — Astra-medium recommendation and integration review
status: coherent proposal for owner review; no implementation approval
target_release: 0.8.28
observed_on: 2026-11-01
---

# Astra-medium design record

Reviewer/designer: `gpt-6-astra`, medium reasoning, continued agent
`astra_medium_reuse_review`. Read-only source base `14dbbd57a`; reviewed the
main thread's integrated [design](built-in-provider-design.md),
[requirements/AC](built-in-provider-requirements.md),
[plan](built-in-provider-plan.md) and proposed successor ADR. The agent made no
file changes. Owner scope and pre-implementation gate: steward `seq-355`.

## Recommendation

Built-in-only production admission before 1.x; close caller-provider injection
and new projection selectors, contain deferred test/evaluation seams, preserve
Mean and the finite numerical check. Add independently qualified immutable
per-runtime strict-F32 Candle execution, actual-consumed-byte construction
provenance and bounded numerical-evidence reuse. No generic provider descriptor
framework is needed for excluded pre-1.x support.

Astra proposed 16 positive entries / 2 MiB accounted payload, 69,120-byte live
probe output evidence per entry, and one evidence reference per active baselined
engine. Preserve engine-owned model/dispatch lifetime. Cache-wide epoch clear
fences failures/late publication without unbounded tombstones. Context health
is engine-owned and validated at admission/execution/evidence-use boundaries.

## Design refinement

Astra initially considered terminal projection failure for a rejected automatic
mean pin. The main thread challenged that it could recreate the characterized
drain/nonembedding stalls. Astra revised the recommendation: reject only the
optional candidate, continue under the accepted old mean, expose a bounded
sticky advisory and suppress automatic retries per accepted-state token. Apply
this rule to open-time automatic recovery too; current-state verification
failure still fails open. Explicit recompute returns typed rollback failure.
This is an explicit proposed successor policy, not a silent relaxation.

Preserve the accepted unbaselined first-enrollment residual. Existing references
make provider-enabled open relevant even if all dense kinds were removed.
Canonical path plus freshly validated comparison inputs scopes reuse; identical
comparison-state replacement can reuse evidence without a provenance claim.

## Final integration verdict and disposition

**Coherent proposal for owner review.** Astra's final read identified three
remaining concrete consistency edits, all applied:

1. Diagnosis preserves database/reference state but may invalidate/repopulate
   process evidence; remove the contradictory cache-immutability wording.
2. Validate current committed state before compatibility-conditioned optional
   mean recovery; remove stale post-recovery ordering in the state table.
3. Baseline-bearing ordinary no-provider open defers automatic mean recovery;
   explicit no-provider recompute remains available without a numeric verdict.

Astra reported no further substantive inconsistency. This is a design review,
not code review, test execution, release placement, acceptance of the successor
ADR or implementation authorization. Behavior-specific tests and the full gate
remain future implementation requirements. The scope ruling is recorded; the
owner's detailed contract review remains open.
