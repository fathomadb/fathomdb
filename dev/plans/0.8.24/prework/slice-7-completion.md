---
title: 0.8.24 Slice 7 — accepted prework completion
status: COMPLETE
target_release: 0.8.24
---

# Slice 7 — accepted prework completion

Slice 7 implemented exactly the four owner-accepted packages. No feature-slice
code, CI/release workflow, runner, registry, environment, publication, tag, or
release-state change occurred.

## S7-01 — root Markdown security remediation

- **Owner decision:** P24-01.
- **Baseline:** `npm audit --json` reported two findings through
  `markdownlint-cli2@0.23.0 → js-yaml@5.2.0` (one moderate and one high).
- **Changed files:** `package.json`, `package-lock.json`.
- **Change:** updated to `markdownlint-cli2@0.23.2`, resolving
  `js-yaml@5.2.2`; corrected the root security provenance comment.
- **GREEN:** `npm audit --json` reported zero vulnerabilities;
  `npm ls markdownlint-cli2 js-yaml` reported 0.23.2 / 5.2.2;
  `bash scripts/agent-lint-md.sh` and `git diff --check` passed.
- **Commit:** `3f976dd4` (`build: update markdownlint tooling`).

## S7-02 — remove unused Prettier tooling

- **Owner decision:** P24-03.
- **Baseline:** the supported-command scan found Prettier only as the root dev
  dependency/lock entry and obsolete bootstrap wording; no active command
  invoked it.
- **Changed files:** `package.json`, `package-lock.json`,
  `scripts/bootstrap.sh`.
- **Change:** removed the root Prettier dependency/lock entry and corrected the
  bootstrap message while retaining the Markdown safety policy.
- **GREEN:** `npm ci` completed with zero vulnerabilities; the supported-command
  scan found no invocation; `bash scripts/agent-lint-md.sh` and
  `git diff --check` passed.
- **Commit:** `0a84035b` (`build: remove unused prettier tooling`).

## S7-03 — maintained public-link and release-currency correction

- **Owner decision:** P24-04, plus the owner-authorized four-file scope
  amendment recorded in Slice 6.
- **Baseline:** `npm view fathomdb version` and `python3 -m pip index versions
  fathomdb` both reported 0.8.23; bounded scans found former-owner URLs and
  stale active 0.8.21/held-0.8.22 guidance.
- **Changed files:** `mkdocs.yml`; maintained Python/TypeScript/Rust package
  READMEs; the approved active `docs/**` surfaces; the three historical
  release-note current-version banners; `dev/DOC-INDEX.md`; and
  `dev/doc-index/docs.md`.
- **Change:** updated active repository routes to `fathomadb/fathomdb` and
  active published-release guidance to 0.8.23. Historical release detail stayed
  untouched except its stale current-version banners.
- **GREEN:** bounded active-surface scan found no stale former-owner route or
  active current-release assertion; `bash scripts/agent-lint-docs.sh`,
  `mkdocs build --strict`, `bash scripts/agent-lint-md.sh`, and
  `git diff --check` passed.
- **Commit:** `f21146b2` (`docs: refresh published release guidance`).

## S7-04 — active engineering navigation correction

- **Owner decision:** P24-05.
- **Baseline:** `dev/README.md` called the superseded 0.8.6–0.8.16 schedule
  master; active indexes declared completed 0.8.23 live.
- **Changed files:** `dev/README.md`, `dev/plans/README.md`,
  `dev/DOC-INDEX.md`, `dev/doc-index/plans.md`.
- **Change:** pointed active navigation to the 0.8.20–0.9.0 schedule and the
  release-state lookup rule; removed fixed claims that 0.8.23 is current.
- **GREEN:** `npx markdownlint-cli2 dev/README.md dev/plans/README.md
  dev/DOC-INDEX.md dev/doc-index/plans.md`, `bash scripts/lint-plans-status.sh`,
  `bash scripts/lint-plan-anchors.sh`, and `git diff --check` passed.
- **Commit:** `ded32301` (`docs: correct active release navigation`).

## Scope and final state

- Slices 10–70 remain feature work with their own draft-to-ready plans.
- Dependabot remains paused; Pyright and broader dependency upgrades remain
  postponed; no archive/delete work was performed.
- The S7-03 plan amendment was owner-authorized and did not change the
  pre-approved historical-evidence boundary.
