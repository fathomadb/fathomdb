---
title: Inspector 0.2.0 Slice 8 — reserved placeholder
date: 2026-11-19
status: DRAFT
target_release: 0.2.0
---

# Slice 8 — reserved placeholder

The [owner checkout decision](../slice-0/checkout-decision.md) requires future
Slice 0 to create and enter the release branch/worktree from the approved final
planning HEAD. Reuse that release checkout sequentially for 0–9 reports, with
one writer. Other environment/infrastructure changes remain proposals for
actual Slice 9 disposition and conditional Slice 10 implementation. Recording
this ruling and updating drafts performs no checkout creation or slice execution.

## Reservation and baseline

Slice 8 has no allocated work. Preserve its position in the release ladder;
it is a no-op unless the owner later allocates explicitly reviewed scope.
It adds no investigation, review gate, implementation, test or prerequisite
for Slice 9. Absence of a deliverable is intentional, not unfinished work.

Inspected source: `llm/0.8.29-inspector-additional` at
`609dd85c41fb7980e05faa1132a248d376d8ec89`, clean when inspected on
2026-11-19. This is the adopted Inspector 0.2.0 scope on the unpublished
0.1.0 record-browser port, with unchanged Engine base `6edc1155f` and current
format 37. The [adopted scope](../scope.md) and current
[master allocation](../plan.md) govern; historical slice numbers and gates do
not establish completion of the current ladder.

The [design](design.md) documents the reservation, not a proposed product
architecture. Evidence consulted: `AGENTS.md`, adopted scope, the authority,
components and capability-map portions of the
[high-level design](../../../design/high-level-design.md), and the
[quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md).
Relevant actual memory entries were `dont-be-distracted-by-ceremony.md`,
`verify-design-against-code-not-just-architecture.md`,
`tdd-red-must-fail-per-finding.md` and `keep-measurement-tools-in-repo.md`.
No code or dependency contract is changed or qualified here.

## Scope and local draft delta

In scope: retain the reservation and its no-action boundary. Out of scope:
all product work, cleanup, audits, research, tool construction and changes to
Engine, SDK, MCP, schema, APIs, package pins, environment or support policy.

| Canonical area | Local DRAFT disposition | Observable check / owner |
| --- | --- | --- |
| Existing Need/Should and INS requirements | No addition or change: no behavior allocated. | This reservation adds no contract; existing feature owners remain as allocated in the master plan. |
| V01/X/F/T acceptance; INS-05.7–05.10/M40 | No addition, waiver, reassignment or completion claim. | Existing canonical registers retain their obligations; comparison belongs to current Slice 60. |
| Architecture and design | No change to components, routes or lifecycle. | Reservation remains a no-op; current Slice 70 retains integrated qualification. |

Do not edit canonical needs, requirements, acceptance or architecture to make
this empty slot appear substantive. No competing acceptance register is created.
The quality roadmap supplies no applicable runtime campaign for an unallocated
placeholder. Real-DB, oracle, property, resource, cancellation, browser and
human checks stay with allocated feature work; generic fuzzing and global
MC/DC campaigns gain no authorization here. No new technical evidence alters
that assessment.

## Future handling and change control

Do not commission a Slice 8 agent merely to occupy the slot. If the release
runner explicitly dispatches it while still reserved, its own directly spawned
agent is the sole writer and records only a short actual no-op status in the
selected report worktree. Reuse that worktree sequentially; do not execute
in the primary checkout or create an extra branch/worktree for this reservation.
The future release branch must descend from the approved final planning HEAD,
including port, scope and plans, rather than raw `dev/0.8.29` or `main`.

At such a dispatch, only reconcile the latest allocation against this draft:
enumerate changes to this plan, assigned source/features and allocated draft
items, if any. Keep the reservation if unchanged; mark a removed reservation
already done if explicit evidence establishes that disposition. Reject an
unsupported assignment. An actual owner allocation requires adjusting this plan
and design to the reviewed scope before any work starts; it is not permission
to infer a task from nearby findings. Use the [shared workflow](../slice-workflow.md)
for that later substantive assignment, including its relevant review and TDD
requirements. No prior elapsed time, empty slot or generic prompt grants scope.

For the unchanged reservation, there are no requirements/design/code reviews,
RED/GREEN runs, Terra runtime tests, web-research questions or cleanup actions
to invent. Document edits, if made, use the applicable Markdown checks; do not
run full workspace verification or borrow a Python environment. Status may say
`RESERVED — no work allocated`; if dispatch bookkeeping requires `COMPLETE`,
qualify it as reservation handling only, with no product or runtime-green claim.
Create no execution status during this drafting turn.

Slice 9 consumes the unchanged reservation alongside Slices 0–7. It does not
wait for a fabricated positive gate from Slice 8. Missing mandatory positives
elsewhere remain open and cannot be converted into unavailable passes here.
Preserve release worktree and unrelated work; no hooks bypass, force push, tag
or publication is authorized. There are no slice-owned extra branches to merge
or remove under the unchanged plan.

## Future execution prompt

**Placeholder constraint:** the following generic prompt authorizes only the
reservation handling above while Slice 8 is unallocated. Its “implement” and
review/testing language does not create work, research or gates. Perform no
substantive action unless the owner later allocates reviewed scope and the
plan/design are revised accordingly.

```text
Work on and complete Slice 8 for Inspector inspector/dev/0.2.0 (and in a worktree).

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
