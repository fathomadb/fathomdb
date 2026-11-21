---
title: Inspector 0.2.0 Slice 0 — environment and project infrastructure proposals
date: 2026-11-19
status: DRAFT
---

# Slice 0 — environment and project infrastructure proposals

## Goal and authority

Create and enter the release checkout under the [owner decision](checkout-decision.md),
then produce a source-grounded environment inventory,
and small actionable proposals that let Slice 9 decide infrastructure work for conditional
Slice 10. Apart from the ruled checkout action, this is inspection and proposals only. No installation,
configuration change, source implementation or runtime qualification is commissioned here.
The 2026-11-19 ruling selects future Slice 0 checkout creation and entry; recording it executes no slice now.

Use [adopted scope](../scope.md), the [master plan](../plan.md),
[workflow](../slice-workflow.md) and [local design](design.md). The inspected source was
`llm/0.8.29-inspector-additional` at `609dd85c41fb7980e05faa1132a248d376d8ec89`,
clean when inspected, carrying Engine base `6edc1155f`, workspace packages 0.8.28 and
current format 37. Inspector remains unpublished package 0.1.0. These are provenance facts,
not qualification or approval to bump a public package.

## Inputs and checked baseline

- Read repository `AGENTS.md`, the external memory index and actual applicable entries:
  `dont-be-distracted-by-ceremony.md`, `verify-design-against-code-not-just-architecture.md`,
  `tdd-red-must-fail-per-finding.md`, `keep-measurement-tools-in-repo.md`,
  `agent-worktree-stale-base-trap.md`, and `windchill3-never-change-driver-or-primary-cuda.md`.
- [HLD](../../../design/high-level-design.md), [requirements](../../../requirements.md),
  [acceptance](../../../acceptance.md), [needs](../../../needs.md), and
  [port evidence](../../0.1/compatibility-0.8.29-port.md) are evidence inputs, not proof of completion.
- [`Cargo.toml`](../../../../../Cargo.toml) includes `src/inspector` in the workspace;
  [`rust-toolchain.toml`](../../../../../rust-toolchain.toml) pins 1.95.0 with clippy/rustfmt.
  [`Inspector manifest`](../../../../../src/inspector/Cargo.toml) has empty default features,
  disables Engine defaults and exposes only opt-in `fixture-control` for fixture tooling.
- [`agent-verify.sh`](../../../../../scripts/agent-verify.sh) requires checkout-owned
  `.venv/bin/python`, pip and developer tools. This source checkout has no executable
  `.venv/bin/python`; the historical port's full gate stopped there. No full green carries forward.
- [`bootstrap.sh`](../../../../../scripts/bootstrap.sh) and
  [`agent-python-env.sh`](../../../../../scripts/lib/agent-python-env.sh) select Python >=3.11
  and prepare local tools. Inspect their exact actions before proposing invocation.
  Native-binding installation must use only the intended checkout's own environment;
  never borrow/rebind a sibling `.venv`, run `maturin develop` against shared state,
  or use manual cargo-build-and-copy. Resolve any applicable worktree-install prohibition
  explicitly before execution; propose a checkout-owned wheel/install route if needed.
- [`src/ts/package.json`](../../../../../src/ts/package.json) declares Node `>=25 <26`,
  npm 11.12.1, `@types/node ^26.1.0`, TypeScript `^6.0.3`; CI selects Node 25.9.0.
  HLD's historical actual host was Node 26.8.2. This mismatch needs a scoped proposal,
  not a claim the TS gate ran compatibly. Inspector runtime itself needs no Node/Python/TS.
- [`package.sh`](../../../../../src/inspector/tools/package.sh) builds the default offline
  Linux x86_64 GNU artifact and emits build-description/checksums; it does not establish
  a minimum glibc merely by selecting the GNU triple. The
  [artifact README](../../../../../src/inspector/package/README.md) records Ubuntu
  24.04.5/glibc 2.39 and GLIBC2.39 requirements: stock Ubuntu 22.04 cannot run that artifact.
- HLD selects Chrome for Testing/Chromium 156.0.8078.4, Playwright 1.62.1/build 1248.
  Availability and exact actual pairing must be rechecked at future qualification; old inventory
  is not a current browser test. Installed target remains one Linux x86_64/Chromium pairing.
- [CI](../../../../../.github/workflows/ci.yml) Rust change filter lists `src/rust/**`,
  `Cargo.toml`, `Cargo.lock`, excluding `src/inspector/**`. Slice 7 owns routing/gate review.
  An unchanged manifest plus Inspector-only source change can therefore miss that Rust signal.

## Scope and dependencies

