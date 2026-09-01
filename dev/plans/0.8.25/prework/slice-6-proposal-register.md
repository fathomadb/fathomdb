---
title: 0.8.25 Slice 6 — proposal register
status: DECIDED
target_release: 0.8.25
observed_on: 2026-08-31
---

# Slice 6 — proposal register

This is the complete, deduplicated decision register for Slices 0–5. A
recommendation is not authorization. The owner may approve all recommended
dispositions in one ruling or name exceptions by ID; either form produces an
explicit disposition for every row.

## Scoring key

- **Understood:** high, medium, or low. Medium/low names the missing fact.
- **Risk:** critical, high, medium, or low, considering both change and
  non-change.
- **Effort:** XS, S, M, L, or XL, with the principal cost driver.
- **Disposition:** include in 0.8.25, postpone, reject, or retain/no change.

## Repository-preparation proposals

Only rows whose destination is Slice 7 may enter the prework implementation
plan.

| ID | Proposal and source IDs | Understood | Risk | Effort | Recommended disposition | Destination |
| --- | --- | --- | --- | --- | --- | --- |
| P25-01 | Add a release-worktree-built wheel plus fresh external-venv verification route; prove imported Python and native-library provenance. Sources: P25-ENV-01, V25-02. | High | **High:** current shim can test `main` while appearing to test the release. | M; packaging, isolated install, provenance assertions. | **Include.** | Slice 7 |
| P25-02 | Retain npm 11.12.1 as the manifest/release contract; do not rewrite it to match this host's 11.19.0. Sources: P25-ENV-03, Slice 1 pin review. | High | Low; changing the pin creates release-tool drift, while a local mismatch is not release evidence. | XS; decision only. | **Retain/no change.** | Existing release workflow |
| P25-03 | Require Windows CPU/native SDK parity in owning feature slices; keep Windows CUDA outside 0.8.25 absent a concrete route. Source: P25-ENV-07. | High | Medium; silently claiming Windows CUDA would be false, while CPU parity is already a product obligation. | XS now; Windows CPU proof remains feature-local. | **Include CPU parity; postpone Windows CUDA.** | Slices 15–75 / future CUDA release |
| P25-04 | Generalize release-state/view tooling so release-branch completion is representable without falsely claiming `origin/main` reachability or special-casing 0.8.25. Source: P25-INFRA-03. | Medium; exact schema change needs RED-first design. | **High:** current honest state makes strict board checks red; a careless change could falsify historical boards. | M; state schema, renderer, fixtures, migration of only the live state. | **Include.** | Slice 7 |
| P25-05 | Rebuild Rust in a fresh release-bound target directory, assert the removed `/tmp` worktree path is absent from executable/dep metadata, and rerun the unchanged serial suite. Sources: P25-INFRA-04, V25-06. | High | **High:** retained binaries produce false product failures and non-reproducible verification. | M; full rebuild and serial workspace runtime. | **Include.** | Slice 7 |
| P25-06 | Patch lock-compatible `crossbeam-epoch`, `anyhow`, `event-listener`, and `memmap2` advisories without crossing Candle/SQLite/ORT/binding pins. Source: Slice 1 required updates. | High | **High:** one vulnerability and three unsoundness advisories remain; resolver movement can affect native/GPU code. | M; RED policy fixture, lock update, all-feature CPU/CUDA verification. | **Include as one bounded security package.** | Slice 7 |
| P25-07 | Upgrade the test-only `httpmock 0.7.0` path to 0.8.3, which upstream changed from `async-std` to Tokio. Source: Slice 1 RUSTSEC-2025-0052 and the P25-07 follow-up in `slice-6-hitl-decisions.md`. | High; upstream declares no expected 0.8.0 API break, current MSRV is compatible, and the use is isolated to one loader-test file. | Medium; test-oracle/source or lock churn versus retaining a discontinued dependency. | M; RED dependency-policy fixture, manifest/lock change, loader tests and tree proof. | **Include**, stopping if source compatibility fails or product pins move. | Slice 7 |
| P25-08 | Remove transitive `paste` by changing the Candle/GEMM/tokenizers stack. Source: Slice 1 RUSTSEC-2024-0436. | Medium; coupled upstream route is unknown. | Medium; unmaintained transitive crate, but blind removal risks core embedding platforms. | L/XL; coupled native-stack migration. | **Postpone.** | Candle/platform backlog |
| P25-09 | Update Pyright 1.1.410 to 1.1.411. Source: Slice 1. | High | Low; exact diagnostic pin could change, with no security driver. | S; guard, typecheck, clean-environment proof. | **Postpone** to avoid coupling optional maintenance to security work. | Tooling backlog |
| P25-10 | Update Ruff 0.15.17 to 0.16.5. Source: Slice 1. | High | Medium; lint/output churn with no security driver. | M; output review and broad fixes may follow. | **Postpone.** | Tooling backlog |
| P25-11 | Replace trivial Rust/Python/schema property scaffolds with real current-contract invariants, preserving visible RED then GREEN. Source: V25-01. | High | Medium; leaving vacuous properties violates repository policy, while overly broad fixtures can duplicate feature work. | M; bounded codec/reopen/registry invariants. | **Include.** | Slice 7 |
| P25-12 | Add a bounded traceability consistency check and reconcile only proven stale NEED-026 / REQ-067 / AC-077 mappings. Sources: V25-03 and Slice 3. | Medium; NEED-026 wording needs owner approval below. | Medium; active canonical prose is inconsistent, but broad rewrites could damage historical authority. | M; checker fixture plus narrow docs changes. | **Include, conditional on P25-23/P25-24.** | Slice 7 |

