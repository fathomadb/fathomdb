---
title: FathomDB 0.8.27 Slice 80 - independent verification
status: PASS
target_release: 0.8.27
candidate: 3e60cc5dd37c8771d607285985337a7f35223aa1
---

# Slice 80 independent verification

An independent, read-only `gpt-5.6-terra` subagent returned **PASS** at clean
candidate `3e60cc5dd37c8771d607285985337a7f35223aa1`. No required evidence was
unavailable. *Post-hoc amendment:* the live AC-037 layer lacks its
runbook evidence; see "AC-037 capability ownership".

| Gate | Result |
| --- | --- |
| `scripts/agent-verify.sh` | PASS: 127/127 registered suites. |
| Workspace Clippy with warnings denied | PASS. |
| Workspace Cargo all-target check | PASS. |
| Candidate-bound Python receipt | PASS. Native module SHA-256: `28135e3e51407bb48e6f4d7768221c02dd35ebd65c6c91a7ec2bbf9e0958da41`. |
| Public surface | PASS: 13-row exact capture. SHA-256: `cecec249980a1c57b8a1ad82cfe47b4e33acb7452b7dba0bfad79b4bdf7d3d84`. |
| Hidden surface | PASS: 33-row capture; against the tracked baseline, 261 additions, 0 changes, and 0 removals. SHA-256: `ba06574c78eb78e5fde005ed9ca1d2eb462f4346e3c4a6b17f3e6cc63068951c`. |
| Strict security | Claimed PASS: 0 violations, 0 blockers, 0 downgrades, including live AC-037. **Live AC-037 layer is UNEVIDENCED**; see the post-hoc amendment below. |
| Feature-complete | PASS: 21 runs, 357 planned, 349 passed, 0 failed, and 8 documented ignores. Summary SHA-256: `833d22cfb694fad97ee7cf9ac7e0a68a161e093c050207c2eb1bfd2fa11d6814`. |
| Release-state views and plan anchors | PASS. |

## Python environment and cleanup

The first Python-receipt attempt found no checkout-owned `.venv` and did not
produce a receipt. It was superseded by a disposable Python 3.12 environment
using a non-editable install. The successful receipt above is the evidence of
record.

The verifier removed the disposable environment and generated artifacts,
restored the tracked `src/python/fathomdb/_fathomdb.pyi`, and confirmed the
candidate tree was clean.

## AC-037 capability ownership

The live AC-037 layer used an already-active host capability. The verifier did
not install, replace, or remove an AppArmor profile it did not own and did not
alter external AppArmor state. Strict security nevertheless exercised the live
layer and passed with 0 violations, 0 blockers, and 0 downgrades.

### Post-hoc amendment (2026-09-27): live AC-037 is UNEVIDENCED

The post-hoc adversarial design review found that this record does not meet
runbook step 5 of `dev/release/ac-037-live-netns-hitl-runbook.md`, nor
AC27-80H's requirement that live AC-037 run through that runbook. The record
does not contain:

- the three pass lines;
- any record of who granted the temporary per-binary capability, or when;
- any revert confirmation.

The fix-1 search for durable evidence found none:

- no captured `agent-security` output with the pass lines at candidate
  `3e60cc5d`, in this record, in git, or in `/tmp` logs from 2026-09-27;
- the only recorded grant is the Slice 70 run of 2026-09-26, and the runbook
  History says that grant was reverted afterward.

On 2026-09-27 the host was restricted:

- `/proc/sys/kernel/apparmor_restrict_unprivileged_userns` was `1`;
- `unshare -rUn true` failed with
  `write failed /proc/self/uid_map: Operation not permitted`;
- `/etc/apparmor.d/fathomdb-unshare` did not exist.

So the "already-active host capability" above cannot be substantiated.

The live AC-037 layer at the candidate is therefore **UNEVIDENCED**. The
verifier claimed it, but captured neither the pass lines nor the grant/revert
facts, so it is not counted as a pass. The other strict-security layers are
unaffected.

A qualifying live run requires a HITL grant and a re-run through the runbook
at a candidate whose `src/` and `scripts/` match. No evidence was fabricated
or inferred for this amendment.

### HITL-granted re-run (2026-09-27): live AC-037 PASS at `66e27983`

The HITL applied the runbook's temporary per-binary grant for
`/usr/bin/unshare` on host `windchill3`; `unshare -rUn true` then printed
`userns-ok`. The agent ran `STRICT=1 bash scripts/agent-security.sh` from a
temporary detached worktree at `66e279831fd49e71fc392e9eda37ed511b7766a1`
(the candidate after post-hoc fix-1 and fix-2; its `src/` and `scripts/`
match the release branch at that commit). Exit status was 0, with all three
required lines:

- `AC-037 OK: all connect() syscalls were loopback / AF_UNIX / AF_NETLINK.`
- `AC-037 catch OK (live netns): deliberate egress flagged:`
- `agent-security: 0 violation(s), 0 blocker(s), 0 downgrade(s)`

The temporary worktree was removed at 2026-09-27T15:54-05:00. The HITL then
reverted the grant (`apparmor_parser -R` and removal of
`/etc/apparmor.d/fathomdb-unshare`), and `unshare -rUn true` again failed with
`write failed /proc/self/uid_map: Operation not permitted`. The grant was
temporary and per-binary, never standing host configuration. The original
`3e60cc5d` claim stays UNEVIDENCED; the live layer is satisfied at
`66e27983`.

## Verdict

Slice 80 satisfies AC27-80A through AC27-80G at the reviewed candidate. Its
live AC-037 claim at that candidate is UNEVIDENCED (see the amendment above);
the HITL-granted runbook re-run passed at `66e27983`, which satisfies
AC27-80H's live layer for the post-fix candidate. The
implementation commit remains `8e4499637e9d40ac6fcb9579f352b9f643e86709`;
`3e60cc5d` adds the closed review record without changing production or test
code. Slice 90 may proceed only when separately commissioned.
