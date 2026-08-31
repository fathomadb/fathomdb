---
title: 0.8.24 Slice 10 — accepted slice-local CI contracts
status: ACCEPTED
target_release: 0.8.24
---

# Slice 10 — accepted slice-local CI contracts

These statements govern Slice 10 and its handoff. They do not mint canonical
product requirements or edit `dev/needs.md`, `dev/requirements.md`, or
`dev/acceptance.md`.

## Draft review and disposition

| Draft | Disposition | Accepted wording |
| --- | --- | --- |
| N10-DRAFT | **Adjust and accept locally.** | The maintainer needs fast, informational feedback whose cost and platform scope follow the changed surface, with an explicit trusted lite route for administrative changes. |
| R10-DRAFT-1 | **Adjust and accept locally.** | Main owns proportional CI and the fast/heavy partition. `ci.yml` provides PR/main feedback; `release.yml` separately owns tag/dispatch artifact rehearsal, publication, and target smokes. |
| R10-DRAFT-2 | **Accept locally.** | Any later workflow behavior mutation requires a meaningful mutation-sensitive RED contract, GREEN after the implementation, and `actionlint`; current structural evidence is never target-hardware proof. |
| AC10-DRAFT | **Adjust and accept locally.** | Current-main ancestry, workflow inspection, and named executable contracts must either identify one exact gap or justify a no-code result. They justify no code for this slice. |
| Canonical CI product requirement | **Reject.** | CI topology is release/process design, not a new user-facing product requirement. |
| Speculative target route | **Reject.** | Slice 10 does not invent Tegra identity, Windows SDK support, runner labels, artifact paths, or target smokes. |

## Accepted requirements

- **R10-1 — main-owned proportional feedback.** `ci.yml` remains owned on
  current main, triggers on pull requests and main pushes, and retains the
  existing classifier plus fast/heavy ownership split.
- **R10-2 — explicit release boundary.** `release.yml` remains a distinct
  `v*` tag or operator-dispatch path for release gates, artifacts, publishers,
  and target smokes. `[ci-lite]` never suppresses it.
- **R10-3 — proportional, non-ceremonial policy.** Slice 10 adds no required
  status check, merge queue, soak period, required aggregator, scheduled full
  run, or manually dispatched hosted full-tree confirmation. “Informational”
  is the owner-approved policy; this record does not claim an unqueried GitHub
  repository setting.
- **R10-4 — proof fidelity.** Job text, runner labels, and local workflow
  structure prove configuration only. Hardware, toolchain, installed-package,
  registry, and publisher facts require evidence from their owning target or
  release slice.
- **R10-5 — later-change discipline.** If Slice 30 or 40 later proves a
  concrete missing route, that slice revises its own ready plan, identifies
  the exact shared-workflow change, and applies RED/GREEN/actionlint. Slice 10
  is not held open waiting for that possibility.

## Acceptance evidence

| Signal | Evidence | Result |
| --- | --- | --- |
| Exact proportional routing and trusted-lite semantics | `scripts/tests/test_ci_proportional_routing.py` | PASS |
| Fast/heavy cache and advisory ownership | `scripts/tests/test_ci_long_job_efficiency.sh` | 24 passed, 0 failed |
| Heavy-only bootstrap behavior and routing | `scripts/tests/test_bootstrap_heavy.sh` | 11 passed, 0 failed |
| Windows WAL job remains structurally attributed | `scripts/tests/test_windows_wal_attribution_ci_job.sh` | PASS; structural evidence only |
| Existing five-target CPU native-artifact route | `scripts/tests/test_native_artifact_runtime_validation.sh` | PASS; structural evidence only |
| Existing release-ready topology | `scripts/tests/test_release_contract_truth.sh` | All mutation fixtures passed |
| Workflow syntax and action semantics | `actionlint` with `.github/actionlint.yaml` | PASS |
| No current-main integration delta | ancestry, log, and path-scoped diff recorded in `prep.md` | PASS |