## Documentation and evidence proposals

| ID | Proposal and source IDs | Understood | Risk | Effort | Recommended disposition | Destination |
| --- | --- | --- | --- | --- | --- | --- |
| P25-13 | Correct the active 0.8.25 plan's obsolete `/tmp` worktree path. Source: C25-01. | High | Medium misinformation risk; no product behavior. | XS | **Include.** | Slice 7 |
| P25-14 | Add a historical/superseded banner to root `0.8.23-release-todo.md`. Source: C25-02. | High | Medium; it presently contradicts the shipped release. | XS | **Include.** | Slice 7 |
| P25-15 | Mark maintained 0.8.24 navigation rows shipped/history. Source: C25-03. | High | Medium; current navigation misstates release state. | S | **Include.** | Slice 7 |
| P25-16 | Apply A25-01–A25-07 to architecture v2, make it the active versioned successor, deprecate v1 in place, and update authority indexes. Sources: C25-04/05, Slice 4. | High; all seven changes are allocated and code-grounded. | **High:** two active architectures invite contradictory implementations; premature acceptance would freeze ambiguity. | M; architecture/index edits and contract lint, no product code. | **Include.** | Slice 7 architecture close |
| P25-17 | Preserve `dev/plans/runs/**`, experiment receipts, and the completed performance program; do not bulk-delete or rewrite historical paths. Sources: C25-06/07/08. | High | **High:** deletion would lose scientific/review evidence; retention has context cost. | XS decision; a later manifest is L. | **Retain/no change; postpone any evidence-manifest cleanup.** | Historical evidence |
| P25-18 | Replace broken `/tmp` citations in active chunking guidance with committed receipts without changing conclusions. Source: C25-09. | High | Medium verifiability risk. | S | **Include.** | Slice 7 |
| P25-19 | Correct stale Dependabot comments after authenticated inventory; keep automated PR creation paused. Source: C25-10 and Slice 1. | High | Low/medium; stale claims mislead, re-enabling changes maintenance policy. | S | **Include comment correction; retain paused policy.** | Slice 7 |
| P25-20 | Make only bounded current-authority corrections in maintained documentation indexes. Source: C25-11. | High | Medium churn risk if widened. | S; serial with P25-13–P25-19. | **Include.** | Slice 7 |
| P25-21 | Preserve frozen progress/history, intentional compatibility branches, committed workflow fixtures, prune ledgers, and ignored local links/caches. Sources: C25-12–C25-16. | High | **High if removed:** references, recovery compatibility, or fixtures could break. | XS decision only. | **Retain/no change.** | Existing historical/source policy |

## Product-contract and feature-scope proposals

These rows authorize or postpone 0.8.25 feature planning. They do not authorize
Slice 7 to implement product behavior.

