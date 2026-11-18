---
title: Slice 10 approved-scope preparation
status: ACTIVE
target_release: 0.8.30
slice: 10
date: 2026-11-18
---

# Slice 10 approved-scope preparation

Prepare the owned release environment, apply selected dependency/security and
runtime changes, correct targeted current documentation, and deliver the local
contract allocations needed by20–80. The scope ruling is seq-399, recorded from
“I agree with all of the proposals”; see the [proposal register](../../prework/slice-9.md).
This concrete plan passed [Astra-medium review](planning-review.md) after FIX-1
and is approved for implementation at seq-400.
No implementation or platform/support claim is established by this document.

## Entry reconciliation and slice prep

1. Verify branch `release/0.8.30` and its owned worktree. Reconcile current main
   and changes since prework base `18427b7a8`, including .29 completion at
   `b86b19847`. Preserve the separately governed Inspector branch. Retain .29's
   completed Tegra/public-consumer evidence; do not redo its delivery.
2. Read the [master](../../plan.md), [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md),
   [environment](../../prework/slice-0.md), [library inventory](../../prework/slice-1.md)
   and prework2–7. Enumerate new fixes, candidate versions, lock/feature changes,
   already-applied PRs and allocated drafts. Adopt, adjust or reject each stale
   draft with a brief reason. Recheck official release/advisory metadata at
   implementation time; listed candidate versions below are dated leads.
3. Retain owned Python3.12 and Rust1.95 unless an approved dependency requires
   a justified compatible adjustment. Set up only this checkout's `.venv` using
   the current owned noneditable build path. Do not borrow another checkout's
   venv or use editable native installs. Select an exact project npm version
   compatible with the chosen Node/runtime graph; distinguish it from the
   separately pinned trusted-publishing CLI. Keep routine Dependabot version
   PRs paused and immutable Actions/source/model/native pins intact.
4. Before changes, inventory the exact affected manifests, lockfiles, workflows,
   setup/version selectors, native loader/build assumptions, docs and existing
   behavior tests. Record unresolved private security-alert access honestly.
   Preflight a fresh implementer worktree before commissioning it. Root owns
   integration and release-state edits; every concurrent writer has isolation.

## Local needs, requirements and acceptance

These labels are local to this slice; they do not modify canonical AC numbering.

| Local requirement | Need and intended result | Acceptance evidence |
| --- | --- | --- |
| PREP-ENV | Contributors and CI select an owned, coherent environment. | Candidate development/test selectors, project npm and Python ownership agree; production publisher selectors remain held for20. An unrelated/missing venv fails the existing ownership/preflight controls. Record actual setup and tool versions; no borrowed native import. |
| PREP-SEC | Fix the three selected affected resolutions without unrelated graph churn. | Separately reviewable Markdown, TLS-root and Tokio deltas resolve outside the recorded affected ranges. Record selected target/features and advisory reachability; pass the affected controls below. Preserve immutable model/fork pins and explain additional transitive changes. |
| PREP-NODE | Qualify a coordinated Node26LTS migration without unsupported package claims. | Slice10 qualifies the exact candidate/npm with a packed native SDK on the owned host, existing build/loader/error tests and a disposable real-store persistence/reopen probe. This is candidate evidence only. Slice20 implements the missing shared installed-consumer checks;70 uses them on the selected matrix before public engines/support metadata is promoted. Unavailable targets remain open, with no new support claim. |
| PREP-TRIAGE | Exporter/rendering advisories receive bounded, truthful dispositions. | Each identified advisory has affected API/graph, owner and fix or documented hold. A selected narrow exporter change preserves reproducible output/model identity; any required model-byte change returns for a separate decision. Rendering changes use a supported graph and real rendering/browser-startup proof. Already-fixed DOMPurify is not upgraded a second time merely to satisfy an old PR. |
| PREP-DOC | Current user/developer/contributor guidance matches implemented behavior. | Targeted statements agree with source and accepted contracts; links/navigation resolve. Public edits pass docs lint and strict MkDocs. History and unique evidence remain available. No unsupported runtime/model/architecture claim is introduced. |
| PREP-ALLOC | Approved drafts have a receiving owner and falsifiable proof. | A concise allocation record maps prework3–7 and P9-08–12 into20–80 with needs/requirements/acceptance/design/test obligations, dependencies and limits. Each receiving plan reconciles selected items; proposal-only analysis cannot close R30-TEST. |
| PREP-STATE | The approved release has one truthful execution record. | On final approval, create minimal .30 JSON state/board with this master pointer and seq-399, truthful prework/proposal statuses and next_slice10. Preserve closed .29 state. Generated views and current-release selection agree; no feature or publication status is fabricated. |

