---
title: Worktree and Branch Consolidator — implementation design
date: 2026-08-13
desc: Local, manifest-gated implementation of the worktree and branch consolidation protocol
status: PROPOSED
blast_radius: local Git worktrees, refs, and operator archives only; no product API or remote mutation
refs:
  - dev/design/worktree-branch-consolidation.md
  - dev/design/orchestration.md
---

# Worktree and Branch Consolidator — implementation design

## 1. Objective

Implement `scripts/worktree-consolidator.py`, a stdlib-only local Git operator
tool that applies the [worktree and branch consolidation protocol](worktree-branch-consolidation.md).
The tool has four authority-separated modes:

| Mode | Purpose | Permitted mutation |
| --- | --- | --- |
| `audit` | Observe and classify one repository snapshot. | None. |
| `manifest` | Solve a declared or inferred topology goal into a candidate manifest. | No Git mutation; may explicitly write a candidate manifest outside every registered worktree. |
| `dryrun` | Rehearse one independently approved manifest against the current repository state. | No Git mutation; may explicitly write a receipt outside every registered worktree. |
| `consolidate` | Apply one approved and freshly rehearsed exact retirement manifest. | Local bundle/archive creation, approved worktree removal, and approved local-ref deletion only. |

The tool must not infer that semantic work is equivalent merely because a ref is
merged or `git cherry` reports no unmatched patch. It may identify candidates;
the manifest and reviewer decide what is retired.

## 2. Non-goals and authority boundary

The tool does not:

- reset, clean, checkout, rebase, merge, cherry-pick, commit, or force-push;
- modify a remote ref or invoke `git push`;
- delete a worktree with tracked, untracked, or ignored content;
- delete a branch not explicitly named in an approved manifest;
- widen a manifest when a fresh audit discovers more candidates; or
- decide product/release content or turn an `integration-required` branch into
  a merged branch automatically.

Semantic integration remains a human-reviewed operation in a deliberately
chosen integration worktree. `consolidate` performs only preservation and
retirement transitions whose proof obligations are already witnessed.

## 3. CLI

```text
scripts/worktree-consolidator.py audit [OPTIONS]
scripts/worktree-consolidator.py manifest --audit SNAPSHOT --evidence-dir PATH [OPTIONS]
scripts/worktree-consolidator.py dryrun --manifest APPROVED-MANIFEST --owner-map PATH --approval-attestation PATH --archive-dir PATH --evidence-dir PATH [OPTIONS]
scripts/worktree-consolidator.py consolidate [OPTIONS]
```

Common options:

| Option | Meaning |
| --- | --- |
| `--repo PATH` | Repository root; defaults to the current Git root. |
| `--baseline REF` | Ref used for ancestry evidence; defaults to `origin/main`. Its current SHA is recorded, never presumed fresh. |
| `--baseline-attestation PATH` | JSON evidence that the named baseline was fetched by an operator at the stated time and SHA; required for an executable plan, `dryrun`, and `consolidate`. |
| `--owner-map PATH` | Strict JSON ownership/release-role mapping. It is required for a snapshot eligible for an executable manifest, and for `dryrun`/`consolidate`; absent or unknown ownership blocks retirement. |
| `--policy PATH` | Strict JSON theme/goal/recovery policy; required by `manifest`, and its canonical hash must match later stages. |
| `--evidence-dir PATH` | Durable manifest/receipt/attestation directory for `manifest`, `dryrun`, and `consolidate`; must be outside every registered worktree. |
| `--json` | Emit canonical machine-readable JSON to stdout. |
| `--output PATH` | `manifest` only: explicitly write the otherwise-stdout candidate directly under `--evidence-dir`. |

`audit` emits a snapshot and a classification proposal to stdout. It never
creates a bundle, lock, archive ref, report file, Git object, or any other
filesystem artifact. It records the observed baseline ref/SHA as `unattested`;
manifest generation later validates a baseline attestation against that exact
snapshot. An unattested audit can support analysis but not a retirement plan.

`manifest` consumes an audit snapshot and policy, then solves for a declared or
inferred goal. Its command-specific options are:

| Option | Meaning |
| --- | --- |
| `--audit PATH` | Immutable JSON snapshot emitted by `audit`. |
| `--policy PATH` | Strict policy file describing themes, ranges, and reflog dispositions. |
| `--owner-map PATH` | Strict owner/release-role map whose hash must match the audit snapshot. |
| `--evidence-dir PATH` | Durable directory containing the baseline attestation and candidate output. |
| `--target-worktrees N` | Desired steady-state active-worktree count, `1 ≤ N ≤ 64`. |
| `--infer-target` | Infer a feasible target instead of accepting a number. Mutually exclusive with `--target-worktrees`. |

