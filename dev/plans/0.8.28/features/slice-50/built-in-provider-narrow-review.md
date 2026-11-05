---
title: Narrow built-in-provider contract — independent Astra-medium review
status: APPROVE_FOR_OWNER_REVIEW; no implementation authorization
target_release: 0.8.28
observed_on: 2026-11-01
---

# Independent review of the narrow contract

Author of the revision: continued `astra_medium_reuse_review`, `gpt-6-astra`
medium. Separate independent reviewer: fresh
`astra_medium_narrow_contract_review`, `gpt-6-astra` medium, with no authoring
history inherited. The author stopped writing before review began. Review was
read-only against the uncommitted narrow revision based on
`ae231e3c87727a9ff1cefdc4a03c6f290e62e9d4`, under owner scope `seq-357`.

**Verdict: APPROVE_FOR_OWNER_REVIEW. No P1/P2 findings. No corrections required.**

The reviewer read the complete [design](built-in-provider-design.md),
[requirements/AC](built-in-provider-requirements.md), [plan](built-in-provider-plan.md)
and [successor ADR](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md),
plus direction/status/index text and historical-review supersession banners.
Source checks confirmed:

- TC-68 persistent acceptance, fingerprint, gate, ordinary 90/45/0 counts,
  restart reuse and explicit backend-drift residual remain unchanged.
- Production caller variants and string selectors close while contained fake
  tests, low-level helpers, standalone CLS and device choice remain.
- Actual refusal becomes typed failed open through existing cleanup ownership
  (`open.rs:867–878`; guards `:14–153`).
- Registered first-baseline staging is limited to rejected-open atomicity,
  justified by existing registered/prospective ordering differences
  (`vector_equivalence.rs:206–242`). Marker-error differences and startup
  recovery effects remain explicit; whole-open rollback is not promised.
- No-provider admission corrects fictitious BGE comparison without promising
  corruption bypass or repaired provider-free rebuild.
- Accepted provider/degraded-open clauses are superseded only upon owner approval;
  other obligations and behavior-specific failing-first evidence remain.

The excluded cache/runtime/mean redesign has not re-entered the contract.
The known no-provider rebuild defect remains disclosed for separate owner
disposition. The reviewer ran no tests or validators and changed no files.
This is independent design review for owner consideration, not code review,
contract acceptance, release placement or permission to implement.
