---
title: FathomDB 0.8.27 Slice 140 — documentation and structural convergence
status: REVIEWED_PLAN
target_release: 0.8.27
planning_baseline: 360bbe63e65aca93cb2200f6c8e27624f4042ea8
---

# Slice 140 — documentation and structural convergence

This plan approves the bounded Slice 140 scope in the [release plan](../../../plan-0.8.27.md).
The [design](design.md) gives the current owners, edit rules, and verification
routes. Planning was approved beside Slice 117 in an isolated worktree. Slice 117
has since closed at its qualified, unpublished candidate on the release
branch. Product edits and final candidate receipts must use that integrated
source; an unpublished candidate is not a published Jetson claim. Slice 150
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
| Slice 117 closed its Tegra Node delivery route at a qualified, unpublished candidate; Slice 160 owns publication and post-publication proof. | Use its exact-source closeout and installed-artifact receipt. Do not mark Jetson Node GPU support shipped from local qualification evidence. Recheck install docs against the integrated candidate before closing Slice 140. |
| Slice 137 is only a proposed Rust consolidation/extension-lockdown approach with four unruled choices. | Reject its work from Slice 140. No crate consolidation, provider-contract withdrawal, or ONNX decision is implied by documentation convergence. |
| Slice 70 carried a proposed dual-runtime ADR, two unreviewed historical memos, a Slice 40 precedence defect, and default-compiled test seams. Slice 80/90 moved many seam owners; the hidden-surface plan requires a successor baseline for an intended gate change. | Accept all four as explicit work. ADR status requires a HITL ruling and code/decision consistency check. Mark the memos historical or review them. Correct the precedence table. Inventory and gate seams at their current owners, record a reviewed hidden diff and a `CHANGELOG.md` Removed entry. |
| The original five-file size target has nine current first-party production files at or above 2,000 lines: engine `lib.rs` above 3,500, seven other engine files and CLI `lib.rs` in the review band. A wider tracked-code scan also finds owned tests/tools and vendored/generated files. | Enumerate all tracked code-file triggers in the register. Record a concise disposition for each first-party file and a provenance explanation for third-party/generated files. The register requires no human review or approval for slice or release closure. No size threshold becomes CI policy. |

## Needs, requirements, and acceptance

`dev/acceptance.md` stays locked; these IDs are release-local.

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-140A | A reader can find the current architecture, owners, and three SDK contracts without following stale authority. | AC27-140A: architecture, design, interfaces, public docs, examples, and source citations name the as-built taxonomy and `fathomdb-sdk` peer surface. Historical documents identify their status. A clean release checkout resolves tracked links and source anchors. |
| R27-140B | Documented public routes work on the installed or compilable surface. | AC27-140B: representative Rust, Python, and TypeScript documented imports and runnable examples compile/run through the respective documented package route. Failures name the exact example, environment, and correction; no source-tree-only import is reported as an installed-package pass. |
| R27-140C | Large code files have an accountable, bounded disposition without a size or sign-off gate. | AC27-140C: a source-SHA-bound register enumerates every tracked code-file trigger above 3,500 lines or in the 2,000–3,500-line review band, records a disposition for each first-party file and provenance for vendored/generated files, and explains the original five outliers. Re-scan the final candidate and update the register for new triggers. Check completeness against the advisory scan; neither file size nor human approval of the register blocks release. |
| R27-140D | Test-only Engine access is absent from default production builds without losing required test routes. | AC27-140D: a current-owner inventory records each always-compiled doc-hidden seam, callers, cfg and chosen gate. RED compile/feature tests precede gating. Isolated external-dependency compile-negative controls prove removed public paths absent in default dev and release builds, paired with positive explicit-feature controls; unit and `test-hooks` targets still exercise them. `CHANGELOG.md` has the Removed entry required by AC-050c. The public comparator, hidden/test-inventory comparison, and a reviewed successor hidden baseline explain every intended delta. |
| R27-140E | Carried documentation decisions are truthful. | AC27-140E: the dual-runtime ADR and decision-index row receive a consistent HITL-ruled status; the two `UNREVIEWED` memos are reviewed or marked historical; the 0.8.25 Slice 40 table states that `failed` plus absent physical rows wins over unenrolled-edge corruption; and Slice 135 claims retain their stated limits and tracked citations. |
| R27-140F | The slice is independently reviewed and verified. | AC27-140F: a first design review by `gpt-6.1-sol` high and any subsequent design review by `gpt-6-sol` high have no open P0/P1; material source edits receive `gpt-6-sol` high code review; an independent Terra verification checks the candidate and acceptance matrix. Run focused checks while resolving findings, scoped Markdown/public-doc/release-state gates for document edits, and the full `agent-verify` gate for executable or source edits. After all corrections, obtain the user-required three independent subagent reviews. Record exact commands, statuses, source SHA, and limits. |
| R27-140G | Library users and contributors can navigate source-derived documentation on Pages. | AC27-140G: a deterministic stdlib Python generator supports `--write` and non-mutating `--check`; tracked generated pages enumerate all 44 governed operations with exact Python/TypeScript/Rust spellings and locators, and current Rust workspace members, Python/TypeScript package names and manifest paths, and typed developer entry points. The operation map identifies itself as an index and links detailed manual API references. A contributor landing page explains TDD, scoped/full gates, worktrees, and release state; README and MkDocs navigation link these pages. The shared Pages builder rejects stale generated output before MkDocs, so both complete-site deployers include it; docs-pages path filters cover the generator and all input families. Both whole-site Pages routes freshly fetch current `origin/main` with complete ancestry and require `git merge-base --is-ancestor origin/main "$GITHUB_SHA"` before building/uploading and immediately before deployment; fail closed on stale or unavailable source history. RED tests prove source changes change output and stale output fails, and focused pipeline/navigation checks prove both delivery routes. No package version or publication claim is generated from pre-bump manifests. |
| R27-140H | Publication-facing docs have an explicit release target without claiming publication. | AC27-140H: the active tracked release state may declare `public_docs_target_release` equal to its own release; otherwise public-doc truth falls back to the newest valid published receipt. The checker uses canonical active-release resolution, validates marker identity, selected-target schema 34 and publication receipts separately, requires target version/install guidance, rejects explicit false publication assertions for a pending target, and retains platform/schema/false-historical-publication controls. Workspace ownership is checked against the generated contributor map; optional README count claims must still be truthful. RED fixtures cover pending 0.8.27 target, historical published 0.8.26 fallback, malformed/foreign/untracked targets, missing target guidance and false publication claims. Public API examples document actual default-embedder and search options; compatibility links the digest-verified Tegra install procedure. |

