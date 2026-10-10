---
title: FathomDB 0.8.27 Slice 140 — documentation and structural convergence
status: REVIEWED_PLAN
target_release: 0.8.27
planning_baseline: 360bbe63e65aca93cb2200f6c8e27624f4042ea8
---

# Slice 140 — documentation and structural convergence

This plan approves the bounded Slice 140 scope in the [release plan](../../../plan-0.8.27.md).
The [design](design.md) gives the current owners, edit rules, and verification
routes. Planning can proceed beside Slice 117 in an isolated worktree. Product
edits and final candidate receipts must be rebased on its reviewed handoff;
Slice 117's unpublished candidate is not a published Jetson claim. Slice 150
owns final release qualification.

## Reconciliation since the release draft

The release draft began with five large facades and a documentation cleanup.
The review at `360bbe63e` checked the assigned master-plan text, the current
source tree, completed slice records, the Slice 117 handoff, and allocated
follow-ups. The decisions below supersede the draft's stale assumptions.

| Change or allocated item | Decision for Slice 140 |
| --- | --- |
| Slices 40–90 split the engine by semantic owner. The original `lib.rs` 34,184-line figure is historical; it is now 3,905 lines. Slice 90 retains cross-domain test fixtures at root and assigns other hooks to their current modules. | Accept the current owner map. Review the remaining root for test-seam gating and navigation. Do not restart a broad engine decomposition. |
| Slices 100–115 split PyO3 and NAPI, corrected diagnostic transport, and audited warning/dead-code paths. Slice 120 split TypeScript; Slice 130 split Python. Their current largest production files are 1,111, 1,049, 1,035, and 1,551 lines respectively. | Treat four of the five original outliers as resolved by their own slices. Check their interface/docs citations and imports; do not move code solely to reduce line counts. |
| Slice 132 added the dedicated `fathomdb-sdk` Rust application SDK, with a 44-operation parity map beside Python and TypeScript. | Document three peer SDKs and the separate lower-level `fathomdb` facade accurately. Run published-path import/compile examples for all three. Do not add SDK operations or reopen Slice 132's external-provider decision. |
| Slice 135 closed a five-area diagnostic inquiry and a 611-query CPU–CUDA sample at a pinned candidate, host, and protocol. Some raw licensed archives remain local only. | Cite tracked reports and their limits. Do not claim final-candidate equivalence, improvement, or publicly available raw archives. Slice 150 repeats final qualification. |
| Slice 117 is concurrently developing a Tegra Node addon. Its current plan/status remain draft, with delivery-shape and publication conditions. | Use its reviewed final handoff when available. Do not mark Node Jetson GPU support shipped from draft or local qualification evidence. Recheck install docs against the landed candidate before closing Slice 140. |
| Slice 137 is only a proposed Rust consolidation/extension-lockdown approach with four unruled choices. | Reject its work from Slice 140. No crate consolidation, provider-contract withdrawal, or ONNX decision is implied by documentation convergence. |
| Slice 70 carried a proposed dual-runtime ADR, two unreviewed historical memos, a Slice 40 precedence defect, and default-compiled test seams. Slice 80/90 moved many seam owners; the hidden-surface plan requires a successor baseline for an intended gate change. | Accept all four as explicit work. ADR status requires a HITL ruling and code/decision consistency check. Mark the memos historical or review them. Correct the precedence table. Inventory and gate seams at their current owners, record a reviewed hidden diff and a `CHANGELOG.md` Removed entry. |
| The original five-file size target has nine current first-party production files at or above 2,000 lines: engine `lib.rs` above 3,500, seven other engine files and CLI `lib.rs` in the review band. A wider tracked-code scan also finds owned tests/tools and vendored/generated files. | Enumerate all tracked code-file triggers in the register. Review every first-party owned file; identify third-party/generated exceptions with provenance. Record an approved root exception or follow-up and a human owner verdict for each first-party review-band file. No size threshold becomes CI policy. |

## Needs, requirements, and acceptance

