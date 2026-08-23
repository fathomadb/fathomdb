---
title: 0.8.24 Slice 6 — proposal register
status: AWAITING-HITL
target_release: 0.8.24
---

# Slice 6 — proposal register

This is the complete, deduplicated decision register for Slices 0–5. A
recommendation is not authorization. Every row is **awaiting initial HITL**;
only an explicit owner decision can make a Slice 7 package eligible for its
separate implementation plan.

## Scoring key

- **Understanding:** clear, needs evidence, or needs owner choice.
- **Risk:** concrete downside if the proposal is wrong or incomplete.
- **Effort:** S, M, or L; external prerequisites are called out explicitly.
- **Recommendation:** include, postpone, reject, or decision required.

## Prework maintenance candidates

| ID | Proposal and sources | Understanding / risk | Effort and dependencies | Recommendation / destination | Minimum verification | HITL decision |
| --- | --- | --- | --- | --- | --- | --- |
| P24-01 | Update root `markdownlint-cli2` and its lockfile to remove the audited `js-yaml` path. Sources: Slice 1 sweep/design; R24-11. | Clear. **Low:** a dev-tool lock update could alter guarded Markdown behavior. | S; root npm install/audit available locally. | **Include** only as a bounded Slice 7 package. | Preserve the audit baseline; named dependency/lock change; guarded Markdown checks and audit of the named path. | Awaiting initial HITL |
| P24-02 | Review Pyright `1.1.410 → 1.1.411`. Sources: Slice 1 sweep; R24-11. | Needs evidence. **Low/medium:** exact version guard and type output may change. | S; independent of P24-01. | **Postpone.** Do not bundle it with security remediation. | If later accepted: version guard plus Python typecheck and output review. | Awaiting initial HITL |
| P24-03 | Remove unused root Prettier dependency and obsolete bootstrap wording. Sources: Slice 1 sweep; Slice 2 cruft review. | Clear. **Low:** clean tooling install or non-Markdown formatter use might be disrupted. | S; direct-use scan and clean root install required. | **Include** as a separate Slice 7 package, not coupled to P24-01. | Baseline direct-use scan; clean tooling install; guarded Markdown checks. | Awaiting initial HITL |
| P24-04 | Correct former-owner links and stale reader-facing current-release assertions on maintained surfaces only. Sources: Slice 2 review/design; R24-13 / AC24-13 draft. | Clear. **Low:** a broad replacement could alter historical evidence or pre-announce 0.8.24. | S; requires maintained-path allowlist and historical exclusions. | **Include** as one bounded Slice 7 documentation package. | Baseline/green bounded scans, affected documentation checks, and link/build checks where applicable. | Awaiting initial HITL |
| P24-05 | Reconcile active engineering navigation to the release-state lookup rule. Sources: Slice 2 review/design; R24-14 / AC24-14 draft. | Clear. **Low:** a wrong hard-coded active-release claim would rot again. | S; serial with P24-04 because it edits shared indexes. | **Include** as a separate Slice 7 documentation package. | Bounded navigation/index consistency check; do not create release state merely for this check. | Awaiting initial HITL |
| P24-06 | Keep Dependabot automated PR creation paused. Sources: Slice 1 sweep. | Needs owner policy confirmation; GitHub alert state was rate-limited. **Medium:** re-enabling changes notification/review volume; remaining paused leaves manual sweep responsibility. | S decision; no code dependency. | **Keep paused / postpone policy change.** Not a Slice 7 implementation package. | No change; record owner posture. | Awaiting initial HITL |
| P24-07 | Do not archive or delete reviewed material. Sources: Slice 2 review; R24-15 / AC24-15 draft. | Clear. **Low:** deletion would threaten retained evidence and references. | None. | **Reject deletion/archive for 0.8.24.** | No change. Any later candidate must meet R24-15 safeguards. | Awaiting initial HITL |

## Feature-scope and contract decisions