It emits the only manifest format `consolidate` may execute, but the result is
a candidate until independently approved with a separate attestation. Without
a valid baseline attestation it emits a review-only candidate, never an
executable manifest. It validates that the operator-designated durable evidence
directory is resolved, non-symlink-ambiguous, outside all registered
worktrees, owned by the effective user, and not group/world writable; it must
contain the baseline attestation before the tool reads or writes plan evidence.

`dryrun` consumes the approved manifest, matching owner map, approval and
baseline attestations, archive destination, durable evidence directory, and
current repository state. It re-audits, checks every proof witness, validates
the archive and evidence destinations through metadata/access checks (resolved
path, device, inode, effective-user ownership, restrictive mode, and no-
symlink result), and simulates the exact ordered transaction without creating a
bundle, removing a worktree, deleting a ref, or writing under a registered
worktree. It writes one expiring receipt under the evidence directory
containing the manifest and approval hashes, current snapshot ID, expected
actions, and predicted post-state. It returns non-zero if any action would be
blocked; it never repairs or regenerates the manifest.

`consolidate` requires all of:

```text
--manifest approved-manifest.json
--owner-map PATH
--approval-attestation PATH
--archive-dir PATH
--evidence-dir PATH
--baseline-attestation PATH
--dryrun-receipt PATH
--freeze-attestation PATH
--confirm-manifest-sha256 HEX
--confirm "CONSOLIDATE <manifest-id>"
```

`--manifest` must resolve to a regular non-symlink file directly under
`--evidence-dir`, just like every attestation and receipt. A candidate printed
to stdout is useful for review, but it is not executable until the exact file
is placed in that durable evidence directory and independently approved.

The confirmation hash is the canonical JSON SHA-256 of the manifest. This makes
the user confirm the exact plan, not merely a filename that could change.

The approval attestation is an independently created JSON record with the
candidate manifest SHA-256, reviewer identity, decision, review time, and
expiry. `dryrun` and `consolidate` reject an absent, non-approved, mismatched,
or expired attestation; a reviewer-looking field in a manifest is not approval.
It is an operator-attestation control, not a cryptographic signature scheme:
the tool establishes an auditable independent-review precondition but cannot
authenticate a reviewer against a same-host attacker. Use externally signed
attestations or a protected evidence store when that threat model is required.

The baseline attestation is an operator-created JSON record with baseline ref,
SHA, fetch time, and expiry. The tool checks that its SHA matches the current
baseline and rejects an expired attestation. The freeze attestation identifies
the manifest SHA-256 and snapshot, known writers/automations, accountable
operator, issue time, and short expiry. It is an explicit human coordination
control, not a claim that a local process lock can stop arbitrary Git commands.

The dry-run receipt is tool-generated JSON with the approved manifest hash,
observed snapshot ID, baseline-attestation hash, result, issue time, and expiry.
`consolidate` requires an unexpired successful receipt. A receipt is evidence
that the plan was possible at a point in time, not permission to skip the
transaction's own revalidation.

All approval, baseline, freeze, and dry-run receipt paths must resolve to
regular non-symlink files directly under the resolved `--evidence-dir`; the
tool rejects paths elsewhere or beneath a registered worktree. This gives the
attestation chain one durable, inspectable retention boundary.

Before publishing a bundle, `consolidate` checks that every deterministic
receipt name for that manifest (preservation, execution, and each possible
progress record) is unoccupied. A pre-existing name therefore aborts before
retirement. If a post-preservation execution-receipt write still fails, the
tool publishes a separately named, no-clobber partial receipt that records the
failure and completed actions; preservation and progress receipts remain the
recovery trail.
If the evidence storage itself cannot accept that fallback, the tool exits as
`PartialBatch` and reports the preservation receipt location and completed
action count. No program can durably record a receipt while the designated
storage is unavailable; this terminal report identifies the already-durable
recovery evidence for operator follow-up.

## 4. Target inference and constrained planning

The tool supports both an operator-supplied target and an inferred target.

```text
target = --target-worktrees N
      | infer_target(snapshot, owner_map, policy)
```

