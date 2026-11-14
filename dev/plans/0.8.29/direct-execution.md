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
dedicated owner per slice, each in its own task tree or worktree. Three things
stay serial: root release-plan/state/ledger integration by the parent; the single
local heavy window (native builds, full gates, GPU, Windows VM), held by one owner
at a time; and each slice's own design review → RED/GREEN → implementation review.
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

This continuation excludes CI work/execution and registry publication, including
remote workflow dispatch and publishing tag pushes. Required local verification
and selected permitted hardware/installed-consumer proof remain in scope. Keep
CI/publication and post-publication smoke explicitly outstanding at160; local
completion is distinct from published-release completion. Specific unresolved
policy/artifact/fault/venue/spend choices are not answered by this mandate.

Excluding CI discharges no evidence. A selected mandatory160
installed-consumer or platform row (Q14) that a permitted local or owned venue
can qualify must be qualified there, and stays open until it is; the same
holds for any locally runnable part of the Q18 release dry-run. Finished
bounded local tasks are recorded as such and are not local completion. The
local mandate is complete only when the sole unmet rows are those that need
excluded CI execution or publication; each such row is named outstanding,
neither waived nor passed.

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
  review at coherent implementation milestones.
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

Dispatch by true dependency, not ladder number.
Slice8 remains explicitly reserved.0–8 remain proposal-producing prework;9
resolves genuinely open scope choices.10 prepares the approved environment/state.
The current execution mandate authorizes advancing approved work through these
review gates; do not add another generic role-confirmation or planning ceremony.

## Isolation, checks and retained scope

Only one writer per checkout. Planning owners may draft in distinct assigned
`/mnt/data/` task trees for reviewed parent import. Source owners get fresh isolated worktrees,
checkout-owned tools/artifacts and strict release/dependency preflight after10
activates valid state. Pre-activation proposal research cannot fabricate strict
preflight success. Never borrow a venv or repoint another checkout's bindings.

Use relevant scoped checks while editing, then required final source-phase full
verification once on a stable tree; repeat affected failed checks as needed.
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
open for actual Windows A6; approved A3/A5 proofs and RAW capacity preparation
are qualified at f803a964 after fresh independent final review. Seq-369/370 are
the current owner rulings. The accepted30 P subset has separate recorded entry/limits; no
whole20/30 clearance or Block1 exit follows.
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

## Continuation checkpoint

Reviewed Slice20 continuation documentation391ae66aa is integrated after fresh
independent final Astra-medium PASS. It reconciles actual60/70, preserves all
mandatory criteria and records the exact reviewed Windows operations proposal.
No product source changed in that documentation checkpoint. Current owner
rulings are ledger seq-369/370. Their Linux proofs and capacity preparation
subsequently passed fresh final review and were integrated at f803a964; native
Windows A6 remains unqualified.

Slice30 documentationbea7f05cd and its exact P-only dependency amendment passed
independent Astra-medium design review. State/master now permit initial direct-
text Linux P entry from closed10 plus qualified integrated20/current70 after
actual ancestry/preflight, own source/native setup and parent GO. G/D need later
concrete review; the20 proof condition for M is now satisfied, while its selected
G-store/tooling prerequisites, F frozen TC5, whole30/40/50 and pre90 Block1 remain
held. The eligible30 P subset subsequently completed its own recorded
qualification; whole30 remains open. Keep each dedicated owner for continuation.

## Owner decisions recorded 2026-11-13

The two actual Slice20 replies are recorded in the append-only ledger at seq-369
and seq-370, and their exact scope is referenced in release-state decisions.ruled.
A3/A5 execution now follows the approved verification-only route. Capacity work
requires a reviewed derivative that stops after creation/attachment and read-only
identity checks; the prior full guest-sequence packet cannot run unchanged under
the narrower ruling. No renewed generic permission or broader authority is inferred.

The accepted20 continuation at f803a964 has assembled216/216-suite qualification
and actual source/nonce/native child proof, preserving its first lint failure and
strict resumed phases. The64GiB disk is attached RAW only; no formatting or actual
Windows native A6 evidence is inferred. The G-first/D proposal and actual-integrated20 fit have independent design
PASS and are imported; fresh isolated G entry and parent source GO remain.

## Parallel lanes

JSON holds slice status; this lists only what each isolated lane still owes.
No lane below is integrated at root or qualified until its own gate and review
pass and the parent integrates it.

- **20:** remaining repairs proceed by per-behavior RED/GREEN in the owner's
  worktree under an exclusive heavy window. Original Windows test bodies stay
  frozen. Actual Windows A6 remains mandatory; it needs the VM's existing
  C:8/5GiB and host-memory start floors met and the disk-format answer.
- **30 G/M:** the closed-copy tooling and M source candidates each have
  implementation-review ACCEPT but no full-gate PASS. G campaign stands at11
  qualified and73 required cap-refused of84; M46 cells and D24 remain;
  successor caps are inactive and no runtime GO exists. The owner may propose a
  bounded cost-probe admission; it takes effect only through review and parent
  disposition, and the accepted inventory is unchanged.
- **100:** qualified subset is integrated; Q1 interruption stays UNQUALIFIED
  pending the additional matched-pair question.
- **110, 120:** owners are drafting requirements/design/test map in isolated
  task trees, source read-only. Design review, the fault-scope and
  artifact-selection answers and all source/runtime work remain.
- **40, 50, 80, 90, 130–160:** reviewed bounded designs; source held on their
  JSON conditions.
