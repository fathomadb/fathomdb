---
title: 0.8.24 Slice 10 — main CI assessment and integration draft plan
status: DRAFT
target_release: 0.8.24
---

# Slice 10 — main CI assessment and integration

## Planning boundary

This document plans Slice 10. It does not perform the assessment, edit a
workflow, start hosted CI, or make the slice ready. The slice must complete its
own preparation and design review before any implementation decision.

The default outcome is **no CI change**. The proportional-routing and
fast/heavy-bootstrap work is already on `main`, including `5e2a05e2`. Slice 10
integrates that current-main contract into 0.8.24 planning; it does not recreate,
backport, or overwrite it on the release branch.

## Goal and outcome

Establish the smallest truthful interface between current `main` CI and the
0.8.24 target-distribution work:

1. prove what current `main` already routes and verifies;
2. identify a concrete selector, artifact-transfer, or smoke gap only if one
   actually follows from the approved Tegra or Windows CUDA designs;
3. preserve informational, proportional CI and the fast/heavy ownership split;
4. make a narrow current-main change only if the reviewed evidence disproves
   the no-change presumption; and
5. hand Slices 30, 40, and 70 an explicit CI/release interface record.

## Authority and inputs

- `dev/plans/plan-0.8.24.md`, especially P24-12, R24-7, A24-5, and the
  merged-main assignment.
- `dev/plans/0.8.24/prework/{main-ci-interface,ci-and-release-controls}.md`.
- `dev/plans/0.8.24/prework/slice-6-hitl-decisions.md`.
- Current `origin/main`, not the release branch's remembered baseline.
- `5e2a05e2`, `.github/workflows/{ci,release}.yml`,
  `scripts/bootstrap-heavy.sh`, and their local contract tests.
- Slice 30 and Slice 40 ready-design inputs when those drafts identify a real
  target route. Their absence is not permission to speculate.
- Memory `fathomdb-ci-single-maintainer-minimal-not-gated.md`: CI is
  informational and diff-scoped; no required-check, merge-queue, soak-period,
  or administrative full-suite ceremony is introduced.

Accepted ADRs and the release design remain authoritative. A workflow is an
implementation of those contracts, not a substitute architecture.

## Scope

### In scope

- Compare the execution-time `origin/main` workflow and test contract with the
  committed `5e2a05e2` landing and subsequent CI changes.
- Map relevant file changes to the existing `changes` classifier and jobs.
- Confirm ownership of `verify-fast`, `verify`, Windows WAL, native-artifact,
  security, release, and target-evidence routes.
- Document how target slices request or transfer evidence without requiring a
  release-wide hosted run.
- If and only if a concrete route is missing, plan and implement the smallest
  classifier/job/dependency edge plus its local contract test.

### Non-goals

- Rewriting CI, adding branch protection, required checks, a merge queue,
  nightly schedules, soak periods, or a required aggregator job.
- Recreating `5e2a05e2` on `release/0.8.24`.
- Building Tegra or Windows CUDA artifacts; configuring runners; changing
  registry publishers; or dispatching a release workflow.
- Using a full hosted CI run to confirm a local structural change when no
  external executor behavior is in question.

## Slice prep — planned first phase

Slice execution begins by creating these durable records under this directory:

- `prep.md` — goals, current-main SHA, inputs, and evidence inventory;
- `draft-contracts.md` — draft needs, requirements, and acceptance signals;
- `design.md` — the reviewed CI/release-interface design; and
- `research.md` — primary-source questions, findings, and applicability.

These records remain slice-local drafts until the slice reviewer accepts them.
They do not edit canonical product requirements or architecture by themselves.

### Prep tasks

1. Fetch and record current `origin/main`; verify whether `5e2a05e2` is an
   ancestor and inspect every later change to the relevant workflow/scripts.
2. Restate the accepted inputs:
   - R24-7: current-main CI must be compatible with the release topology;
   - P24-12: no-change presumption, current-main ownership, no ceremony run;
   - A24-5: no architecture change without a demonstrated route gap.
3. Propose, for review, slice-local draft statements such as:
   - **N10-DRAFT:** the maintainer needs fast, informational feedback whose
     cost and platform scope follow the changed surface;
   - **R10-DRAFT-1:** existing proportional routing and fast/heavy ownership
     remain unchanged unless a named target route cannot be selected;
   - **R10-DRAFT-2:** a CI edit must have a local executable contract test and
     must not require a hosted full-tree confirmation merely for integration;
   - **AC10-DRAFT:** the current-main diff and contract tests either prove no
     change is needed or identify one exact missing route and its proof.
4. Read `dev/architecture.md`, `dev/design/release.md`, the Tier-1 platform ADR,
   and relevant interfaces. Record that this slice changes no runtime SDK
   contract unless evidence establishes otherwise.
5. Inspect actual workflow/test bodies in full before claiming what they assert:
   `test_ci_proportional_routing.py`, `test_ci_long_job_efficiency.sh`,
   `test_bootstrap_heavy.sh`, Windows WAL workflow tests, and native-artifact
   workflow tests.