The inference function does not guess through uncertainty. It first forms the
protected theme set from `protected-active` and `integration-required` entries.
One worktree is required for each distinct active theme, plus one clean primary
checkout and, only when legacy work remains, one triage checkout:

```text
lower_bound = 1 + active_theme_count + legacy_triage_required
if lower_bound > policy.target_range.max:
    goal_inference_blocked
else:
    inferred_target = max(policy.target_range.min, lower_bound)
```

If a target lies outside `policy.target_range`, falls below `lower_bound`, or
ownership/theme assignment is unknown, the output is
`goal_inference_blocked`; no retirement is proposed. Themes, target range, and
reflog-candidate dispositions are policy inputs, not hardcoded project
knowledge. The strict policy schema is:

```json
{
  "primary_role": "main",
  "active_themes": ["campaign", "earp", "performance", "cuda-memex"],
  "theme_targets": {
    "campaign": "/absolute/path/to/campaign-worktree",
    "earp": "/absolute/path/to/earp-worktree"
  },
  "legacy_triage_required": true,
  "target_range": [5, 7],
  "baseline_max_age_seconds": 900,
  "dryrun_max_age_seconds": 900,
  "retire_local_heads": false,
  "reflog_candidates": {
    "<commit-sha>": "preserve-in-bundle|unresolved"
  }
}
```

Unknown policy keys, duplicate theme assignments, malformed commit IDs, or a
candidate absent from the audit's reflog/unreachable-candidate set fail loudly.
`theme_targets` maps every active theme to one protected or
integration-required **worktree path** and the planner never retires a mapped
target; theme targets must be distinct. `retire_local_heads` is false by default; when true, local-head
retirement is separately proof-gated only for an already unused local head.
Canonical policy bytes are hashed and carried in the manifest, dry-run receipt,
and execution receipt. Audit remains purely observational and has no policy
input; manifest generation binds one audit snapshot to one explicit policy.

The planner minimizes this lexicographic objective:

```text
minimize (
  unresolved_transitions,
  dirty_uncaptured_transitions,
  active_theme_collisions,
  worktree_surplus,
  local_ref_surplus
)
```

The first three terms are hard constraints: a candidate plan with a non-zero
value is rejected, never traded for fewer worktrees. Candidate reduction uses
only classifications with retirement proof; it does not solve by deleting a
branch with unreviewed content.

## 5. Snapshot and evidence collection

All Git subprocesses use argument arrays, a scrubbed Git-location environment,
`GIT_OPTIONAL_LOCKS=0`, and a fixed repository cwd. The tool must not invoke a
shell. Audit uses read-only Git invocations only; its tests detect any index,
ref, worktree, or other repository file mutation.

`audit` collects:

- `git worktree list --porcelain` for path, HEAD, branch/detached, and lock;
- per-worktree `git status --porcelain=v1 --ignored` counts, never file payloads
  in the default report;
- local refs, upstreams, and ref use by each registered worktree;
- ancestry to the selected baseline and exact duplicate tip SHAs;
- `git cherry` only as an advisory relation;
- locally extant reflog tips and unreachable commit candidates, without source
  payloads, so prior-cleanup recovery work is explicit;
- strict owner-map entries and active Git/process ownership evidence, including
  the canonical owner-map SHA-256; and
- primary checkout identification by resolving each worktree's
  `git rev-parse --git-dir` and comparing it to the resolved
  `git rev-parse --git-common-dir`; only the matching worktree is primary; and
- canonical real paths for the repository root, Git common directory, every
  worktree, and every operator-supplied output/receipt/archive path.

The recovery-candidate set is the sorted union of commit IDs that are reachable
from a reflog but not from a current local ref, and commits reported by
`git fsck --no-reflogs --unreachable --no-progress`. Non-commit unreachable
objects are reported as counts only. The policy must assign every candidate a
disposition; `preserve-in-bundle` adds it to the deterministic bundle inputs
and `unresolved` blocks retirement.

The snapshot receives a deterministic identifier:

```text
snapshot_id = sha256(canonical_json(snapshot_without_timestamp))
```

`canonical_json` is UTF-8 JSON with recursively sorted object keys, separators
`,` and `:`, no insignificant whitespace, no floating-point values, normalized
absolute real paths, and explicitly sorted sets/lists where their order is not
semantic. SHA-256 is computed over those exact bytes.

Audit records `baseline_freshness: "unattested"` because it does not fetch or
read attestations. Fetching changes local remote-tracking refs and requires
separately authorized network activity. An operator obtains an attestation
after an explicit fetch; `manifest`, `dryrun`, and `consolidate` validate its
bounded lifetime and exact SHA.