## Design, architecture fit and difficult seams

Use [architecture v2](../../../../design/fathomdb-data-plane-architecture-v2.md),
[ADR index](../../../../adr/ADR-0.6.0-decision-index.md),
[bindings design](../../../../design/bindings.md) and
[release design](../../../../design/release.md). The [prework source review](../../prework/slice-4.md)
found existing storage, transaction, runtime and delivery owners sufficient.
This slice introduces no schema, writer lane, retrieval algorithm or model owner.

- **Security units:** take #257 as a candidate, not an automatic merge:
  `markdownlint-cli2`0.23.3/`markdown-it`15.0.1 → qualified0.24.0/15.0.3.
  Preserve the CommonMark AST guard; exercise its existing controls and actual
  lint/format path without accepting semantic rewrites. Narrowly advance
  `webpki-roots`1.0.7 via its0.26.11 wrapper to a patched compatible1.x (prework
  candidate1.0.12, recorded fix≥1.0.10); no ureq3 migration. Qualify TLS trust,
  refusal/retry/digest controls on both default model-loader graphs. Advance
  Tokio1.52.3 to a patched compatible release (candidate1.54.1), avoiding the
  recorded1.54.0 interval regression. Inspect resolved features/targets before
  selecting MCP stdio/admission, Node async/lifecycle and HTTP controls. Retain
  exact rmcp/httpmock pins unless a consumed compatibility fix is necessary.
- **Node migration:** use official Node release/security/support metadata and
  npm/native-addon release documentation to select the exact26LTS candidate.
  Inventory `.nvmrc`, package engines/packageManager, setup/workflow pins and
  installer/runtime checks. Coordinate development/test selectors against the
  candidate graph. In10 build and pack the native SDK, install it in a fresh
  local directory with the exact candidate/npm, run existing build/loader/error
  controls, and run a disposable real-database probe: write/read an expected
  body, check a missing record, close/exit, then reopen/read in another process.
  This local probe is not the shared registry-smoke implementation assigned
  to20. Leave normal production publishing Node/npm selectors unchanged in10:
  its `NPM_BIN=npm` currently selects bundled npm, so a blanket Node pin update
  would silently alter trusted publishing. Slice20 owns that coherent change
  and its pinned OIDC qualification. Hold public package engines/support claims
  until70 consumes20's installed-consumer checks on the selected matrix after
  final native/fork inputs settle. Missing targets remain explicitly open;
  local10 closure never promotes them by inference. Record each proof's source,
  artifact and runtime. Slice80 owns actual public-registry proof.
- **Exporter/rendering:** inspect the APIs and supported dependency constraints
  behind #256/259/260/261/263, upstream advisories and the existing pinned
  exporter/renderer instructions. Prefer the smallest compatible security
  resolution. Do not force Zod4 into a Mermaid graph declaring Zod3 support.
  A narrowly selected exporter fix may run a disposable local verification
  export with the exact pinned model/tokenizer inputs, comparing the resulting
  graph/tokenizer digests to the accepted assets. If that reproduction cannot
  run or bytes differ, hold adoption of the exporter change and retain the
  observed limitation; changed canonical model assets need a separate decision.
  Do not replace/publish canonical assets or perform a broad exporter re-export
  campaign or Transformers/Torch/browser major migration. If a reachable
  advisory cannot be fixed within this scope, retain
  its exact limitation/owner and seek a concrete scope decision instead of
  silently leaving it unresolved or claiming it safe from version alone.