`dev/acceptance.md` stays locked; these IDs are release-local.

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-140A | A reader can find the current architecture, owners, and three SDK contracts without following stale authority. | AC27-140A: architecture, design, interfaces, public docs, examples, and source citations name the as-built taxonomy and `fathomdb-sdk` peer surface. Historical documents identify their status. A clean release checkout resolves tracked links and source anchors. |
| R27-140B | Documented public routes work on the installed or compilable surface. | AC27-140B: representative Rust, Python, and TypeScript documented imports and runnable examples compile/run through the respective documented package route. Failures name the exact example, environment, and correction; no source-tree-only import is reported as an installed-package pass. |
| R27-140C | Large code files have an accountable, bounded disposition. | AC27-140C: a source-SHA-bound register enumerates every tracked code-file trigger above 3,500 lines or in the 2,000–3,500-line review band. Every first-party file has an approved exception/follow-up or recorded human owner review verdict, respectively; vendored/generated files have an explicit provenance-bound exception. It covers the original five outliers and any new trigger on the final candidate. A coverage check compares the register with an advisory scan without asserting a maximum size. |
| R27-140D | Test-only Engine access is absent from default production builds without losing required test routes. | AC27-140D: a current-owner inventory records each always-compiled doc-hidden seam, callers, cfg and chosen gate. RED compile/feature tests precede gating. Isolated external-dependency compile-negative controls prove removed public paths absent in default dev and release builds, paired with positive explicit-feature controls; unit and `test-hooks` targets still exercise them. `CHANGELOG.md` has the Removed entry required by AC-050c. The public comparator, hidden/test-inventory comparison, and a reviewed successor hidden baseline explain every intended delta. |
| R27-140E | Carried documentation decisions are truthful. | AC27-140E: the dual-runtime ADR and decision-index row receive a consistent HITL-ruled status; the two `UNREVIEWED` memos are reviewed or marked historical; the 0.8.25 Slice 40 table states that `failed` plus absent physical rows wins over unenrolled-edge corruption; and Slice 135 claims retain their stated limits and tracked citations. |
| R27-140F | The slice is independently reviewed and verified. | AC27-140F: a first design review by `gpt-6.1-sol` high and any subsequent design review by `gpt-6-sol` high have no open P0/P1; material source edits receive `gpt-6-sol` high code review; an independent Terra verification checks the candidate and acceptance matrix. Run focused checks while resolving findings, scoped Markdown/public-doc/release-state gates for document edits, and the full `agent-verify` gate for executable or source edits. Record exact commands, statuses, source SHA, and limits. |

## Execution after design approval

1. **Rebase and inventory.** Bring in Slice 117's reviewed handoff when ready.
   Re-enumerate current owners, documented paths, links, and file sizes at that
   exact source. Record a before-state hidden manifest. Keep the Slice 117
   checkout and its build/package state untouched.
2. **RED.** Add narrow tests that fail for the actual unwanted default-build
   seam exposure and still permit explicit `test-hooks`/unit routes. Add
   example/import or register-coverage checks only for an observed gap.
   Stage the failing tests visibly; leave them unchanged while fixing code.
3. **GREEN and refactor.** Gate only inventoried seams, retarget genuine test
   callers to explicit features, and keep production diagnostics available.
   Update architecture, design, interfaces, examples, citations, the
   precedence table, historical memo labels, and `CHANGELOG.md`. Avoid moving
   unrelated implementation or extending public SDK behavior.
4. **Compare and review.** Run focused Rust/default/test-hooks and import
   checks, the immutable public comparator, and the hidden/test-inventory
   comparator. Review the intended hidden diff, commit a successor baseline,
   and obtain the code review. Resolve findings with focused blast-radius
   checks; rerun a full gate only for a materially changed candidate.
5. **Close.** Run required scoped checks and one full code gate on the final
   executable/source candidate. Obtain independent Terra verification. Write
   `status.md`, update release state and generated views, merge the exact
   reviewed branch into `release/0.8.27` after Slice 117's work is integrated,
   verify from Git, then remove this temporary worktree and branch.

## Boundaries

- No tag, registry publication, Pages publication, or final release claim.
- No size-based blocking test, wholesale source move, SDK feature addition,
  ONNX/provider policy change, or change to locked global acceptance.
- The unruled Slice 117 delivery shape and Slice 132 provider disposition
  remain with their owners. A conditional documentation claim cannot resolve
  either decision.