An audit must distinguish “Git does not know this” from “safe.” For example,
an absent upstream, missing owner, unavailable process inspection, or an
unreadable worktree yields `unresolved` rather than a deletion candidate.

## 6. Manifest schema and validation

The output of `manifest` is a versioned JSON manifest. It is intentionally
separate from the repository release-state JSON; neither is the writer for the
other.

```json
{
  "schema": "fathomdb-worktree-consolidator/v1",
  "plan_sha256": "<sha256 of payload without manifest_id or plan_sha256>",
  "manifest_id": "wtc-<snapshot-sha8>-<plan-sha8>",
  "snapshot_id": "<sha256>",
  "repository": {
    "primary_root": "/canonical/absolute/path",
    "git_common_dir": "/canonical/absolute/path/.git"
  },
  "baseline": {
    "ref": "origin/main",
    "sha": "<sha>",
    "attestation_sha256": "<sha256>",
    "expires_at": "<RFC3339>"
  },
  "baseline_requirement": {"max_age_seconds": 900},
  "policy_sha256": "<sha256>",
  "owner_map_sha256": "<sha256>",
  "goal": {"target_worktrees": 6, "source": "inferred"},
  "preservation": {
    "bundle_name_algorithm": "wtc-bundle-v1",
    "include_all_local_refs": true,
    "required_tips": ["<sha>"],
    "reflog_candidates": ["<sha>"]
  },
  "entries": [
    {
      "kind": "worktree|branch",
      "target": "absolute-worktree-path|refs/heads/name",
      "classification": "merged-retirable|archive-required",
      "owner": {
        "value": "none|<owner-id>",
        "release_role": "none|<role-id>",
        "evidence_sha256": "<owner-map-sha256>"
      },
      "action": "remove_worktree|delete_local_ref",
      "witness": {
        "tip": "<sha>",
        "clean": true,
        "unused_by_retained_worktree": true,
        "recovery_requirement": "execution_bundle|retained_ref:<ref>"
      }
    }
  ],
  "approval_requirement": {"max_age_seconds": 86400},
  "dryrun_requirement": {"max_age_seconds": 900},
  "freeze_requirement": {
    "snapshot_id": "<sha256>",
    "max_age_seconds": 900
  }
}
```

`baseline_requirement.max_age_seconds` must equal
`policy.baseline_max_age_seconds`, and `dryrun_requirement.max_age_seconds`
must equal `policy.dryrun_max_age_seconds` when `manifest` generates the
candidate. Later modes enforce these serialized values and do not need to infer
them from a policy file.

The strict owner-map schema is:

```json
{
  "schema": "fathomdb-worktree-owner-map/v1",
  "entries": [
    {
      "target": "absolute-worktree-path|refs/heads/name",
      "owner": "none|<owner-id>",
      "release_role": "none|<role-id>",
      "evidence": "<durable-pointer>"
    }
  ]
}
```

Targets are canonical real paths or fully-qualified local refs; entries are
unique and sorted. All currently auditable worktrees/local heads require an
entry. `owner: "none"` and `release_role: "none"` are explicit assessments;
any other value blocks retirement. Canonical owner-map bytes are hashed in the
audit snapshot, manifest, dry-run receipt, and execution receipt. Later modes
must receive a map with that exact hash.

`plan_sha256` is SHA-256 over the canonical manifest payload after omitting
exactly `manifest_id` and `plan_sha256`. `manifest_id` is then
`wtc-<snapshot_id first eight hex>-<plan_sha256 first eight hex>`. The final
confirmation hash is SHA-256 over the complete canonical manifest, including
both fields; it is distinct from `plan_sha256` and has no self-reference.

`bundle_name_algorithm: "wtc-bundle-v1"` derives the final basename only after
`plan_sha256` is known:
`refs-before-wtc-<snapshot-id first eight hex>-<plan-sha256 first eight hex>.bundle`.
The derived value is not serialized into the plan-hash payload. It must match
that exact pattern, contain no path separators or `.`/`..`, and have no
overwrite-target semantics.

The generated manifest stays immutable. The separate approval attestation names
the manifest's SHA-256 and is the independent-review gate. The later freeze
attestation likewise names that SHA-256 and required snapshot, so neither can
change the confirmation hash.

