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

## Current continuation scope

The owner requests remaining-slice completion by parallel isolated owners, one
dedicated owner per slice, each in its own task tree or worktree. Root
release-plan/state integration stays single-writer. D/M/S50 scientific
execution stays serial under its existing shared-ledger custody; physical GPU
work takes the specific device lease it consumes. Independent CPU source/build/
test, metadata and Windows work may overlap where actual memory/disk/actor
limits permit. CPU-only metadata must not retain a GPU lease. Each substantive
slice still follows design review → RED/GREEN → implementation review.
Each owner reconciles actual integrated predecessor work and updates its scope,
plan, requirements/AC, design and product architecture as needed before the
independent Astra-medium adversarial design review. After PASS, use real RED/GREEN
and a fresh Astra-medium implementation review. Preserve existing completed exits
unless affected evidence requires a scoped recheck.

A slice waits only for what it consumes: a named reviewed artifact, contract,
ruling, measurement or input. Whole-slice closure of a predecessor is required
only where the JSON ladder or a reviewed plan says so. Scope, research, design
and design review never wait on another slice's closure or on full20. Narrowing
an existing reviewed source-entry gate is itself a design change: it needs Astra
design review and parent integration, and the old gate holds until then.

Use `/mnt/data/` for parent and slice worktrees and all owned task scratch, build
targets, caches, temp files and qualification output. Set `TMPDIR` and
`AGENT_LOG_DIR` to the assigned task root; installed immutable tools are read-only
inputs. No cleanup or relocation of unrelated resources is implied.

The current owner objective is complete release preparation except registry
publication: finish required local qualification, **review CI scripts for
correctness**, then run the appropriate CI. CI review/execution does not precede
local passes. Publishing tags, registry uploads and post-publication smoke stay
excluded. Earlier local-only/CI-excluded wording is superseded by that objective.
Specific unresolved policy/artifact/fault/venue/spend choices remain explicit.

Every selected locally runnable Q14 installed-consumer/platform row and Q18
rehearsal part remains required. Local-phase completion does not imply CI or
publication success; finish the subsequent reviewed CI phase before declaring
the owner's complete-except-publication objective achieved. Retain actual skipped,
failed and venue-only results without calling them passed.

## Work before records

Per slice, the maintained record is: scope, requirements/AC, design, test map,
the committed failing tests and fixes, raw results and real exits, and the two
review verdicts. Update these once, with the substantive work.

- Do not add supplemental receipts, audit packets, review recaps, hash
  inventories, execution diaries or status appendices, and do not add
  bookkeeping-only commits or documentation phases beyond the work. Explicitly
  requested plan or contract work, such as a plan correction or the required
  new110/120 designs, is real work, not bookkeeping.
- Do not replay passing tests, rehash artifacts or reconstruct old receipts for
  bookkeeping. A reviewer reads the diff and the recorded results; a rerun needs
  a concrete doubt about a specific result.
- Do not add approval stops for settled scope or per-helper reviews. Batch
  review at coherent implementation milestones. Owner2026-11-15 explicitly
  reinforces this: implement corrections and run the meaningful work batch,
  then review the combined result; do not turn each helper compile or fixed
  assertion into a fresh model-review checkpoint. Mechanical identity and
  actual actor/deadline/resource/oracle checks remain part of execution.
- Keep: raw scientific measurements with their inputs and caps, source/native
  binding checks, resource limits and owned cleanup, per-behavior RED/GREEN,
  both independent reviews and real platform proof. Existing append-only
  ledgers and historical records stay as they are.

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
   passing scope and update the maintained plan/state. Owners already running
   reread integrated predecessor changes at source entry and again at
   integration; they do not restart for unrelated landings.

Dispatch by true dependency, not ladder number. Completed prework/preparation
is not repeated;8 remains reserved. Resolve only genuinely new item decisions.
Do not add a generic role-confirmation, new packet or parent approval between
reviewed work units. Parent technical GO still controls scarce resource/fault
execution and integration; it does not serialize independent source preparation.
Within the authorized local measurement scope, reviewed current cost/resource
fit and working guards permit the parent to select the technical profile. This
is not a new HITL cap-approval question by inference. A higher spend, new venue,
expanded fault envelope or explicitly one-use action retains its actual owner
gate. Unknown future D/M costs are disclosed, not turned into a G-only forecast
prerequisite; retain the reviewed shared allocation and stop/refusal rules.

## Isolation, checks and retained scope

Only one writer per checkout. Planning owners may draft in distinct assigned
`/mnt/data/` task trees for reviewed parent import. Source owners get fresh isolated worktrees,
checkout-owned tools/artifacts and strict release/dependency preflight after10
activates valid state. Pre-activation proposal research cannot fabricate strict
preflight success. Never borrow a venv or repoint another checkout's bindings.

Use relevant scoped checks while editing, then required final source-phase full
verification once on a stable tree; repeat affected failed checks as needed.
A coherent phase includes related helpers and their tests: do not declare a new
phase or full gate for each helper, review correction or maintained-plan update.
Reuse valid unchanged suite results and fork qualification; product adoption adds
its own affected integration/consumer proof rather than replaying fork suites.
A full gate interrupted for a reason outside the source is finished by running
its remaining registrations on the unchanged source, not replayed from the
start; it is not a PASS until those complete.
Run Markdown/state checks for documents/metadata; no source qualification claim
follows a proposal review. Retain raw compiler diagnostics and real exits.

Fixed scope: format37/no database migration; .29 fork work is A required repairs
and necessary repair-release versioning; B/C rebase/marker/multiarch work is .30.
Keep the query scaffold and settled compact-default/B2-8 decisions. Preserve
current driver/primary-toolkit constraints. Specific unresolved program/policy,
fault venue, priced-run, optional experiment and publication gates remain; the
direct mandate is not a blanket approval for those actions.

## Delivery and current state

[Release state](../release-state-0.8.29.json) owns current exits, decisions and
remaining evidence. The [release plan](../plan-0.8.29.md#remaining-delivery-path)
names the parallel deliverables. Historical source/results/review logs retain
their actual subjects and limits; do not copy them into another status narrative.

Completed0–10 preparation is an input, not a recurring phase. Source entry and
integration require the actual affected-predecessor reread and applicable strict
preflight. A documentation-only landing cannot invalidate an unchanged source,
native artifact, passing test or design verdict. Recheck a design only when the
scope, contract, consumed interface or safety/resource assumption changed; review
the coherent delta with the existing design, without replaying settled reviews.
