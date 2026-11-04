---
title: Slice 84b — certification work and transactional totals
status: COMPLETE
date: 2026-11-03
---

# Slice 84b status

Both targets were implemented and passed focused correctness/work tests.
**Integrated service qualification failed.** Astra recommended rejection; baseline
production code is restored. The candidate and its tests remain at `1e9c6d584`.
Verification is complete through the scoped checks and resumed workspace stages below.

The [plan](plan.md) and [design](design.md) reconcile retained Slice 84 source
`77e2de796`, enumerate actual mutation effects and preserve its negative result.
No scheduling, public API, schema format or physical-success contract changed.
The candidate primary-connection cleanup-job certificate skipped repeated structural proof on
proved plain writes. Actual SQLite class/cursor rows and exact retention deletes
supplied transactional envelope deltas. Unknown drift reconstructed authority;
COMMIT failure discarded it; finalization always used the original full proof.

RED initially failed on missing totals/tokens. GREEN implements both targets.
Focused tests compare real writes with full totals/zero proof, generated retained
history, same-batch supersession, unnamed/NULL edges, configured property FTS,
Unicode/NUL, opposite-class cursors, maximum deletion, retention across the cap,
all logical cap boundaries, actual COMMIT failure and the external COMMIT/cache
publication race. Separate warm counters show zero full-envelope reconstructions
and zero repeated zero proofs; changed-row accounting remains active. Small-store
VM work was 7411 cold versus 6725 warm; this is not a latency qualification.

Astra code findings (exact class/cursor accounting and optional reconstruction
fallback) are fixed. The screening controller was tested RED/GREEN and its
review finding (idle recovery cannot count as initial public-call success) fixed.
No new branch/worktree was created. Existing development checkout is the sole
writer; final landing and release records follow verification.

## Acceptance and service screen

| Requirement | Disposition |
| --- | --- |
| B1 exact admission | Focused candidate and original guard/safety suites passed independently; baseline restored. |
| B2 less repeated proof | PASS in isolated warm commits: zero repeated full zero proofs. No warm benefit in measured service. |
| B3 transactional envelope | PASS for modeled commits: exact indexed class/cursor row bytes, retention deltas, NULL/history/property and threshold tests agree with full reference. |
| B4 authority/recovery | Scoped token/scope/generation/lost-cache, same-connection bypass, real COMMIT failure and external-publication-race tests passed. Candidate not shipped; original reopen/crash guards retained. |
| B5 physical/service safety | Independent candidate physical/maintenance and Slice 82/83 safety passed; all eight timed cells verified physical/survivor witnesses without native error/retry. |
| B6 integrated benefit | NOT QUALIFIED: edge screen failed. Node passed only one pair; property, later pairs and idle were not run. No three-pair medians or p99 qualification. |
| B7 truthful outcome | All 42 dispositions retained: eight executed, 34 explicitly not run after screen failure. No replacements or relaxed gates. |

| Family | Baseline erase (s) | Candidate (s) | Read completions | Write completions |
| --- | ---: | ---: | --- | --- |
| Node | 7.632 | 7.245 | 1471 → 1471 | 771 → 808 |
| Edge | 3.223 | 3.408 | 1487 → 1486 | 1198 → 1181 |

All four matched controls completed 1500 reads and 1500 writes. All 24,000
scheduled offers reconcile to 21,873 issued/completed and 2,127 generator misses;
Engine refusals, errors, unfinished, unknown and accounting errors were zero.
The runner took 150.383 seconds; adding a conservative five-second freeze/setup
charge gives 155.383 seconds of the 1800-second allowance. Peak sampled RSS was
484,872,192 bytes and owned scratch 454,364,991 bytes, within the 8/12-GiB caps.
Raw events, 42 dispositions, frozen binaries/inputs and source identities remain
in `/tmp/fathomdb-slice84b/`; no large generated artifacts are tracked.

The mechanism did not stay warm in service: node reconstructed totals 33 times
for 33 modeled commits, edge 21 for 21. Exact invalidation causes were not
measured. This explains why the isolated work reduction does not establish a
service improvement, without attributing the invalidations to an unmeasured path.
Astra reviewed and approved rejection, preserving original safety oracles.

## Final verification and landing

Retained production is byte-identical to `77e2de796`; rejection is `426d22782`.
Final retained-tree lint passed. The remaining official heavy test tier passed
all seven registered suites: Rust workspace, Python, Python native receipt,
TypeScript, hermetic native binding build, test-seam boundary and the Tegra
wrapper. The Tegra wrapper's hardware-specific exercise skips on this Linux
x86_64 host; it is not a Tegra qualification. Normal network-model exclusions
remained, and one Slice 135 arm needs an explicitly supplied consumer binary.
Eight remaining fast registrations
and four controller tests also passed independently. Prior completed pre-Rust
harness checks and unchanged typecheck/strict-security evidence are reused.
This is assembled verification with scoped resumption, not one uninterrupted
full-gate pass. Logs: `/tmp/slice84b-final-{verify,heavy}.log`.

Both bounded investigations (84 and 84b) are complete with negative qualification.
Baseline production is retained. No new branch/worktree was created; the existing
development line is merged into `release/0.8.28`. Slice 90 and the open
guarantee/API decision remain separate. Further optimization needs a separately
reviewed scope; no replacement campaign is implied.
