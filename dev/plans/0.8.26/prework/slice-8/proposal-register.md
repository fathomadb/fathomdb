---
title: FathomDB 0.8.26 Slice 8 — scored proposal register
status: AWAITING_HITL
observed_on: 2026-09-12
last_updated: 2026-09-13
---

# Slice 8 scored proposal register

## Authority and settled boundary

`scripts/release-current.py` selects 0.8.25. Its machine-readable
`decisions.unruled` array is empty. The 0.8.26 state does not yet exist because
Slices 0–7 deliberately deferred activation. The decisions below are therefore
new Slice 8 items, not reopened 0.8.25 rulings.

The repository owner's existing direction already settles these points:

- 0.8.26 implements the Memex P0, P1, and P2 capabilities in the prioritization
  scaffold; Priority 3+ requests remain outside the release;
- execution uses direct agents, not Steward or Orchestrator roles;
- Slice 8 decides scope and placement but performs no implementation; and
- publication remains separately gated and is not authorized.

## Scored proposals

Risk is implementation stability risk, not the severity of leaving a problem
unfixed. Recommendations are proposals pending HITL ruling.

| ID | Proposal | Value | Understood | Risk | Effort | Recommendation | Placement |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P26-01 | Repair the inconsistent 0.8.25 publication receipt/state and add lifecycle validation | restores truthful verifier and release authority | high | medium | S | include | 9 |
| P26-02 | Create 0.8.26 state/board only after P26-01, then prove release preflight | enables governed slice execution | high | low | S | include | 9 |
| P26-03 | Make preflight select an explicit authoritative main ref; cover stale-local/remote-only/offline cases | removes misleading stale-base evidence | high | low | S | include | 9 |
| P26-04 | Establish checkout-owned dev tools and fresh artifact environments; add disk budget before full matrix | prevents stale environment and ENOSPC recurrence | high | low | S | include narrowly | 9 setup + 50 budget |
| P26-05 | Correct maintained authority/platform/CLI inventory documentation found by Slice 2 | removes stale operator/developer truth | high | low | S | include exact files only | 9 and owning feature docs |
| P26-06 | Attempt bounded remediation of root markdownlint/`smol-toml` high advisory | supply-chain hygiene on a gating tool | high | medium | S–M | include investigation; adopt only with AST/lint proof | 9 |
| P26-07 | Upgrade uncovered Mermaid/DOMPurify toolchain | developer-tool security | medium | medium | S | postpone from feature release | separate housekeeping |
| P26-08 | Correct download-artifact v4.3.0 comments now; defer v8 runtime upgrade pending all-runner proof | restores pin truth without runtime churn | high | low for comment; medium-high for runtime | XS / M | split: include comment, postpone runtime | 9 / later |
| P26-09 | Derive Windows structural and installed-binding inventories from one machine-readable contract | prevents repeated candidate-only mismatch | high | medium | M | include | 9 |
| P26-10 | Add Windows runner prerequisite probe | may prevent host-drift reruns | medium | medium | M | needs more information | post-10 if confirmed |
| P26-11 | Register versioned benign digest evidence through exact, mutation-tested release inputs | avoids Gitleaks closeout false positives without broad suppression | high | medium | S | include checklist/input work, not generalized suppression | 50 |
| P26-12 | Triage 21 unknown full-history Gitleaks fingerprints | reduces persistent advisory debt | medium-high | high | M | include outside critical feature path | post-10 security slice |
| P26-13 | Poll/retry only exact-version-unavailable registry responses before package-smoke fan-out | removes known PyPI/npm propagation reruns | high | low-medium | M | include | 50/release |
| P26-14 | Reuse an in-progress exact-SHA candidate workflow | possible runner savings | low | medium | M | postpone pending proof dispatches were accidental | later |
| P26-15 | Broad Cargo refresh or immediate `paste` replacement | general maintenance, no current RustSec vulnerability | low-medium | high | L | postpone; investigate only with Candle driver | later library sweep |
| P26-16 | Ruff, Pyright, `@types/node`, or major TS/N-API housekeeping | tool currency, no current product driver | low | medium | S–L | postpone | later library sweep |
| P26-17 | Delete candidate run logs or Prettier config | repository size/navigation | low | medium-high | M | postpone until per-target proof | later cleanup |
| P26-18 | Move historical 0.8.25 plans/evidence | little functional value; path risk | low | medium | M | reject movement; keep/archive in place | none |
| P26-19 | Install every cross-target locally during prework | no added release truth over named executors | low | high | L | reject | use owning feature/Slice 50 executors |

## Feature decisions

### D26-01 — exact graph-evidence public shape

- **Draft HITL position:** Generally accepts the recommendation, not final.

