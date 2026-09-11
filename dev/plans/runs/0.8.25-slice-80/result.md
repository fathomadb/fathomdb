# Slice 80 result

## Outcome

Implementation is complete, but Slice 80 cannot close because required
environment qualification is incomplete. The prior bounded replacement budget
is historical. Slice 80.m authorizes exactly one new collector-repaired campaign
per gate; no Slice 80.m timing cell has started yet. Release state remains on
Slice 80.

AC-020 is retired under seq-277; its historical failures remain failures.
AC-081a/b/c replace it without changing the search fixture or shipping product.

## AC-081a/b

The final build-once release executable is bound in `manifest.json`. Seven
fresh direct processes all passed numerically with no warnings. Their recorded
start/end controls pass, but the then-current competitor scanner missed Linux
truncated and hash-suffixed performance-binary names. The corrected scanner at
`eab4c2b0` cannot reconstruct the earlier process census, so environment
applicability is unproved rather than valid.

| Run | Sequential ms | Concurrent ms | Ratio, descriptive |
| --- | ---: | ---: | ---: |
| R1 | 184.868446 | 56.478269 | 3.273267 |
| R2 | 176.546395 | 41.336902 | 4.270915 |
| R3 | 179.911464 | 67.162199 | 2.678761 |
| R4 | 171.986776 | 66.224173 | 2.597039 |
| R5 | 184.352509 | 46.410745 | 3.972195 |
| R6 | 167.485947 | 57.370858 | 2.919356 |
| R7 | 174.591925 | 61.982696 | 2.816785 |

Medians are 176.546395 ms sequential and 57.370858 ms concurrent. Every
sequential result is below the 200 ms warning and 500 ms inclusive limit; every
concurrent result is below the 80 ms warning and 100 ms inclusive limit.
Sequential range is 167.485947–184.868446 ms with 4.559619 ms median absolute
deviation. Concurrent range is 41.336902–67.162199 ms with 8.853315 ms median
absolute deviation.

The first numerically passing series is preserved under `raw/ac081-invalid-quota`:
the original collector did not walk to the delegated parent `cpu.max`. A second
pre-review-fix passing series is preserved under `raw/ac081-pre-review-fix`.
Neither is substituted for the final artifact receipt.

## Slice 80.m collector completion

The prior authorized-final AC-081 series remains preserved as strong numerical
evidence with a qualification defect. Its 172.733800 ms sequential and
62.389734 ms concurrent medians are below both warning thresholds; it does not
demonstrate an AC-081 performance defect. It is not retrospectively relabeled
valid.

Focused RED/GREEN repair adds AC-081's missing PID-specific census-subshell
exclusion, aligns AC-072's complete environment record, and adds an exact
collector-only readiness path for both runners. The independently reviewed
readiness proof validates normal identity/environment records, rejects a live
separate runner-shaped competitor, and fails closed for malformed or incomplete
records. The sealed executable and relevant product-input hash are unchanged.
The active allowance and final collector identities are in the execution
manifest. No benchmark was launched by this repair.

## Owner-authorized final attempt

The owner authorized one additional seven-process AC-081 campaign and one
additional three-cell AC-072 campaign. The sealed AC-081 candidate at
`e0b2a14b2fcc2bcf88a2103ad9ce0ae75973eef6` completed all seven fresh
processes once. Every cell is numerically below both warning thresholds:

An earlier malformed expanded source SHA was rejected by the runner before a
raw log, environment snapshot, or test process was created; it was not a cell.

| Run | Sequential ms | Concurrent ms | Qualification |
| --- | ---: | ---: | --- |
| R1 | 170.620864 | 62.428466 | invalid: own census shell |
| R2 | 172.733800 | 41.579400 | invalid: own census shell |
| R3 | 173.352778 | 65.812139 | invalid: own census shell |
| R4 | 173.132814 | 42.363298 | invalid: own census shell |
| R5 | 178.120022 | 62.389734 | invalid: own census shell |
| R6 | 172.525128 | 65.803744 | invalid: own census shell |
| R7 | 171.988307 | 39.106786 | invalid: own census shell |

The corrected scanner truthfully recorded the active AC-081 runner's own
command-substitution shell because that runner lacks the AC-072 `BASHPID`
exclusion. This is a collector defect, not evidence of a competing workload.
Nevertheless, the retained policy makes every cell invalid. The authorization
requires stopping after any invalid campaign, so AC-072 was not dispatched and
no campaign was repeated or replaced.

## AC-081c and focused checks

- Four full-precision oracle tests pass, including every warning/hard boundary,
  one-nanosecond violations, independent arms, both failures and ratio
  irrelevance.
- The real-database same-Engine test proves worker 1 completes while worker 0
  holds a live snapshot. Its test control releases on send, disconnect or a
  five-second worker timeout; the caller uses bounded receives.
- Fifteen evidence-tool tests pass for identity, positive execution/counts,
  environment applicability, campaign non-vacuity, warnings, binary selection,
  raw-log parsing and competing-process recognition.
- The compiled-list check finds one active AC-081 test and confirms both AC-020
  registrations are ignored under ordinary `AGENT_LONG=1` execution.

## AC-072

All six exact 10k/384d/1,000-query cells pass numerically:

| Run | p50 ms | p99 ms | Swap-in delta | Applicability |
| --- | ---: | ---: | ---: | --- |
| R1 | 73 | 81 | 1 | invalid |
| R2 | 73 | 80 | 0 | competitor census unproved |
| R3 | 72 | 80 | 4 | invalid |
| R4 | 70 | 77 | 167 | invalid |
| R5 | 70 | 77 | 5 | invalid |
| R6 | 69 | 76 | 2 | invalid |

No test failed and no recorded load, memory or thermal limit failed. The
retained policy requires zero machine-wide swap I/O, so five cells cannot count.
R2 passes the recorded controls, but its old competitor scanner cannot establish
complete exclusion. No cell is fully proved environment-valid. No swap setting
was changed and no unrelated process was terminated.

## Applicability and scope

Slice 79's six write receipts remain applicable: Slice 80 changed contracts,
test harnesses, tooling, documents and debug-only synchronization—not shipping
write/search/SQLite behavior. The AC-072 exact operation and release product path
were unchanged by post-run review fixes, so the six logs remain honest numerical
candidate evidence, but none is complete acceptance evidence.

No broad regression, CI, platform/package campaign or optimization sweep ran.
Slice 85 has not started.
