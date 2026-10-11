---
title: FathomDB 0.8.27 Slice 80 - independent verification
status: PARTIAL_HISTORICAL_REVIEWED
target_release: 0.8.27
historical_candidate: e9631b9761d292a4115d1beee95678801512f4f2
reviewed_candidate: b7403958a3839d371c1672335c517fa762a451cf
authoritative_binding: REVIEW_PASS_VERIFICATION_PARTIAL
---

# Slice 80 independent verification

The full independent, read-only `gpt-5.6-terra` execution below returned
**PASS** at historical candidate
`3e60cc5dd37c8771d607285985337a7f35223aa1`. Those receipts are evidence for
that exact historical candidate only. Production subsequently changed through
`0efa62c544af00858aa6975944e8f36c99f13218`, so the Terra receipts do not
verify later production or the former reviewed candidate
`e9631b9761d292a4115d1beee95678801512f4f2`. The production changes have
focused test, Clippy, and rustdoc surface-diff evidence, but there is no
post-fix full canonical PASS and no official post-fix public or hidden capture.
The live AC-037 layer claimed at `3e60cc5d` also lacks its runbook evidence;
see "AC-037 capability ownership".

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
`3e60cc5d` claim stays UNEVIDENCED. The `66e27983` pass is valid historical
evidence for that post-fix candidate only; it does not cover later HEAD, the
`9700991f` property follow-up and its lint-only `31e78529` adjustment, or the
final release candidate.

### Owner ruling: exact-candidate AC-037 is deferred

The repository owner ruled on 2026-09-27 that no further live AC-037 claim is
made for current Slice 80 HEAD. Slice 150 must execute the HITL runbook on the
exact final candidate, capture the required pass/catch/summary lines and
grant/revert evidence, and bind the receipt to that SHA. This avoids repeated
host-capability changes while later binding and SDK slices can still change the
candidate.

## Follow-up review and candidate binding

The result-codec follow-up added generated coherent typed round-trip and
positional evidence-corruption properties. Each killed its specified
temporary production mutant without changing its test or oracle, passed after
exact byte restoration, and passed the focused and full `slice60_wire` routes
at test implementation candidate `31e78529`. Production remains byte-for-byte
unchanged after `0efa62c544af00858aa6975944e8f36c99f13218`.

An independent, read-only `gpt-5.6-sol` subagent at high reasoning reviewed
the tests, mutation evidence, records, and Slice 85 architectural handoff in
three cycles. The final verdict is **PASS** at clean reviewed candidate
`e9631b9761d292a4115d1beee95678801512f4f2`; the two earlier cycles and their
dispositions are recorded in `code-review.md`.

The statement formerly made here that the historical Terra receipts remained
applicable to later production was incorrect. Production changed through
`0efa62c5`; only the focused tests, Clippy routes, and rustdoc surface diff
recorded in the implementation and review chronology cover those changes.

At exact historical candidate `e9631b97`, attempts to replace the missing
canonical evidence did not complete:

- the public capture refused to run with less than 100 GB free;
- the hidden capture stopped when `nvidia-smi` exited 9; and
- `agent-verify` could not pass because a historical-reference fixture is
  intentionally incompatible with the advanced release reference.

At live `24813b8eb321ae5a12e2943d33b1a30dc1b7e121`, whose `src/` is identical
to `31e78529` and `e9631b97`, an unconfined `agent-verify` run established:

- strict security, including AC-036 and both AC-037 layers: 0 violations, 0
  blockers, and 0 downgrades;
- `test-rust`: PASS in 516,820 ms; and
- 127 suites registered, 125 run, 123 passed, 2 failed, 2 skipped, and 0
  excluded. Both failures were environmental: the checkout lacked the local
  `fathomdb._fathomdb` native module, and the candidate-bound native receipt
  consequently failed.

This is useful diagnostic evidence, not a post-fix canonical PASS. No official
post-fix public or hidden capture exists. The `66e27983` runbook receipt and
the live `24813b8e` security result are not release qualification: by owner
ruling, Slice 150 alone must run AC-037 through the runbook on the exact final
candidate and bind the grant/run/revert receipt to that SHA.

Before Slice 85 moves any code, a capable host must establish a successful
exact-baseline `agent-verify`, candidate-bound native receipt, exact public
capture, and hidden capture. This is a Slice 85 preflight and acceptance
prerequisite, not evidence that Slice 80 already obtained those post-fix
receipts.

## Verdict

The independent rereview at `b7403958` is a review PASS for the strengthened
property and corrected Slice 85 plan. It may support release-state binding, but
it does not alter this verification record's limits: no post-fix canonical
PASS, official post-fix public or hidden capture, or final-candidate AC-037
pass is claimed. The Terra receipts remain limited to `3e60cc5d`; the partial
`24813b8e` diagnostic run and the blocked `e9631b97` captures are not passes.
Slice 85 still needs the capable-host exact-baseline prerequisite, and Slice
150 alone qualifies live AC-037 on the exact final candidate.