## Execution after design approval

1. **Rebase and inventory.** Bring in the closed Slice 117 handoff.
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

## Full-gate follow-up plan

The first integrated full gate found stale cfg/blob owner assertions and
whole-target feature gating that disabled required default C1 proofs. Update
only assertions for the intentional fixture gate, retaining their narrowing,
widening, and behavioral mutants. Restore the two pinned integration targets
to the default route; use production caller/no-embedder opens and governed
vector declarations. Move the structural-writer proof to its default unit
owner because its writer is strictly test-only. Update only its C1 evidence
location, preserving the clause and contract pin. Re-capture hidden/test
inventory after these changes and explicitly review the renamed unit proof
and item-gated fixture distribution.

The user requested an excellent 0.8.27 publication-facing README/docs set.
Extend document convergence to installation, compatibility, embedder,
operations, references, package READMEs, release notes, and site navigation.
Public pages describe final install routes without transient candidate
notices; internal verification records unavailable registry/Pages routes
until Slice 160. Check runnable API examples from source/installed candidates
and the strict retained-manifest digest script. Run focused
pinned/owner/CLI/scanner tests and scoped document checks. Obtain a subsequent
`gpt-6-sol` high design review and source review before final full-gate/Terra
verification; keep source versions and retention unchanged.

## Boundaries

- No tag, registry publication, Pages publication, or final release claim.
- No size-based blocking test, wholesale source move, SDK feature addition,
  ONNX/provider policy change, or change to locked global acceptance.
- The Slice 117 delivery shape is ruled and its unpublished candidate is
  closed; do not turn that evidence into a publication claim. The unruled
  Slice 132 provider disposition remains with its owner. Documentation
  convergence cannot resolve that decision.

## Generated-documentation amendment

The user added source-derived library-user and contributor documentation,
published through CI to Pages. Current manual SDK pages remain the detailed
contract. The operation parity manifest already records 44 governed
operations; the workspace and Python/TypeScript package manifests and typed developer
scripts supply a
bounded contributor index. Existing Pages deployments share
`scripts/release/build-pages-site.sh` and replace the whole site, so the
generation freshness gate belongs there.

After a subsequent `gpt-6-sol` high design review approves this amendment:

1. **RED:** commit narrow generator tests for the three SDK spellings/locators,
   all operation rows, deterministic output, source-change propagation, stale
   `--check` failure without mutation, workspace/member, Python/TypeScript package-name and script-input
   changes, Pages-builder/navigation integration, and publication-source
   currency fixtures (current/equal/ahead pass, behind/diverged/missing
   history fail).
2. **GREEN:** implement a stdlib generator, tracked operation/contributor
   maps, the short contributor landing, README/nav links, shared-builder
   `--check`, complete docs-pages trigger coverage, and fresh-main ancestry
   checks in both complete-site publication routes. Keep output version
   neutral; public manual pages target 0.8.27, while internal status records
   that registries/Pages have not yet published it. Check remaining language
   preference claims against actual three-SDK evidence.
3. Run focused generator/Pages tests, scoped Markdown/public-doc checks and
   workflow validation. Executable script changes require the integrated
   full `agent-verify` gate; the parent owns this gate and final package
   examples. Refresh hidden/test inventory and the large-file register on
   the complete source candidate. Obtain code review, Terra verification
   and the three final independent reviews before closing the slice.

Generated pages are navigation indexes, not fabricated API signatures or a
replacement for maintained contracts. The large-file register stays advisory
and requires no human signoff.

## Integrated-gate documentation-target correction

The integrated gate at `1a462d036` exposed a checker that requires every
landing page to call 0.8.26 current/published, requires an obsolete README
count bullet and requires `npm@next`. Update that contract to the explicit
0.8.27 publication-facing intent while preserving the historical publication
receipt. Design review precedes code. Commit RED fixtures, then checker/state
and focused documentation fixes; run public-doc truth/mutation tests,
canonical release/current-view checks, scoped Markdown/public docs and
workflow-safe generated freshness. The parent reruns the full gate and final
reviews. Historical 0.8.26 notes remain history; no publication state,
registry version, retained digest or human large-file gate changes.
