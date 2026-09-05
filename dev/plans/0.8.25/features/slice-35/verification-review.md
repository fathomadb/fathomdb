---
title: 0.8.25 Slice 35 independent verification
status: VERIFIED
verified_commit: 74c191b5
updated: 2026-09-05
---

# Slice 35 independent verification

Independent verification reproduced the final FIX-7 RED/GREEN boundary,
validated the authoritative performance receipt, and found one closeout wording
error. Commit `74c191b5` corrected that wording and passed re-verification with
no remaining factual blocker.

## Verified evidence

- RED `380399e1` changes only three test files. In a clean disposable clone its
  exact regression set produced three expected failures and four passes.
- GREEN `386783e9` makes that same set pass 7/7 without changing tests. The
  current focused classification, SCALE-02, Slice 35, and mutation-audit suite
  passes 46/46.
- The authoritative receipt is
  `scale-02-slice35-20260905T0404Z-659c38ea`. Its record is clean and binds the
  runner and invocation git blobs at `386783e9`.
- Both measured arms truthfully identify `Engine.search_text_only`, record
  exactly 5,500 calls, and report zero errors and timeouts.
- Independent paired-bootstrap recomputation produced 1.623% p50 and 2.121%
  p95 upper relative regression, both inside the preregistered 3% boundary.
- The repository-portable classification validator passes without the external
  `data/` tree. Configuration, plan, artifact, wheel, binary, extension, record,
  and result hashes resolve.
- Policy version 3 quarantines exactly four historical receipts that falsely
  labelled the operation as `Engine.search`; it does not rewrite them. The 0347
  receipt remains valid but is superseded by the authoritative rerun.
- The final FIX-7 sequence changes no executable product/API implementation
  source. The measured native candidate is `0aff1cb0`; later `67f708fb` adds a
  public Python type-stub constructor signature without changing the measured
  runtime path.

## Repository gate

The unmodified fast verifier ran outside the sandbox so the authorized
ptrace-dependent security test could execute. It passed 103/103 suites with
zero skipped or excluded. AC-036 observed no successful listen syscall; AC-037
observed no off-loopback egress; injection and AST surface checks passed.

Windows runtime was unavailable and is not claimed. The CUDA dense-eligibility
route, applicable combined Engine feature route, fresh wheel, and offline
npm/native package smokes are retained in the verification matrix.
