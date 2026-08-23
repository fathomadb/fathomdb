---
title: 0.8.24 Slice 6 — HITL decisions
status: INITIAL-DECISIONS-RECORDED
target_release: 0.8.24
---

# Slice 6 — HITL decisions

## Initial decision

**Owner decision, 2026-08-23:** “Approved.” This records approval of the
Slice 6 register's stated recommendations and feature allocations. The approval
does not silently invent the target-specific choices the register marked as
external prerequisites; those remain explicit inputs to each feature slice's
separate draft-to-ready plan.

## Recorded dispositions

| Register IDs | Owner disposition | Effect |
| --- | --- | --- |
| P24-01, P24-03, P24-04, P24-05 | **Accepted for Slice 7.** | The accepted-work plan contains exactly these four bounded maintenance packages. |
| P24-02 | **Postponed.** | Pyright remains at its reviewed pin; no Slice 7 work. |
| P24-06 | **Accepted: keep paused.** | Dependabot configuration does not change. |
| P24-07 | **Rejected for this release.** | No archive/delete work enters Slice 7. |
| P24-08 | **Accepted feature direction, choice deferred to Slice 30.** | Use a separate, explicit public Tegra distribution; do not publish the invalid `+tegra` form. Slice 30 must record the exact identity and publisher route before implementation. |
| P24-09, P24-10 | **Accepted feature direction, prerequisites remain explicit.** | Slice 40 is authorized to draft the Windows CUDA route, but cannot implement until it records the selected SDK surface and an approved remote CUDA executor. No local Windows build is authorized. |
| P24-11 | **Accepted for Slice 20.** | Review the retained SCALE-02 engine delta under its no-rerun decision rule; it remains a separate ready-plan and implementation decision. |
| P24-12 | **Accepted for Slice 10.** | Retain the no-change presumption and integrate only from current main; no release-branch recreation or ceremony CI run. |
| P24-13 | **Accepted for Slice 50.** | Obtain and compare actual Memex evidence before proposing a WAL product change. |
| P24-14, P24-15 | **Accepted for Slice 60.** | Align retry-safe completion semantics and preserve CPU artifacts with target-native installed smokes for every selected route. |

## Boundaries reaffirmed

- Slice 7 implements only P24-01, P24-03, P24-04, and P24-05. It is not a
  prerequisite gate for Slices 10–70 and it must not absorb their artifact,
  CI, package, executor, or verification work.
- No release tag, publication, registry/environment/secret mutation, runner
  configuration, hosted workflow dispatch, or broad dependency sweep is
  authorized by this decision.
- The feature decisions above authorize their own separate planning. They do
  not declare a Tegra public name, Windows SDK matrix, or remote executor that
  has not yet been observed and recorded.

## Next decision point

Slice 6 now submits the detailed Slice 7 plan to its required independent
read-only review. After up to two documented correction cycles, the owner must
make the final Slice 7 plan disposition before implementation starts.
