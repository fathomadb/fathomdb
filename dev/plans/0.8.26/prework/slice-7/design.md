---
title: FathomDB 0.8.26 Slice 7 — CI/CD failure evidence model
status: COMPLETE
---

# Slice 7 design — CI/CD failure evidence model

## Delivery-stage model

Classify each incident at the first stage that failed after a successful
product build:

1. CI workflow validation and permissions;
2. target build/test matrix;
3. package assembly and metadata/content inspection;
4. gitleaks and supply-chain checks;
5. signing, attestations, and provenance;
6. artifact upload or GitHub Release assembly;
7. registry authentication and publication; or
8. clean post-publish install/open/close/exit smoke.

Downstream symptoms remain linked to the first failure but are not counted as
independent root causes.

## Failure record

Use the Slice 6 evidence fields plus release/tag, workflow/job, target artifact,
registry, credential scope without secret value, retry count, and last known
successful boundary. Mark evidence as source-build, non-publishing package,
staging-equivalent, or real-registry proof.

## Correction classes

- **Slice 9:** feature-independent workflow validation, diagnostics, pinning,
  package scaffolding, or local dry-run infrastructure approved as prework.
- **Reserved post-10 slice:** a correction or hardening witness that requires
  one or more implemented 0.8.26 features or candidate artifacts.
- **Slice 50:** integrated packaging matrix, signing/provenance assembly,
  registry-free publication simulation, and final release evidence.
- **Postpone/reject:** low-value, speculative, externally resolved, or riskier
  than the demonstrated delay warrants.

Reserved placement may use 11–19, 21–29, 31–39, or 41–49 without renumbering
the mod-10 feature ladder. Slice 8 records the dependency and insertion point.

## Safety boundary

All proposed fixes have a local or non-publishing witness first. Credential and
registry testing requires later explicit authority. Gitleaks findings are
handled as security evidence; the durable review contains only safe locators
and remediation state, never secret material.

## Completed failure register

| ID | Finding and confidence | Smallest correction and proof | Draft placement |
| --- | --- | --- | --- |
| S7-01 | Confirmed: duplicate Windows structural and installed-binding inventories twice expected test-only probes absent from release wheels. Fixes `4e78afd1` and `18fefc67` preceded the final 307/307 proof. | Derive both inventories from one machine-readable contract while retaining mutation tests. | Slice 9 |
| S7-02 | Confirmed and closed: release fixtures inherited local-main, dispatch-event, model-cache, and macOS path assumptions. Fixes and focused tests already landed. | Preserve regression tests; do not reimplement the closed fix. | Slice 9 verification only |
| S7-03 | Probable recurrence: the Windows runner required PATH, Python, Maturin, and PowerShell repair; complete diagnostics are unavailable. | Add a fail-fast runner prerequisite probe with accepted tool ranges. Keep provisioning out of product code. | Slice 9 or post-10 |
| S7-04 | Confirmed: exact-path Gitleaks registration for a benign digest caused deterministic closeout delay and is likely to recur for versioned evidence. | Decide between generated schema-known digest registration and exact per-artifact checklist registration. Keep positive and credential-shaped mutation tests. | Slice 8 decision; Slice 9 or Slice 50 |
| S7-05 | Confirmed persistent, non-blocking: full-history Gitleaks reports 21 unknown fingerprints and is intentionally advisory; current-tree scanning remains gating. | Bounded sanitized triage and rotation of any live secret. Do not restore it as a release gate implicitly. | post-10 security hardening |
| S7-06 | Confirmed recurring transient: PyPI visibility required a third Intel macOS smoke attempt after successful publish; prior npm CDN behavior is similar. | Poll exact-version visibility or retry only exact-version-unavailable, never runtime/import/test failures. | Slice 50/release |
| S7-07 | Confirmed downstream symptom of S6-02, not a separate root cause: tag and registry publication succeeded while canonical publication state stayed incomplete and post-tag CI remained red. | Use the single S6-02 lifecycle correction; distinguish candidate-time from post-publish facts. | consolidated into Slice 9 S6-02 |
| S7-08 | Possible: overlapping exact-SHA candidate dispatches consumed runners, but retained evidence does not prove they were accidental. | After confirmation, reuse/watch an in-progress exact-SHA run; do not globally serialize intentional reruns. | postpone/post-10 |
| S7-09 | Confirmed configuration drift: seven release action refs are labeled v8.0.1 but pin v4.3.0. | Validate all affected hosted/self-hosted runner classes, action runtime prerequisites, dependent static guards, and non-publishing artifact flow before changing the pin. | Slice 9 |

## Evidence and scoring

| ID | Evidence locator | Recurrence / delay | Stability risk / effort |
| --- | --- | --- | --- |
| S7-01 | Slice 85 `code-review.md`; commits `4e78afd1`, `18fefc67`; final 307/307 proof | twice in final candidate sequence; principal part of 92 minutes to successful-run start | low product risk, medium CI-contract risk / medium |
| S7-02 | commit `36b7b86d`, 21 preflight assertions, 22 Python tests; macOS fix `d1bff6f7` | repeated fixture assumptions; now closed | regression-only low risk / low |
| S7-03 | Slice 85 status/closeout Windows PATH, Python, Maturin, PowerShell account | plausible runner recurrence; exact delay unavailable | medium runner risk / medium |
| S7-04 | Slice 85 closeout and 24/24 security tests for versioned digest path | likely each versioned evidence path; deterministic closeout delay | high security-policy risk / medium |
| S7-05 | `dev/design/ci-challenges-review-20260821.md`; advisory count 100 expected/121 observed | persistent, non-gating in publish run `34703662459` | high security-triage risk / medium-high |
| S7-06 | publish run `34703662459`; Intel macOS PyPI attempts `103583977860` then `103584342209` | proven PyPI and prior npm recurrence; retry latency only after publish | low if narrowly classified / medium |
| S7-07 | post-tag runs `34703643542`, `34705717950`, `34707655130`, `34708289554`; same null receipt as S6-02 | deterministic; verifier remains red | scored once under S6-02 |
| S7-08 | multiple workflows for exact SHA `36b7b86d`; intent not retained | possible runner waste; delay unproven | medium cancellation risk / medium |
| S7-09 | seven `.github/workflows/release.yml` `d3f86a...` refs labeled v8.0.1; actual v8.0.1 `3e5f45...` used elsewhere | deterministic drift, no demonstrated failure yet | medium-high release data-flow risk / medium |

The score inputs above are deliberately coarse. Slice 8 performs the common
understood/risk/effort/include scoring and may request more evidence where
confidence is below confirmed.

The final 0.8.25 candidate sequence took about 92 minutes from the first failed
run to the successful run start and about 114 minutes to green completion.
Durable closeout records say the corrective commits changed verification/CI
material, not native product bytes. The actual v0.8.25 publish run succeeded
across registries; its history-advisory failure was non-gating.

Failed-job log bodies have expired or are unavailable, so detailed Windows
host diagnoses remain medium confidence. No current-tree Gitleaks failure or
credential/publication-permission defect was found, and no secret value is
recorded here.