- **Docs/architecture:** correct the selected structured-hit body-dedup and
  mandatory provenance wording against actual record-version identity and
  nullable source fields; do not redefine public-reference semantics. Repair
  owned setup, long-test/ptrace guidance, current architecture/status pointers
  and ADR-index/dev-entry applicability as supported by prework6. Preserve
  accepted ADR decisions and canonical ACs; detailed API/model-release changes
  remain with their owning feature design. Main has already repaired .29
  publication metadata; avoid repeating it.
- **Allocations:** carry the selected local drafts into20(CI/test harness),
  30(correctness/reference/property),40(latency),50(lifecycle/resource/fault),
  60(fork),70(integrated accounting) and80(delivery). Preserve two bounded
  hunts, fixed-state exactness, real-store independent oracles, positive
  reader/writer progress, targeted test-cost evidence and all scope limits.
  No20 CI behavioral repair,30/50 hunt or40 treatment is implemented in10.

Any architecture/interface design additions receive direct Astra-medium design
review before implementation. Primary-source research resolves the named
compatibility challenges; it is not a new general research campaign.

## Execution order, reviews and verification

1. After final HITL approval, activate minimal state once, then commission a
   direct Sol6.2-high implementer in a preflighted owned worktree. Reconcile the
   plan and local contract/design/test drafts, including newly discovered
   prerequisites. Hold only affected work when a prerequisite is missing.
2. Use separate coherent security changes and a coordinated Node migration.
   For changed behavior, including executable setup/version guards, show a
   genuine observed TDD RED first, then GREEN/refactor; do not invent RED for
   mechanical lock/version edits or weaken accepted tests. Test actual behavior
   and compatibility, not text copies of the implementation. For dependency
   changes retain existing meaningful regressions and add only a missing
   discriminator justified by the actual risk.
3. Run scoped lint/type/affected tests during each unit. Markdown uses
   `scripts/agent-lint-md.sh`; only `npm run format:md` may autofix Markdown.
   Public docs also use `scripts/agent-lint-docs.sh` and strict MkDocs. Use
   actionlint for workflow deltas, never YAML parsing as validation. Run the
   actual native/MCP/model-loader controls selected in design, recording
   commands, relevant case counts, exits, skips and unavailable platforms.
4. Obtain direct Sol6.2-high code review and direct Terra independent verification
   of affected behavior and environment/native identity. Keep a native candidate
   clean during verification; stage release-record edits elsewhere. One full
   `scripts/agent-verify.sh` is required for the final coherent source phase;
   narrow corrections rerun failed, affected and previously unrun scope. A
   ptrace-denied strict gate needs a ptrace-capable executor, not a weakened
   command. Full later long/security/platform/artifact gates remain70/80.
5. Record selected versions, exact source/lock/native identities, advisory/PR
   dispositions, actual tests, docs and receiving allocations in one slice
   status record. Unavailable required qualification remains explicitly open
   in70 and does not become a support claim. Integrate reviewed work to the
   release branch with normal hooks; update state/generated views. Clean only
   owned inactive scratch/worktrees after integration and evidence retention.

Existing compute only: no automatic VM startup, GPU reservation/reset, runner or
protected-route mutation, driver/primary-toolkit change, paid/model campaign or
publication. A necessary external action gets its concrete venue/resource scope
resolved at the owning gate. Do not expand to general dependency refresh, broad
cruft deletion/archive moves, structural refactoring or Inspector integration.

## Planning review and final decision

The Slice9/Slice10 planning review uses a direct Astra-medium reviewer and up
to four FIX-n cycles, with findings and dispositions saved alongside this plan.
The parent checks fixes before re-review. After PASS, commit the planning
artifacts and present the concrete preparation plan to HITL for the final
implementation decision. The scope agreement at seq-399 is settled; do not
ask the owner to decide P9-01–14 again.

Activation: seq-400 closes the final planning decision and commissions10.
The implementation status is owned by release-state-0.8.30.json.
