---
title: Worktree Consolidator Execution Status — design
date: 2026-08-15
desc: Read-only durable execution-status design for cross-namespace callers
status: PROPOSED
blast_radius: local consolidator read-only status mode and execution evidence only
refs:
  - dev/design/worktree-consolidator-execution-status-requirements.md
  - dev/design/worktree-consolidator-execution-status-acceptance-criteria.md
  - dev/design/worktree-branch-consolidator.md
---

# Worktree Consolidator Execution Status — design

## 1. Decision

Add a fifth, read-only mode:

```text
scripts/worktree-consolidator.py status \
  --repo REPO --manifest EVIDENCE/manifest.json --evidence-dir EVIDENCE --json
```

It reports durable execution state for one immutable manifest. It is an
observer, not a recovery command. In particular, it does not inspect PID
liveness: PID namespaces make absence ambiguous, while a cooperative lock and
durable receipts are observable state.

## 2. Inputs and derived paths

`status` validates the repository, private evidence directory, and exact
canonical manifest exactly as other executable modes do. The manifest hash and
entry count derive the only receipt namespace it may inspect:

```text
preservation-<manifest-prefix>.json
execution-<manifest-prefix>.json
progress-<manifest-prefix>-0001..NNNN.json
partial-<manifest-prefix>-<16-lower-hex-nonce>.json
```

It observes the cooperative lock at
`<git-common-dir>/worktree-consolidator.lock` with `lstat`, not `exists`, so a
broken symlink is not mistaken for absence. It never opens, parses, returns, or
acts on the lock payload. Any present lock entry—regular, unreadable, malformed,
directory, or symlink—dominates to `executing`; an `lstat` error other than
absence is a safety error. The implementation does not invoke `kill`, `ps`, or
read `/proc`.

Receipt parsing uses canonical JSON and direct regular-child paths. Every entry
in the exact manifest-prefix namespace is inspected without following
symlinks. A foreign full manifest hash, malformed name, non-regular entry,
progress name outside exactly `0001..NNNN`, or more than one randomized fallback
partial is a safety error.

The following exact cross-record checks apply:

| Record | Required invariant |
| --- | --- |
| Preservation | Existing exact schema, matching repository/manifest hash, `verified_before_retirement: true`, and bundle path/SHA/covered-tip set equal to the manifest's required tips plus recovery candidates. |
| Progress `i` | Existing exact schema, matching repository/manifest/preservation hash, and `completed_actions` exactly equals manifest entries `0..i`; every earlier number exists. |
| Deterministic final | Existing exact schema, matching repository/manifest/preservation hash, bundle path/SHA/covered tips equal preservation, and `result` exactly `success` or `partial`. Success requires all `N` progress receipts, all `N` manifest actions, and a non-null post snapshot. Partial may be at most one action ahead of its terminal progress receipt because action precedes progress publication. |
| Fallback partial | Existing fallback schema, exact manifest/repository/preservation/bundle linkage, `result: partial`, valid action-prefix relationship, and an ignored-for-output failure string. It dominates a coexisting deterministic success final. |

Any malformed final is a safety error even while a lock is present.

## 3. State machine

```text
lock present ───────────────────────────────────────────────► executing
  └─ valid final receipt also present ───────────────────────► executing/finalizing

lock absent + no execution evidence ─────────────────────────► not_started
lock absent + valid final success + all actions + no fallback ► completed
lock absent + valid deterministic/fallback partial ─────────► recovery_required
lock absent + preservation/progress without final success ──► recovery_required
```

`executing` deliberately has no timeout or "stale" variant. A hidden executor
can write progress while a caller cannot see its PID. `recovery_required`
means the operator must stop and investigate the bundle/evidence; it does not
make the old manifest reusable.

The canonical response is metadata-only:

```json
{"completed_actions":2,"expected_actions":15,"liveness":"unknown","lock_present":true,"manifest_sha256":"<hash>","result":"executing","schema":"fathomdb-worktree-execution-status/v1"}
```

For `recovery_required`, the fixed `operator_action` is
`"preserve evidence; investigate; do not clear lock or resume this manifest"`.
For `executing`, it is `"monitor durable receipts; do not clear lock or start another consolidation"`.
No result includes a PID, source content, or an execution-control instruction.

## 4. Failure handling and compatibility

An unsafe evidence directory, invalid manifest, contradictory receipt chain,
or missing required field raises `SafetyError`; no report file is written. The
mode does not create a lock, bundle, or receipt and invokes no mutating Git
command. Existing modes retain their protocol and deterministic receipt names.

The code centralizes receipt-path derivation and prefix validation so status and
consolidate agree about action ordering. Existing crash safety remains unchanged:
a process killed before its final receipt still needs human recovery, but a
caller can no longer mistake a PID-namespace boundary for proof that it is safe
to interfere.

## 5. Test plan

RED tests create canonical fixture receipts directly, then assert the four
states, finalizing and fallback-partial precedence, no-PID-probe behavior,
strict malformed-chain rejection, and read-only fingerprints. They exercise
ordinary final-write failure and final-path-published-then-directory-fsync
failure, plus every exact-prefix filename edge. GREEN adds the smallest
stdlib-only status parser/validator and CLI wiring. Existing consolidator tests
remain the regression suite; no test sends a signal or starts a background
process.