In: inventory versions, tool paths/ownership, local environments, resource envelope,
architecture and platform, browser/tool pairing, local/native builders, CI relationships,
artifact provenance, external-compute options and formal CUDA-policy applicability.
Out: library sweep execution (1), cruft cleanup (2), product architecture implementation,
Engine/API/pin changes, GPU expansion, OS/driver/toolkit changes, remote provisioning,
registry publication, benchmark campaigns and fixes to other projects.

Future prep enumerates changes since this draft in plan/scope, actual source/functions,
allocated items, manifests, port evidence and owner rulings. Mark each keep/adjust/reject/
already-done with reason and path/commit evidence. Reconcile current master ownership:
new 60 owns comparison; final installed qualification is 70. Historical Slice 20 ACTIVE and
old owners do not prove a new feature slice complete. Update this draft before dependent work.

Future Slice 0 first creates and enters the release checkout under the owner decision.
It depends on scope adoption and the approved final planning HEAD, not on Slice 10.
Its reports and the later 0–9 reports use that release checkout. Slice 9 consumes its proposals;
10 applies only individually approved items. Missing runtime prerequisites do not block prose
inspection, and their absence must never be recast as a passed runtime check.

## Owner checkout decision — 2026-11-19

The [recorded owner ruling](checkout-decision.md) requires future Slice 0 first to
create **and enter** `inspector/dev/0.2.0` at
`/home/coreyt/projects/fathomdb-worktrees/inspector-0.2.0` from the approved final
HEAD of this planning branch, including port, scope and integrated plans.
All 0–9 reports then use that release checkout sequentially with one writer.
Only branch/worktree creation and entry is the action exception; installs,
configuration, source and other infrastructure changes remain proposals for
actual Slice 9 decisions and conditional Slice 10 implementation. Slice 10 has
no initial release-checkout bootstrap responsibility. This draft creates nothing.

The release checkout does not start at raw `main` or raw `dev/0.8.29`; preserve ancestry to `6edc1155f`.
Before future authorized creation, inspect branch/worktree occupancy and preflight applicability;
record explicit selected-0.8.29 ancestry and final approved planning-HEAD evidence. A generic
stale-base guard may assume `origin/main`: never pull/rebase onto it or change Engine to satisfy
that assumption. Report an incompatible guard; any needed script change is an individual
9/10 proposal, never a bypass. Run applicable strict checks without shortening diagnostics;
confirm HEAD contains approved plans. If an existing target
has divergent or unrelated work, report it and preserve it. Do not reset or silently reuse it.
The owner choice is settled; missing preflight evidence is reported without inventing setup approval.

## Local DRAFT requirements/design allocation

These local proposals clarify execution and support evidence; they add no canonical user need
and do not edit global `acceptance.md` or create a competing acceptance register.

| Existing IDs / authority | Local proposal and observable review check | Owner |
| --- | --- | --- |
| INS-X09; initial qualification target | Keep Ubuntu 22.04-compatible installed artifact target; distinguish build host, ABI floor, declared platform and tested browser. Report names/versions/provenance and an explicit unresolved 22.04 builder prerequisite. | 0 proposes; 7 assesses packaging; 9 decides; 10 approved setup; 70 qualifies |
| INS-X01/X07; V01-N07 | Separate developer bootstrap/model assets from offline installed runtime; identify checkout-owned Python/native installation and cache-only CPU prerequisites. No download/repair by Inspector. | 0 inventories; 4/40 validate supported model routes; 10 approved setup |
| INS-X03/X10; F-06 | Preserve four CPU cores, 8 GiB available RAM and scratch >= twice fixture size; inventory actual availability and ptrace capability without inventing cgroup/quiet-host requirements. | 0 proposes environment; 20–70 measure owned behavior |
| Existing UN/INS inventory | No new product needs: this slice changes no product behavior. Trace each proposed environment action to an existing test/support obligation or mark optional. | 0; 3 consolidates draft allocation |

## Deliverable steps and decision candidates

1. Inspect architecture and sources above read-only. Inventory current Rust/compiler/C tooling,
   Python/interpreter/venv/native bindings, npm/Node/TS, browser/Playwright/display,
   OS/architecture/glibc, storage and tracing prerequisites. Record actual versus historical versus
   proposed facts; no install, build, tests, broad dependency sweep or host configuration.
2. Draft minimal candidate actions with owner, necessity, risk, effort and evidence:
   local venv/native-tool route; supported Node selection versus package declaration review;
   exact browser/tool availability; Ubuntu 22.04-compatible builder and installed-test executor;
   local resources and external builder fallback. A container/VM/host is a proposal, not provisioned.
   Existing GCP/Jetson scripts are not authority to spend, connect or broaden CPU x86_64 support.
3. Apply the ruled checkout sequence: approved final planning HEAD → Slice 0 creates and
   enters the release checkout → proposal-only 0–9 reports → actual 9 decisions → approved
   local environment/tool preparation in 10 → scoped readiness checks → feature verification
   → installed 70. Create no other environment/infrastructure changes during prework.