- **Situation:** Graph traversal already reads exact target and terminal-edge
  revisions in one reader transaction, while `GraphTargetV1` is closed across
  dynamic bindings and fixtures. Exact evidence resolution can reuse the
  existing frozen eligibility and nondisclosure machinery.
- **Question:** Should Slice 20 preserve V1 and add a versioned evidence sidecar
  or successor graph result?
- **Options:** (A) additive sidecar/successor result; (B) add required fields to
  `GraphTargetV1`; (C) defer exact graph evidence.
- **Recommendation:** A, conditional on a named Slice 15 performance and
  erasure-linearization spike before Slice 20. It satisfies Memex without the
  binding and fixture breakage of B; C leaves graph evidence gated. See
  [`spike-d26-01-graph-evidence-impact.md`](spike-d26-01-graph-evidence-impact.md).
- **What changes it:** Evidence that every V1 decoder tolerates the exact field
  addition and that no fixture/schema contract closes the shape; cheaply
  checkable, but current code evidence points the other way.
- **Blocked/reversible:** Slice 20 design/implementation is blocked. No
  irreversible action has occurred.

### D26-02 — operator integrity delivery

- **Situation:** The CLI command, crates.io distribution, registry smoke, and
  docs already exist. The unresolved questions are whether Memex can deploy
  that artifact and whether normal `Engine::open` makes the full process
  observably non-mutating.
- **Question:** Should Slice 30 qualify/harden the existing crates.io CLI first,
  adding a prebuilt artifact only if Memex cannot deploy it?
- **Options:** (A) qualify existing CLI with conditional prebuilt fallback; (B)
  commit now to a new prebuilt matrix; (C) expose an SDK doctor method; (D)
  defer operator integrity qualification.
- **Recommendation:** A. It reuses the shipped boundary and minimizes packaging
  and authority risk. B is premature; C expands governed authority.
- **What changes it:** A confirmed Memex deployment constraint that forbids a
  crates.io-installed Rust CLI. This requires the owner's knowledge or a cheap
  deployment check.
- **Blocked/reversible:** Slice 30 scope is blocked. Packaging remains fully
  reversible and no artifact has been published.

### D26-03 — derived-edge actuation version

- **HITL ruling:** Breaking V2-only actuation and fresh databases, `seq-282`.

- **Situation:** Ordinary provenance-bearing edge writes already exist, but the
  closed V1 actuation grammar has no edge operation. Reinterpreting V1 would
  change encoding, digest, replay, and cross-binding contracts.
- **Decision:** Replace V1 with one full V2 grammar containing the inherited
  operation capabilities plus `PutDerivedEdge`; keep one actuation method per
  binding. V1-shaped dynamic ingress may only return a loud V2 direction and
  must not parse, translate, execute, digest, or replay V1. Carry no V1 receipt,
  integrity, data, or operation-ID compatibility. Accept fresh 0.8.26 databases
  only and provide no migration from earlier versions.
- **Placement:** Slice 35 proves the V2/fresh-database contract and performance;
  Slice 40 implements it. See accepted
  [`ADR-0.8.26-breaking-v2-actuation-and-fresh-database-boundary.md`](../../../../adr/ADR-0.8.26-breaking-v2-actuation-and-fresh-database-boundary.md).
- **Blocked/reversible:** D26-03 no longer blocks design. D26-04 endpoint
  semantics and D26-05's exact V2 receipt remain open. Publication and product
  implementation remain unauthorized.

### D26-04 — derived-edge endpoint semantics

- **Draft HITL position:** Generally accepts option B, not final and coupled to
  the reframed D26-05 V2 receipt decision.

- **Situation:** Ordinary edge writes intentionally flag/count dangling edges
  and evaluate the complete batch, including later operations. A stricter
  governed-derived-edge refusal would be a new semantic distinction.
- **Question:** Should derived-edge actuation preserve ordinary dangling-edge
  behavior or refuse missing endpoints?
- **Options:** (A) preserve flag/count semantics over complete prospective
  state; (B) require endpoints over complete prospective state; (C) require
  earlier-operation order.
- **Recommendation:** B. Ordinary flag/count behavior cannot be truthfully
  represented without a dangling count. Under the fresh-database ruling, B
  still gives the smallest V2 receipt and accepts endpoints anywhere in the
  complete batch. A remains available only if Memex explicitly needs
  incomplete-graph admission; C adds needless ordering.
- **What changes it:** A Memex requirement that the governed batch itself—not
  Memex validation—must reject incomplete endpoint sets.
- **Blocked/reversible:** Slice 40 transaction tests and error contract are
  blocked. B would create a durable behavioral distinction; C is rejected by
  current architecture evidence.

### D26-05 — receipt and operation-ID evolution

- **HITL status:** Open and reframed by `seq-282`; the prior draft acceptance of
  current receipt storage is no longer operative.

