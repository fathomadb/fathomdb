---
title: 0.8.29 direct slice-agent execution
status: ACTIVE
target_release: 0.8.29
date: 2026-11-10
---

# Direct slice-agent execution

## Current owner mandate

The owner instructs this run to use neither Steward nor Orchestrator roles.
Use one fresh **GPT-6.1 Sol, high reasoning** owner for each slice and independent
**GPT-6 Astra, medium reasoning** reviewers. This direct mandate supersedes
generic role launchers/seat contracts and earlier generic waiting language.
No such coordinating role or nested agent is launched. The parent dispatches
the requested slice owners and reviewers directly.

## Per-slice sequence

1. Owner rereads its actual scope, plan, requirements/acceptance and design, plus
   release requirements/AC, accepted ADRs and completed predecessor changes.
   Update stale assumptions and missing design/test mappings before code.
2. Independent fresh Astra-medium adversarial review of the updated scope,
   requirements/AC, design, prerequisites and test plan. Findings return to the
   owner; implementation waits for PASS. Reviewer has read-only access.
3. Same slice owner implements only its reviewed scope. Code/behavior changes
   use observed RED before GREEN; stage or commit failing tests before fixes,
   preserve human-intent oracles, then refactor. Proposal/documentation/reserved
   slices use appropriate completeness/scoped checks, without invented tests.
4. Fresh independent Astra-medium review of implementation/diff and actual
   verification. Fix actionable findings and re-review within repository retry
   bounds. No self-review or narration substitutes for a terminal verdict.
5. Verify actual files/diff, test exits and both review results; integrate only
   passing scope, update the maintained plan/state and start the next owner on
   that resulting baseline. Later owners must account for predecessor changes.

Use dependency order rather than launching all26 against the original tree.
Slice8 remains explicitly reserved.0–8 remain proposal-producing prework;9
resolves genuinely open scope choices.10 prepares the approved environment/state.
The current execution mandate authorizes advancing approved work through these
review gates; do not add another generic role-confirmation or planning ceremony.

## Isolation, checks and retained scope

Only one writer per checkout. Planning owners may draft in distinct /tmp output
trees for reviewed parent import. Source owners get fresh isolated worktrees,
checkout-owned tools/artifacts and strict release/dependency preflight after10
activates valid state. Pre-activation proposal research cannot fabricate strict
preflight success. Never borrow a venv or repoint another checkout's bindings.

Use relevant scoped checks while editing, then required final source-phase full
verification once on a stable tree; repeat affected failed checks as needed.
Run Markdown/state checks for documents/metadata; no source qualification claim
follows a proposal review. Retain raw compiler diagnostics and real exits.

Fixed scope: format37/no database migration; .29 fork work is A required repairs
and necessary repair-release versioning; B/C rebase/marker/multiarch work is .30.
Keep the query scaffold and settled compact-default/B2-8 decisions. Preserve
current driver/primary-toolkit constraints. Specific unresolved program/policy,
fault venue, priced-run, optional experiment and publication gates remain; the
direct mandate is not a blanket approval for those actions.

## Current work

Slices0–9 proposal/reservation/checkpoint outputs completed both independent
review gates. Slice10 approved preparation completed at
104dcbcbe92a1eca730d146b99ab17d444df6217 after independent design and fresh
implementation PASS, real per-behavior RED/GREEN and assembled qualification.
Its execution/review records preserve actual failures, resumptions and skips.
JSON is the ladder authority; readiness NOT_READY/publication HELD persist.

Read-only successor designs may be reviewed against a named immutable candidate
while predecessor qualification is pending; that review never opens source
entry. Before implementation, each owner must reconcile the actual integrated
predecessor, update affected plans/designs/requirements/AC and obtain any required
review recheck, then enter a fresh parent-created worktree after strict dependency
preflight. Remaining20 criteria and60 F11 retain their explicit holds;70 does not
wait for whole20/60 closure beyond its declared hard prerequisites.
The current20 test-only policy question does not waive RED/GREEN for defects.
Specific110/120/130/150/publication choices remain explicit. No candidate or
planning review counts as completed source work. This file is not a second ladder.

The parent checkout now resides at /mnt/data/worktrees/fathomdb-0.8.29 on
llm/0.8.29-chat; /mnt/data/worktrees remains the shared directory for sibling
checkouts. The former app-managed path is a symlink to this checkout; custom
path reattachment was rejected by the app, so no successful reattachment is
claimed. Git linkage and original file manifest were verified after relocation.

Slice20 Linux repairs are integrated at657edeb76caaf9c8d2aab433766bec9362e73217
after genuine RED/GREEN and fresh independent implementation PASS.20 remains
open for A3/A5 policy and actual Windows A6; no30 clearance or Block1 exit.
Slice60's implemented format/direct-filter/budget subset is integrated at
5271c1bf022f0ea22f95f7433e226feb2315043e after fresh independent implementation PASS
and assembled214-suite qualification. F11 remains held;60 is not closed. Its
records preserve the original wrapper failure, native skips and actual source
identities. Reviewed successor proposals were imported without source-completion
claims.70 completed at e875f36042f89c1d84756f78135db0e8926c1ed4 after actual
integrated-base rereading, design recheck, seven committed RED/GREEN pairs,
assembled214/214-suite qualification and fresh production Python/TS proofs.
Fresh distinct Astra final review passed; the parent integrated that exact SHA
following strict closed10 landing preflight. Native skips and original failed
checks/targeted resumptions remain explicit.20/60/F11 and Block1 remain open.
JSON remains the sole ladder authority.