Attestations and receipts are strict versioned JSON documents. An approval
attestation contains repository identity, manifest SHA-256, reviewer identity,
`decision: "approved"`, issue time, and expiry. A baseline attestation contains
repository identity, baseline ref/SHA, fetch time, issue time, and expiry. A
dry-run receipt contains repository identity, manifest SHA-256,
approval-attestation SHA-256, baseline-attestation SHA-256, snapshot ID,
owner-map SHA-256,
canonical archive/evidence directories, deterministic bundle path/name,
archive/evidence destination attributes (device, inode, owner UID/GID, mode,
and no-symlink result), expected ordered actions, result, issue time, and
expiry. A freeze attestation contains repository identity, manifest SHA-256,
dry-run-receipt SHA-256, snapshot ID, accountable operator, named
writers/automations, issue time, and expiry. Each is canonical JSON and all
references are checked by hash.

Every attestation and receipt is valid only when `issued_at ≤ now ≤ expires_at`,
its age is at most the applicable requirement, and its expiry is no later than
`issued_at + max_age_seconds`; future timestamps and inverted intervals fail.
Approval and freeze use their manifest requirements, dry run uses
`dryrun_requirement`, and baseline uses `baseline_requirement`.

Validation is consumed-or-loudly-rejected: unknown keys, missing keys,
duplicate targets, a target outside `refs/heads/`, a
classification/executable-action mismatch, or a
snapshot/baseline/repository/policy mismatch fails before any mutation.
Required tips and reflog candidates must be sorted, unique, and present in the
audit evidence. A retirement entry must have `owner.value: "none"` and
`owner.release_role: "none"` bound to the manifest owner-map hash. An approved
manifest is immutable: amendments create a new manifest with a new hash.

## 7. Consolidation transaction

`consolidate` is a fail-closed sequence:

1. validate the unexpired successful dry-run receipt plus the baseline and
   approval/freeze attestations before acquiring a process lock under the Git
   common directory;
2. require repository identity, policy hash, owner-map hash, `snapshot_id`, and
   baseline SHA to match both the manifest and dry-run receipt;
3. resolve and require exact equality of the archive/evidence directories and
   bundle path against the dry-run receipt; re-stat the directories and require
   equal device, inode, owner UID/GID, mode, and no-symlink attributes, then
   validate every entry’s retirement proof immediately before its action;
4. require the archive directory to remain pre-existing, writable,
   effective-user-owned, non-group/world-writable, and non-symlink-ambiguous;
   create the deterministic bundle from all local refs plus manifest-selected
   reflog candidates in a newly created, no-clobber temporary file there; fsync
   it, require `git bundle verify` to report no prerequisites, verify every
   required tip with `git bundle list-heads`, hash it, publish it to the
   previously nonexistent final basename through an atomic no-clobber hard-link
   (never `os.replace`), remove the temporary link, fsync the parent directory,
   and write a fsynced preservation receipt proving required-tip coverage before
   removing any target. A filesystem lacking this safe publication primitive
   fails closed;
5. apply worktree actions first, one entry at a time, retaining their branches;
6. revalidate branch use, then delete only manifest-listed local heads with an
   expected-old-SHA compare-and-delete (`git update-ref -d <ref> <expected>`),
   never a `git branch -d` merge-status shortcut;
7. re-audit, verify invariants, and append a final or partial receipt.

The lock is released on all paths. It coordinates only consolidator instances;
the freeze attestation and per-action revalidation defend against unrelated
Git activity. A failed action or any changed witness stops the remaining batch;
the receipt records which entries did and did not occur. There is no rollback
by destruction: recovery is `git fetch <bundle> <ref>` or a newly created
archive ref under explicit operator control.

`consolidate` refuses a manifest if any target is the primary checkout, a
locked/detached worktree, dirty, in use by a retained worktree, outside the
resolved repository identity, or no longer matches its stated SHA/witness. A
changed target requires a new audit, manifest, approval, dry run, and
confirmation.

Worktree removal is deliberately non-atomic: under the documented freeze
assumption, which prevents all writes including ignored/generated content,
invoke `git worktree remove` without `--force`; a Git safety error aborts the
remaining batch and writes a partial receipt. The tool does not claim that Git
will detect every concurrent mutation or make arbitrary external activity safe.

The execution receipt records the resolved archive directory, bundle path,
bundle SHA-256, `git bundle verify` result, exact bundle inputs, coverage of
every required old tip/reflog candidate, every completed action, and the
post-state snapshot. Receipts and attestations live in an operator-supplied
durable evidence directory outside all registered worktrees; the operator is
responsible for backing up that directory before retirement. The tool rejects a
nonexistent, symlink-ambiguous, or in-worktree evidence/archive location.

