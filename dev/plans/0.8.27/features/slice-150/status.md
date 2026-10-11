---
title: FathomDB 0.8.27 Slice 150 — qualification status
status: BLOCKED
target_release: 0.8.27
planning_baseline: 3a69eef82e0381641e5bfbf7dcd8ed25141d97ff
---

# Slice 150 qualification status

Planning began on 2026-10-10 in an isolated worktree from local
`release/0.8.27` at `3a69eef82`. The initial fetch showed the remote branch
behind the local release branch. At that planning baseline, release state
recorded Slice 140 as `PLANNED`. The owner required explicit confirmation of
its completion before execution; that confirmation arrived later the same day.

The [plan](plan.md) reconciles work since the original 2026-09-21 draft. The
[design](design.md) specifies exact-candidate receipts, gates, artifact
provenance, security and retention. The first `gpt-6.1-sol` high design review
returned **APPROVE WITH FIXES**; the assigned Rust SDK hidden probe and exact
retirement documentation targets were added. The `gpt-6-sol` high follow-up
accepted those corrections and requested exact final security sequencing. The
plan now requires a second complete grant/check/revert cycle and input-tree
identity proof after retirement. The subsequent `gpt-6-sol` high confirmation
returned **APPROVE** with no remaining design finding. Slice 132 has
already delivered the dedicated Rust SDK peer of Python and TypeScript;
Slice 150 verifies its parity and packages rather than implementing it again.

## Execution checkpoint — 2026-10-10

The owner confirmed Slice 140 closed. Fetched `release/0.8.27` at
`1f20b1d1badca9d9e142901ab8cb1e4ea117bd91`; release state records Slice
140 complete at `3a69eef82e0381641e5bfbf7dcd8ed25141d97ff` and advances
to Slice 150. The isolated branch merged that closeout and passed
`scripts/preflight.sh --expect-closed 140` with a clean source checkout.

Slice 132's assigned SDK probe has genuine RED `d80f43bcc` and GREEN
`c3239879e`; all fast hidden-surface self-tests pass, Rust SDK parity is
44/44, and its 28 checker tests pass. The independent `gpt-6-sol` high
[code review](code-review.md) approves that bounded change. The exact-source
[public and hidden captures](surface-comparison.md) have been independently
compared; only the intended SDK hidden row and its equivalent test-row identity
change are new. The [retention survey](retention-survey.md) verifies the
relocated 81,701,383-byte Slice 90 checkpoint set and records zero objects
above 100 decimal GB in each required scope. The completed Slice 140
checkout's ignored Cargo target cache was cleared to meet the public
comparator's 100 GB disk preflight.

A subsequent Slice 140 board-only handoff commit advanced `release/0.8.27`.
It was merged into this branch at `04a024ed1`; source, scripts, tools,
workflows and package inputs are byte-identical to the capture commit. The
release preflight and its 11-test regression pass. A full verifier run at
`04a024ed1` passed lint, typecheck, live security and Rust workspace tests,
then encountered the Python test harness's clean-worktree requirement while
this status and evidence were uncommitted. That interrupted run is not a full
gate pass. The records are being committed before an unaffected full rerun.

The unconfined strict security route produced both live AC-037 pass lines and
`0 violation(s), 0 blocker(s), 0 downgrade(s)`: the network-safety test
passed. The locked executor paragraph in `dev/acceptance.md` and its runbook
also prescribe a temporary AppArmor grant/revert on Ubuntu 24.04, assuming
the initial namespace probe fails. On this host `unshare -rUn true` already
succeeds, so the prescribed before/after denial cannot be witnessed. This is
a mismatch between the host procedure and this host, not a failed security
test. No grant was attempted; the owner has been asked for a restricted host
or an explicit ruling on the already-capable case.

No release PASS, publication, or Slice 150 closeout is claimed here.

## Qualification checkpoint — 2026-10-10

The clean committed candidate `c3268281643d4dcedb005971256293c0378a768f`
has tree `a03b92e0f8bf5699912f472de1ddf50d971debd9`. The full
`agent-verify` run passed 196/196 suites with no failed, skipped or excluded
suites at `a5f7dd833`; its log SHA-256 is
`504f4a0b2021d73506ab70fc4937c84693f3cad1f39617eeb95beb0d13c579d6`.
That commit's product source is byte-identical to `c32682816`, but the latter
changes README CUDA guidance and removes three stale feature-gate ignore
entries. The full verifier has not been repeated at `c32682816`; the scoped
documentation, coverage and affected Slice 72 checks passed. A separate
`c32682816` feature-complete run passed 20 runs: 141 planned, 136 passed,
zero failed and five documented ignores. Its summary SHA-256 is
`f50fa3a2983496abc53f5e9fcdbce9553cc333f97833095e2c254ed7fdf81cd8`.
The earlier failing feature-gate run remains an invalid receipt.

The Python wheel built from `a5f7dd833` has SHA-256
`f480254b2c5f965d3ee255186d995e6cb612b62b3be84207eaaad1a4c008ea42`;
the installed native import and real-database smoke passed, as did 18 focused
native harness tests. The frozen Memex oracle was unchanged and passed 1/1
against that wheel using a byte-identical historical Memex fixture; current
Memex HEAD has since removed the oracle's `MemoryRuntime` import. Its host
receipt is `qualification-evidence/slice-150/memex-consumer-receipt.json`.

The candidate-bound Rust source archive SHA-256 is
`57601d902fafc39ce9cdcca957ad6646b3cdb7932e82c44ccd96efecb3941d66`.
An external default-feature SDK consumer resolved Fathom sibling crates and
the active SQLite patch from that archive and passed write, read, source
erasure, control preservation and reopen against a real database. The CUDA
patch was unused in that graph. This is source-archive proof, not crates.io
package proof. Its manifest, lockfile, test and log are retained with
`qualification-evidence/slice-150/rust-source-consumer-receipt.json`.

Fresh Node production and test-hook packages from `c32682816` passed clean
external installed-consumer checks. An additional production-package smoke
passed write, `read.get`, `eraseSource`, control preservation and reopen. The
main and production native tarball SHA-256 values are respectively
`fb0ccc8ab1feaec4168226ba169c9b0fb938a2ab87ef551cddbe915b25d5795b`
and `a0ba076fba7a55caba42ab77e9fccd320b0ce88d3f9c26de7ddc2cd9347b84d6`.
The witness tarball SHA-256 is
`6dd6545e392d3d77cdb1b75161bbbab333ee918779ab19c20ec2fc07d8b5abdb`.
Independent Sol verification approved archive, wheel, Memex and Node consumer
evidence with explicit CPU/Linux x86_64 and prepublication limits. Terra was
unavailable in this session; the owner authorized a Sol subagent instead.

The public and hidden surface comparisons and 44/44 SDK parity check pass as
recorded in [surface-comparison.md](surface-comparison.md). Retention survey
findings remain in [retention-survey.md](retention-survey.md). No Windows,
Jetson, registry-installed Rust or installed CUDA package pass is claimed.
Historical Slice 135 performance measurements remain bound to their earlier
source and do not establish final-candidate latency equivalence.

**Remaining decisions and work:** the written AC-037 executor procedure needs
an owner ruling for an already-capable host or a restricted-host witness. The
final exact-candidate full verifier and post-retirement security cycle also
remain outstanding. The temporary hidden oracle is intact; release state
stays on Slice 150, and this worktree is retained for completion.
