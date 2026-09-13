---
title: FathomDB 0.8.26 Slice 9 — approved preparation design
status: COMPLETE
execution_authorized: seq-289
---

# Slice 9 design — approved preparation design

## Approved items

The narrow preparation bundle is ruled at `seq-286`:

- repair 0.8.25 publication-state truth and add lifecycle validation;
- create 0.8.26 state and board only after that repair;
- make the authoritative main ref explicit in preflight;
- correct the exact maintained authority/platform/CLI documentation identified
  by Slice 2; and
- correct `download-artifact` v4.3.0 comments without a runtime upgrade.

Checkout-owned tools, fresh artifact environments, and disk-budget checks are
execution conditions. Windows inventory consolidation belongs to Slice 50.
Markdown dependency remediation is excluded unless it becomes a hard build
blocker and HITL explicitly admits it.

## Change boundaries

### Publication and release state

The published 0.8.25 record, machine state, and generated views must agree.
Both `scripts/check-public-doc-truth.py` and `scripts/release-current.py` fail
closed when `release_kind` declares publication complete without a complete
publication receipt. Any present receipt must be an object whose tag matches
the state version and whose `tag_commit`, `published_on`, and `npm_dist_tag`
are respectively a full lowercase SHA, an ISO date, and a non-empty string.
The exact 0.8.25 repair records tag `v0.8.25`, peeled commit
`e2934867436eb9859f31574ad05abf632314906a`, publication date `2026-09-12`,
and npm dist-tag `latest`.

The 0.8.26 single-writer state and board are created only after that invariant
is green. Activation records Slices 0–7 as `COMPLETE_ON_RELEASE_BRANCH` at
closeout commit `880a86ea857105fac61b04e695c08bd76b5ac85d`, Slice 8 with that
status at `7a5682796bc446ac38e5053cff5a18e9d33e9c00`, and Slice 9 as
`IN_PROGRESS`. It sets `landed` to an empty list, `active_ref` to
`refs/heads/release/0.8.26`, `published` to null, and the remaining ladder to
begin with Slice 9. Closeout changes Slice 9 to `COMPLETE_ON_RELEASE_BRANCH`
at the prior implementation commit, advances `next_slice` and the remaining
ladder to Slice 10, and never records the closeout commit as its own evidence.
The complete ladder is 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 20, 30, 35, 40,
45, 46, and 50. Ruled and unruled decisions are copied from current plan
authority. The state owns these exact generated regions:

- `plan-immediate-next` under `## Immediate next slice` in
  `dev/plans/plan-0.8.26.md`;
- `status-current-state` under `## Current state` in
  `dev/plans/runs/STATUS-0.8.26.md`;
- `status-next-action` under `## Immediate next action` in that board; and
- `status-live-open-count` under `## Open decisions` in that board.

It does not select `status-unblocks` or its additional fact schema. All regions
are produced with
`scripts/check-release-state-views.sh --write`, never hand-edited.

### Preflight authority

Preflight performs no fetch. It prefers resolvable `origin/main` as the
authoritative main ref. If that ref is absent, it uses resolvable local `main`
and emits an explicit warning that remote authority is unavailable. It
hard-fails if neither ref resolves. The JSON result reports both `main_ref`
and `main_sha`; a stale local branch therefore cannot masquerade as current
remote authority. Remote-only and offline-local behavior are deterministic
and the fallback is never silent.

### Documentation truth

Only these maintained documents change:

- `dev/platform-capabilities.json`: update current-release metadata while
  preserving the platform matrix;
- `src/ts/README.md`: replace future-release/native-platform wording with
  published 0.8.25 truth;
- `dev/plans/README.md`: correct the stale 0.8.24 and 0.8.25 status rows; and
- `AGENTS.md`: correct the Rust workspace-member inventory and the false claim
  that exactly one release-state file exists.

CLI verb/reference work remains assigned to Slice 30; wildcard cleanup of
`dev/design/*.md` is excluded. Historical evidence is not rewritten. No
dependency, action runtime, product contract, packaging, or feature behavior
changes under a documentation fix.

P26-08 changes exactly seven comments in `.github/workflows/release.yml` at
the `d3f86a106a0bac45b974a628896c90dbdf5c8093` download-artifact pin from
`# v8.0.1` to `# v4.3.0`. The SHA and runtime stay unchanged. Actionlint and
the existing CUDA release-contract check are the focused witnesses.

## Slice 9 requirements and acceptance criteria

| ID | Requirement | Acceptance criterion |
| --- | --- | --- |
| R26-09A | Published lifecycle state is internally complete and consistently interpreted. | AC26-09A: both state readers reject publication-complete/null and malformed receipts; the public-document reader selects 0.8.25 as newest published, while the active-release selector is empty until 0.8.26 activation and then selects 0.8.26. |
| R26-09B | The active 0.8.26 release has one valid machine state, board, and generated view set. | AC26-09B: state/view and release-current checks pass, normal 0.8.26 worktree preflight selects the 0.8.26 plan, and closeout points to Slice 10 without a self-referential SHA. |
| R26-09C | Preflight reports a deterministic no-network main authority. | AC26-09C: stale-local, remote-only, local-only fallback, and neither-ref fixtures prove the defined ref and diagnostic behavior, including `main_ref` and `main_sha`. |
| R26-09D | Maintained planning/platform/SDK guidance reflects current repository truth. | AC26-09D: only the four enumerated documents change and their statements match manifests, workspace membership, and release state. |
| R26-09E | Download-artifact comments identify the pinned runtime truthfully. | AC26-09E: exactly seven comments name v4.3.0, the seven SHAs are unchanged, and actionlint plus the CUDA release-contract check pass. |

## Design constraints

- Preparation changes remain separable from feature changes.
- Slice 6–7 fixes enter Slice 9 only when independent of post-10 product
  artifacts; later dependencies remain in their explicitly assigned slices.
- Accepted contract changes preserve ADR supersession and interface-document
  authority.
- Build and CI/CD fixes use non-publishing regression witnesses first and do
  not weaken tests, preflight, gitleaks, action pins, or credential boundaries.
- Test-infrastructure repairs must not skip or reinterpret a failing product
  test.

## Verification design

Each behavioral item has a failing regression witness before implementation.
P26-01 covers lifecycle consistency; P26-02 covers state/view validity and
preflight; P26-03 covers explicit-ref, stale-local, remote-only, and offline
cases. Documentation edits use source-of-truth comparison rather than invented
behavioral tests. Evidence records source finding, files, RED/GREEN commits,
focused checks, repository gate, reviewer verdict, `seq-286`, and proof that
later-allocated work was not pulled forward.

Before broad verification, record a checkout-local environment witness. The
shared root `node_modules` symlink is inadmissible: replace it with a clean
checkout-owned `npm ci` result using the declared lockfile/tool contract.
Because repository policy forbids editable Python installation from a linked
worktree, use version-verified standalone lint/typecheck executables or a
non-editable checkout-local tool environment, and use fresh external wheel
environments for artifact claims; do not create an editable worktree `.venv`.
The primary checkout's environment is not evidence. Record free disk and
require the existing preflight minimum before running the broad
non-publishing gate.