- **Situation:** 0.8.26 needs one V2 receipt, replay, integrity, and operation-ID
  contract for a fresh database. It must not retain V1 receipt readers,
  integrity checks, replay, collision rules, or migration code.
- **Question:** What is the minimum truthful V2 receipt for committed and
  refused derived-edge batches under the chosen D26-04 endpoint policy?
- **Options:** (A) create `ActuationReceiptV2` with the current compact outcome,
  affected-revision, boundary, projection, generation, closure, and V2
  source-reference concepts plus only edge fields proven necessary; (B) add a
  broad graph-consequence manifest; (C) return no durable receipt.
- **Recommendation:** A. It preserves idempotent V2 replay and bounded audit
  truth without any historical compatibility or Memex semantic inference. B
  is overbuilt; C loses the core actuation guarantee.
- **What changes it:** Proof that a named current concept is unnecessary for
  V2 replay/integrity, or that D26-04 option A requires one explicit dangling
  count.
- **Blocked/reversible:** Exact Slice 35/40 receipt design remains blocked. No
  V1 storage constrains the choice, and no migration is permitted.

## Preparation bundle decision

### D26-06 — Slice 9 preparation scope

- **Draft HITL position:** Generally accepts the revised narrow option A, not
  final.

- **Situation:** The owner's narrow-release ruling removes unrelated dependency
  remediation from the default bundle. P26-01 and P26-02 are mandatory release
  truth; P26-03, exact P26-05 corrections, P26-08 comment truth, and P26-09 are
  bounded corrections to known release failures. P26-04 is an execution
  condition, not product scope.
- **Question:** Should Slice 9 implement P26-01, P26-02, P26-03, exact P26-05,
  P26-08 comment truth, and P26-09, with P26-04 enforced as execution
  conditions?
- **Options:** (A) revised narrow bundle; (B) ultra-narrow P26-01 and P26-02
  only; (C) name individual exceptions; (D) no preparation.
- **Recommendation:** A. It removes deterministic release blockers and repeated
  candidate failures without dependency or cleanup churn. P26-06 is postponed
  unless it becomes a hard build blocker.
- **What changes it:** A requirement to minimize any pre-feature diff beyond
  release-state activation, or evidence that the markdown advisory has no
  viable bounded fix.
- **Blocked/reversible:** Slice 9 plan replacement and all later slices are
  blocked. Changes are repository-local and reversible; state history must
  remain truthful rather than be rewritten.

## Later-delivery bundle decision

### D26-07 — hardening and release-boundary scope

- **Draft HITL position:** Generally accepts the revised option A, not final.

- **Situation:** P26-11 through P26-13 have clear value but belong after
  features or at final artifact/release boundaries. P26-10 and P26-14 lack
  enough evidence for implementation.
- **Question:** Should Slice 50 include P26-11 exact generated-evidence digest
  registration and P26-13 registry visibility polling logic, while postponing
  P26-10, P26-12, and P26-14?
- **Options:** (A) revised narrow placement; (B) postpone all five; (C) choose
  individual exceptions.
- **Recommendation:** A. Slice 50 is the final integrated non-publishing
  verification slice, not publication itself. Actual registry exercise and
  publication remain a separate gate. This placement prevents known release
  friction without pulling unrelated full-history security triage or
  speculative runner work into 0.8.26.
- **What changes it:** Confirmation that Windows runner drift or duplicate
  dispatch is recurring on current infrastructure, which would promote the
  relevant item.
- **Blocked/reversible:** Reserved-slice and Slice 50 planning are blocked, but
  feature implementation need not wait once the placements are recorded.

## Maintenance disposition decision

### D26-08 — non-feature maintenance

- **Situation:** P26-07 and P26-15 through P26-17 have no immediate product
  driver and carry disproportionate regression or evidence-loss risk. P26-18
  and P26-19 have no positive value in this release.
- **Question:** Should 0.8.26 postpone P26-07 and P26-15–17, reject P26-18–19,
  and preserve all historical evidence until a separate bounded effort?
- **Options:** (A) recommended postpone/reject split; (B) include selected
  maintenance items; (C) include all cleanup and dependency work.
- **Recommendation:** A. It keeps the release focused on Memex and deterministic
  release blockers. The runner-up—selected maintenance—only wins if a current
  shipped vulnerability or hard build blocker appears.
- **What changes it:** A new shipped advisory, toolchain incompatibility, or
  proven unreachable evidence group.
- **Blocked/reversible:** Nothing in the feature path is blocked; this controls
  scope and prevents premature deletion.

## Requested HITL response

Reply with rulings for `D26-01` through `D26-08`. “Approve all recommendations”
is sufficient. Any exception can name the ID and selected option or wording.
After the explicit response, the rulings will be recorded durably, Slice 9 will
be replaced and independently reviewed, and the reviewed package will return
for final execution approval.