| ID | Proposal and source IDs | Understood | Risk | Effort | Recommended disposition | Destination |
| --- | --- | --- | --- | --- | --- | --- |
| P25-22 | Retain draft needs N25-01–N25-04 and local R25/AC25 packages for measurement, identity, dependencies, actuation, lifecycle, snapshots, readiness, pages, evidence, tracing, graph, selection, and closure. Source: Slice 3 exact allocation. | High; each has a falsifiable owning-slice proof package. | **Critical if omitted:** Memex/data-plane obligations become untracked; release remains intentionally overweight until per-slice review. | XL release total; already sequenced Slices 10–75. | **Include all in 0.8.25**, subject to each slice's separate draft-to-ready gate. | Slices 10–75 |
| P25-23 | Restore a concise NEED-026 security-hardening statement in `dev/needs.md` matching the existing trace row rather than deleting the trace claim. Source: Slice 3 NEED-026 finding. | Medium; intent is explicit in traceability, exact locked-document wording is not. | Medium; omission leaves false traceability, while careless wording can widen a locked need. | S; exact wording review plus checker. | **Include** as a narrow consistency correction. | Slice 7 via P25-12 |
| P25-24 | Preserve REQ-067/AC-077 as historical placeholders and add successor pointers to Slices 10/65/70/75; do not invent thresholds. Source: Slice 3. | High | Medium; stale “future IR-1/IR-2” prose misroutes readers, but rewriting history would be worse. | S | **Include.** | Slice 7 via P25-12 |
| P25-25 | Adopt A25-01–A25-07 as mandatory feature constraints: observable snapshots, UTF-8 revision-bound locators, operational-state naming, bounded liveness grammar, wire evolution, pre-truncation constraints, and opt-in visibility-bound evidence. Source: Slice 4. | High | **High:** these close concrete ambiguity and safety gaps. | No extra release slice; cost is inside Slices 15–60. | **Include all.** | Slices 15–60; architecture close in P25-16 |
| P25-26 | Keep the fast agent gate bounded; require feature-local long/live/GPU/platform routes and make measurement-layer classification executable. Sources: V25-04/V25-05. | High | High if long/platform evidence is mistaken for a fast-gate pass. | M in Slice 10 plus per-feature runs. | **Include; no Slice 7 gate expansion.** | Slices 10–75 |

## Recommended accepted Slice 7 set

The accepted Slice 7 scope contains exactly P25-01, P25-04 through P25-07,
P25-11 through P25-16, P25-18 through P25-20, P25-23, and P25-24.
P25-23/P25-24 supply the bounded contract decisions for P25-12, while P25-25
supplies the architecture corrections for P25-16.

Slice 7 does not implement P25-22, P25-25, or P25-26 product work. It does not
upgrade Ruff/Pyright, change the Candle stack, delete history, enable
Dependabot, add Windows CUDA, publish, tag, mutate registries, or run hosted
workflows.

## HITL decision framing

**Situation.** Slices 0–5 are complete on `release/0.8.25` at `51043e20`; the
release-state file truthfully selects Slice 6 and records no previously open
decision. The current strict gate has two known infrastructure reds: branch-only
completion cannot be represented by generic state tooling (P25-04), and the
relocated worktree retained binaries compiled against a removed path (P25-05).
Product obligations are already allocated to Slices 10–75 and remain separate
from prework implementation.

**Question.** Approve the recommended disposition for every P25-01 through
P25-26 row, or name exceptions by ID.

**Options.**

1. **Approve recommendations (recommended).** Authorizes the bounded Slice 7
   set above, keeps all product obligations in their owning slices, and records
   every postpone/no-change boundary.
2. **Approve with named exceptions.** State each ID and one of include,
   postpone, reject, retain/no change, or needs more information.
3. **Needs more information.** Name the affected IDs; Slice 6 remains open and
   no Slice 7 plan is written for them.

**Why option 1 over option 2 without exceptions.** It resolves known false-test
and authority risks before feature work while excluding the broad migrations
and cleanup that do not protect the release critical path.

**What would change the recommendation.** A resolver conflict crossing a
protected native pin would move the affected P25-06 package back to HITL; proof
that a historical cohort has no unique evidence could justify a later cleanup;
or a concrete Windows CUDA route could reopen P25-03. None is required to make
the present decision.

**Blocked and reversibility.** Slice 7 and all later slices wait for this
decision. No irreversible external action is behind the gate. Documentation,
tooling, tests, and lock changes are reviewable/revertible; publication remains
separately HITL-gated.

## Sources

- `dev/plans/0.8.25/prework/slice-0-{decision-brief,environment-and-infrastructure}.md`
- `dev/plans/0.8.25/prework/slice-{1,2,3,4,5}-*.md`
- `dev/design/fathomdb-data-plane-architecture-v2.md`
- `dev/plans/fathomdb-data-plane-foldback-v2.md`
- `dev/plans/release-state-0.8.25.json`

## Decision outcome

The owner approved the recommended dispositions for P25-01 through P25-06 and
P25-08 through P25-26 at `seq-272`, with two explicit refinements: P25-17 keeps
all runs and data, and P25-20 remains narrow while completing needed maintained
index corrections. At `seq-273`, the owner included P25-07 as a test-only
`httpmock 0.8.3` upgrade with test-preservation, dependency-removal proof, and
product-pin/API stop conditions. The complete ruling is recorded in
[`slice-6-hitl-decisions.md`](slice-6-hitl-decisions.md).