6. Produce an exists-versus-net-new map. A job name, path filter, or runner
   label is structural evidence only; it is not target hardware evidence.

## Draft design and design review

### Initial design

The expected design has three boundaries:

1. `ci.yml` remains informational and proportionally routes ordinary changes.
2. `release.yml` owns explicit release rehearsal/publication and target artifact
   assembly; it is not made an automatic consequence of administrative edits.
3. Slices 30/40 own target-specific executor and evidence production. Slice 10
   changes CI only if their reviewed design names a route current main cannot
   select or validate.

The design must include a job/file ownership table, trigger/event table,
permissions table, artifact producer/consumer edges, path-classifier mapping,
and the exact local tests for every proposed YAML/script change.

### Challenges and primary-source research plan

- Check GitHub's current workflow-event, path-filter, reusable-workflow, job
  permission, environment, and self-hosted-runner documentation only where a
  proposed edge depends on it.
- For a self-hosted route, review GitHub's secure-use guidance for public
  repositories and prove default-branch workflow ownership rather than trusting
  labels or prose.
- If artifact handoff is proposed, review GitHub artifact retention and
  attestation documentation and state what provenance is verified locally.
- Do not research or add generic CI “best practices” unrelated to a concrete
  0.8.24 gap.

Primary references begin with:

- <https://docs.github.com/en/actions/reference/security/secure-use>
- <https://docs.github.com/en/actions/reference/runners/self-hosted-runners>
- <https://docs.github.com/en/actions/concepts/security/artifact-attestations>

### Architectural-fit review and revision

The reviewer checks the draft against the release design, accepted ADRs,
current workflow code, and the owner-approved low-ceremony posture. Revise the
design to remove any speculative job, required gate, duplicated main work, or
claim that structural CI proves hardware. Record the revision and verdict in
`design.md` before implementation can be considered.

## Planned execution after prep approval

### Phase 1 — current-main assessment

- Work in a fresh isolated branch/worktree from the then-current `origin/main`.
- Produce a current-main evidence matrix for triggers, classifier outputs,
  jobs, runners, permissions, artifacts, and local contract tests.
- Compare the approved Slice 30/40 route needs against that matrix.

### Phase 2 — decision branch

**No gap:** make no workflow/script change. Complete the interface record and
route target-specific needs back to their owning slice.

**Concrete gap:** stop for reviewer acceptance of the exact change. Then use
TDD for the local workflow contract, make only the accepted YAML/script edit,
and retain the no-required-check/no-full-dispatch boundary.

### Phase 3 — documentation and handoff

- Update the applicable design/plan index only if material documents changed.
- Hand Slice 30/40 the route/transfer contract and Slice 70 the final ownership
  statement.
- A newly discovered prerequisite is assigned to its actual owner. Runner
  selection belongs to Slice 30/40; registry identity belongs to Slice 30/40;
  release evidence assembly belongs to Slice 70.

## Verification and evidence

For a no-change outcome:

- current-main ancestry/diff record;
- the relevant local CI-contract tests against current main;
- `actionlint` on the unchanged workflow as an assessment witness; and
- a durable no-change disposition naming what was compared.

For a narrow change:

- RED then GREEN local contract test for the exact route;
- existing proportional-routing, long-job, and heavy-bootstrap tests;
- `actionlint .github/workflows/ci.yml .github/workflows/release.yml` as
  applicable;
- scoped shell/Python lint for changed helpers; and
- `git diff --check` plus plan/document lint.

No hosted full workflow is a default acceptance step. A target-host execution
is requested only by the target slice whose behavior cannot be proven locally.

## Risks and recovery

| Risk | Control / recovery |
| --- | --- |
| Release branch overwrites newer CI | Start from current `origin/main`; never backport the landing from this branch. |
| A structural test is mistaken for hardware proof | Keep executor evidence with Slice 30/40 and label structural evidence precisely. |
| A narrow edit fans out to all jobs | Require an exact classifier/needs-edge rationale and mutation-sensitive local contract. |
| Hosted CI becomes a ceremony gate | Preserve informational status and require external execution only for an actual external claim. |
| Main advances during work | Recompare before review/landing; serialize edits to shared workflows. |

Workflow changes are reversible by a reviewed follow-up commit. Do not use a
destructive history rewrite or overwrite current main.

## Decisions and prerequisites for the next reviewer

The Slice 10 reviewer must decide:

1. whether current main fully supplies the interface (recommended default);
2. if not, the one concrete gap and smallest correction;
3. whether any proposed external execution proves a fact unavailable locally;
4. which target slice owns each new prerequisite.

The slice cannot become ready while its plan contains a speculative target job
or an unresolved shared-workflow ownership conflict.

## Definition of done

Slice 10 closes when current-main behavior is accurately documented, every
claimed gap is evidenced, the no-change path has been genuinely considered,
any accepted narrow edit has local contract proof, no hosted ceremony run was
manufactured, and Slices 30/40/70 have an explicit interface handoff.