| ID | Proposal and sources | Understanding / risk | Effort and dependencies | Recommendation / destination | Minimum verification | HITL decision |
| --- | --- | --- | --- | --- | --- | --- |
| P24-08 | Choose a separate, explicit public Tegra CUDA distribution identity and trusted-publisher route; do not upload `fathomdb==0.8.24+tegra`. Sources: Slice 0 topology/brief; Slice 3 target CRUD; R24-1/R24-8. | Needs owner choice. **High:** an ambiguous/invalid identity could cause wrong CPU/CUDA resolution or an unrecoverable public project name. | M; public-name and registry/trusted-publisher prerequisite. | **Decision required** before Slice 30. Defer Tegra if not selected. | Resolver-selection proof, registry query, identity/provenance record, and later Jetson installed smoke. | Awaiting initial HITL |
| P24-09 | Decide Windows CUDA SDK surface: Python, npm, or both; define unsupported-route behavior. Sources: Slice 0 brief; Slice 1/3 drafts; R24-2/R24-9. | Needs owner choice. **High:** divergent or undocumented SDK surfaces would be a public contract error. | M; depends on P24-10. | **Decision required** before Slice 40. Exclude Windows CUDA if not selected. | Matrix/loader contract plus artifact and installed smoke for each selected surface. | Awaiting initial HITL |
| P24-10 | Name and approve a remote Windows CUDA executor with GPU/toolchain, trust, artifact-transfer, and retention facts. Sources: Slice 0 executor inventory/brief; R24-2/R24-9. | Needs owner-supplied external fact. **High:** hosted Windows CPU cannot prove CUDA; an untrusted/unknown runner undermines artifacts. | L/external; no local Windows compilation. | **Decision required** before Slice 40. Block Windows CUDA otherwise. | Online selector, host/GPU/toolchain witness, artifact provenance, and real installed Windows smoke. | Awaiting initial HITL |
| P24-11 | Integrate the retained SCALE-02 streamed boundary-tie engine work as Slice 20, using its approved no-rerun decision rule. Sources: Slice 0 benchmark index; Slice 4 alignment; R24-5. | Needs owner release-scope choice. **Medium/high:** the unmerged delta is substantial and may regress correctness if adopted without full review. | L; complete-delta design and targeted tests. | **Decision required:** include Slice 20 or defer the branch. | Retained result/provenance plus RED-to-GREEN targeted correctness and regression proof; no speculative benchmark rerun. | Awaiting initial HITL |
| P24-12 | Keep current main CI as the default and perform Slice 10 only for a concrete target-route gap; preserve `5e2a05e2` fast/heavy ownership. Sources: Slice 0 CI finding; Slice 4; merged-main assignment; R24-7. | Clear. **Low:** unnecessary CI edits create ceremony; missing an actual target gap could leave an unsupported route. | S/M; begins from current main after feature planning is authorized. | **Include as Slice 10 no-change-presumption review.** | Relevant local CI-contract tests and `actionlint`; no manually triggered full hosted CI merely for integration. | Awaiting initial HITL |
| P24-13 | Retrieve and compare the linked Memex Windows client evidence before any WAL product change. Sources: Slice 0 controls; Slice 1/3/4; R24-6/R24-12. | Needs evidence. **Medium:** a claimed defect without client attribution would misdirect product work. | M/external access. | **Include as Slice 50 evidence/disposition work.** | Durable reproduce/not-reproduced/insufficient-evidence record tied to actual job/package/environment. | Awaiting initial HITL |
| P24-14 | Replace atomic-publish wording with retry-safe completion semantics and extend installed-package evidence to every affected target. Sources: Slice 3 CRUD; Slice 4 discrepancy; Slice 5; R24-3/R24-4/R24-10. | Clear. **Medium:** canonical contract must match immutable registries without weakening completion. | M; depends on selected package targets. | **Include conditionally in Slice 60** if target publication proceeds. | Publisher existing/error/missing-target tests; canonical docs/requirements; target-native installed smokes and provenance. | Awaiting initial HITL |
| P24-15 | Preserve existing CPU artifacts, publisher idempotency, and target-native installed smokes as release invariants. Sources: Slices 0, 1, 3–5; R24-3/R24-4/R24-10. | Clear. **High if omitted:** a CUDA route can silently damage existing consumers or leave partial publication unrecoverable. | M; depends on selected Tegra/Windows scope. | **Include in Slice 60** for every selected target route. | Candidate-version package matrix, CPU installs, publisher no-op/fail-closed proofs, and target smokes. | Awaiting initial HITL |

## No-change findings retained outside Slice 7

- The reviewed Rust/Cargo/Action pins stay unchanged: Candle, `sqlite-vec`,
  `rusqlite`, ORT, N-API, and Actions SHA pins require dedicated compatibility
  or security evidence rather than a sweep bump.
- Ruff remains pinned pending an explicit reproducibility review. No broad
  Python, Rust, Action, or npm upgrade is included.
- Slice 3 creates no canonical CI or WAL requirement now; the available
  product contracts already cover the applicable outcomes until real evidence
  establishes a gap.

## First-round HITL decision set

The owner must decide P24-01 through P24-07 for prework scope and P24-08
through P24-15 for feature authorization/allocation. A concise decision may
use the IDs and one of **accept**, **postpone**, **reject**, or (for P24-08 to
P24-11) the required explicit choice. On receipt, Slice 6 will write the
decision record and derive a Slice 7 plan containing only accepted P24-01,
P24-03, P24-04, and/or P24-05 work.

## Decision outcome

The owner approved the register on 2026-08-23. The authoritative dispositions
and their precise feature-scope boundary are recorded in
[Slice 6 HITL decisions](slice-6-hitl-decisions.md). The accepted Slice 7 set
is P24-01, P24-03, P24-04, and P24-05 only.

## Sources

- `dev/plans/0.8.24/prework/slice-{0,1,2,3,4,5}-*.md` and the Slice 0
  benchmark, CI, executor, and publication records.
- `dev/plans/plan-0.8.24.md` feature allocation and merged-main CI assignment.
