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

Slices0–8 proposal outputs/reservation completed with independent Astra-medium plan and
fresh final-output PASS; their local review logs retain evidence and tooling
limits. Slice9 checkpoint also completed both review gates, retaining specific
unanswered feature choices. Slice10 updates approved preparation design; Slice20
does read-only queue/test research and must reconcile completed10 before its
own design review. Successor owners must read these completed outputs. Valid release-state/
board ownership begins in10 as planned. This method file is not a second ladder.