## 8. Exit status and output contract

| Exit | Meaning |
| --- | --- |
| 0 | Requested operation completed and all applicable proofs passed. |
| 1 | Safety/invariant/manifest failure; no new target action began. |
| 2 | CLI or schema usage error. |
| 3 | Goal blocked by unresolved state or target lower bound. |
| 4 | Partial `consolidate` batch; receipt identifies completed actions. |

Default human output is concise. `--json` provides the complete structured
record. Neither form contains source payloads from untracked files or secrets.

## 9. TDD and verification plan

Implementation begins with tests using temporary Git repositories and fake
worktree layouts. The tests must assert:

- `audit` makes no filesystem or ref mutation;
- `audit` refuses `--output` and its report labels an unattested/expired
  baseline as non-executable;
- a primary plus linked worktree fixture identifies only the common-dir-matched
  worktree as primary and always refuses its removal;
- `manifest` is deterministic for a fixed snapshot/policy, respects declared
  and inferred target bounds, and rejects an output path inside any registered
  worktree;
- `manifest` blocks when its lower bound exceeds policy maximum and carries the
  exact canonical policy hash and repository identity;
- `manifest` serializes policy-validated baseline/dry-run age requirements so
  later modes enforce them without a policy input;
- `manifest_id`, `plan_sha256`, and full confirmation hash follow the specified
  non-recursive canonical-hash construction;
- `dryrun` makes no Git mutation, rejects archive/evidence paths inside any
  registered worktree or through ambiguous symlinks, simulates actions in
  transaction order, and produces a receipt only if every action is currently
  feasible;
- `consolidate` rejects archive/evidence paths that differ from the dry-run
  receipt, altered directory attributes, the manifest path, and every
  attestation/receipt path outside the evidence directory;
- inferred targets respect the active-theme lower bound;
- an unresolved owner, dirty worktree, lock, primary checkout, detached state,
  changed baseline, changed tip, or ref-in-use blocks retirement;
- duplicate target entries and unknown/missing manifest fields fail loudly;
- canonical JSON, repository identity, approval-attestation, and policy-hash
  mismatches fail loudly;
- absent, changed, malformed, or owner/release-role-non-`none` owner-map
  evidence blocks retirement in every executable stage;
- reflog-only and unreachable commit candidates are reported and each requires
  a policy disposition; selected candidates are present in the verified bundle;
- a complete, hash-verified bundle is created before the first removal;
- bundle publication rejects an existing final name, verifies no prerequisites
  and required heads, and leaves no published bundle after a failed temporary
  write/verification/publication step;
- only exact manifest entries are acted on, even if a fresh audit finds new
  candidates;
- worktree removal does not delete its branch;
- local-ref deletion is refused until all retained worktree uses are absent;
- local-ref deletion uses the witnessed expected old SHA and stops on a
  compare-and-delete mismatch;
- detected worktree mutations or Git safety errors between revalidation and
  non-force removal stop the batch and record a partial receipt; the ignored-
  content race remains an explicit freeze-assumption boundary, not a claimed
  Git-detection guarantee;
- a mid-batch failure writes an honest partial receipt and performs no later
  action; and
- expired, SHA-mismatched, or snapshot-mismatched baseline, dry-run, or freeze
  attestations, or an absent/expired approval attestation, block before bundle
  creation, and a target changed between actions stops the remaining batch; and
- the post-state verifies the protocol invariants and fixed-point measure.

Run targeted Python tests and `python -m py_compile` during the tool slice,
then `./scripts/agent-verify.sh` before shipping.

## 10. Rollout

Land in stages:

1. pure data model, snapshot parser, and `audit` with fixtures;
2. planner, target inference, manifest validator, and side-effect-free
   `manifest`;
3. dry-run receipt support, then bundle/receipt support and `consolidate`
   behind explicit manifest hash plus confirmation;
4. exercise one intentionally tiny, already recoverable test repository before
   authorizing use on FathomDB’s active checkout.

No stage automatically executes against the current topology. The first real
run is a baseline-attested `audit`; `manifest` then generates the candidate
manifest. A human review approves that exact candidate, `dryrun` rehearses it
against the still-current state, and `consolidate` applies the same hashed
manifest only while its dry-run and freeze attestations remain valid.