4. Apply [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md): adopt real DBs,
   independent lexical/numeric/adjacency oracles, property invariants, targeted resource/fault/
   cancellation checks and browser/keyboard/human tasks. Defer general Engine fuzzing, global
   MC/DC and exhaustive allocation campaigns. New evidence is format 37, the bounded queue-cache
   defect/fix, missing venv and the ABI gap; these warrant targeted qualification, not new ceremony.
5. Record formal CUDA review applicability using the [accepted policy](../../../../adr/ADR-0.8.28-cuda-version-support-policy.md)
   and [review procedure](../../../../design/cuda-version-support-review.md). This CPU-only Inspector
   scope changes no CUDA artifact, driver, toolkit, importance rating or support action. Do not claim
   the recurring usage review was performed here. Identify whether a current release review can be
   referenced, or plan the mandated read-only usage/importance research during execution, with
   primary NVIDIA/peer sources and dated measured/reported/inferred facts. Carry unresolved policy
   applicability to 9; rating/action changes require owner ratification and separate scope.
6. Prepare source-backed questions for later primary-source research: Ubuntu/glibc ABI floor and
   builder suitability; browser/Playwright pairing; current Node/TS compatibility; Python native
   installation ownership; CUDA policy applicability. Do no web research in this drafting turn.
7. Have a fresh directly commissioned Sol-high reviewer assess actual proposed design changes
   and prerequisites; Terra reviews evidence/traceability and scoped document validation. No runtime
   green or artificial RED/GREEN is expected from prose. If future approved executable changes arise,
   they belong in 10 with genuine per-behavior RED commits, separate GREEN, independent code review
   and appropriate verification; do not smuggle fixes into this report.
8. Hand 7 the CI path omission, builder/ABI and gate prerequisites; hand 1 pin/version questions;
   hand 4 cache-only public-route feasibility and stale HLD helper→Caller assumption; hand 9 the
   finite proposals and unresolved environment/CUDA questions. Unsupported required positives remain
   blockers, not optional tests. No Engine change is implied by an infrastructure proposal.

## Verification, status and cleanup on execution

Check source references, proposal-versus-action language, current master ownership and the owner-ruled
checkout sequence; run relevant Markdown/plan/design/link checks from the chosen report checkout.
Use `./scripts/agent-lint-md.sh` for changed tracked Markdown; inspect diagnostics without fixing
unrelated files. Full source gates belong to source-changing slices and required release gates,
not this prose-only report. A scoped pass never claims full-workspace green.
Create only a concise execution `status.md` then: actual baseline, deliverables, checks and open
prerequisites. COMPLETE means the ruled checkout action and review/proposal deliverable are finished, not environment ready.
Preserve the shared release/planning checkout and unrelated work. Clean only slice-owned extra
isolation after verified integration; no bypass hooks, force push, tags or publication.

## Future execution prompt — do not execute during drafting

This prompt's future implementation first creates and enters the release checkout under the
[owner ruling](checkout-decision.md), then completes Slice 0's inventory/proposals and document
checks there. Other proposed installs/config/code remain unapplied until actual 9/10 approval.
Recording the ruling during planning performs none of those execution actions.
One directly spawned slice agent is sole writer, reusing the selected checkout sequentially;
additional isolation is only for necessary concurrency. Review subagents are read-only.

```text
Work on and complete Slice 0 for Inspector inspector/dev/0.2.0 (and in a worktree).

Ensure the slice has planned to do any needed work regarding requirements+acceptance criteria, design, implementation, and testing; each should be slice-complete and correct. The slice might have slice-related "change plan" documentation for guidance. The slice's plan should:
- identify and enumerate changes since slice's plan was written (review drafted slice plan, review in-repo related work done, review assigned feature/functions, review allocated draft items).
- Evaluate the enumerated output, and if needed, update/approve/reject/adjust the drafted plan, ensure it is complete but not overbuilt.
- add/adjust needs, requirements, acceptance criteria.
- write/update design (design additions and changes should be reviewed by a design review subagent (gpt-6.1-sol-high).
- The plan should then describe implementation with TDD (RED/GREEN method) and code review subagent (gpt-6.1-sol-high).
- Use a subagent (terra) to test/verify.
- Write Slice status record. Clean-up after itself: While a wt or branch should not be necessary for a slice, if a new worktree or branch was used, merge back and clean-up the worktree.

Keep ceremony to a minimum. Stay focused on the work-at-hand. When resolving a specific finding, do not run full regressions unless the changes are comprehensive enough to warrant the overhead. Generally, solve for the work outlined in the slice; check for impacts in the blast radius; be pragmatic. (do not use Steward or Orchestrator.)

After the planning for requirements+acceptance criteria, design, and implementation is planned: implement the slice according to the plan.
```
