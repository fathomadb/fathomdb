# Preregistration: run to completion and the p90 start check

Written 2026-10-18, before any study run, on branch `exp/agent-access-claude`. The arms, metrics and
decision rules below are fixed; the results go into `REPORT.md` § 12 against them. One timing smoke run
(2 configurations, 4 simulations, to size the study) was made before this file was written; its numbers
are not used.

Prompt: `dev/plans/prompts/queue-run-to-completion-test-prompt.md`. Design of record:
`dev/design/agent-access/queue-admission.md` § 6.1 (owner decision 2026-10-18: the 10 s deadline bounds
waiting and starting; a started call runs to completion, up to `run_limit_ms` counted from the permit).

## Simulator rule

`Config.run_limit` (seconds). `None` is the old rule: the wait deadline also ends a running call, which
is abandoned and keeps its permit until the engine returns. When set, a started call is abandoned only
once it has held its permit for `run_limit`; it keeps its permit (and reader) until its read ends. With
`interrupt` (EP-09) the run limit cancels the call instead and frees the permit and the reader. A call
that completes after its wait deadline is an ok result and a service-time sample. Client cancel is not
modelled: Claude Code's MCP tool timeout is effectively none (~28 h, REPORT § 2), and agents do not
cancel a pending tool call.

## Arms

Every arm: queue depth 32, fixed 10 s wait deadline, cap from the configuration (4 or 7), 50 ms cold floor
before 8 samples of a tool kind, window 128.

| arm | start check | run limit | EP-09 |
| --- | --- | --- | --- |
| `old_p90` | p90 | none (deadline ends running calls) | no |
| `old_p90_ep09` | p90 | none (deadline interrupts running calls) | yes |
| `rtc_p90_rl{30,60,120}` | p90 of last 128 per kind | 30, 60, 120 s | no |
| `rtc_p95_rl{30,60,120}` | p95 | 30, 60, 120 s | no |
| `rtc_p99_rl{30,60,120}` | p99 | 30, 60, 120 s | no |
| `rtc_wait_rl{30,60,120}` | wait deadline only: a call that gets a permit before its deadline starts | 30, 60, 120 s | no |
| `rtc_{p90,wait}_ep09_rl{10,30,60}` | p90 or wait only | 10, 30, 60 s, interrupting | yes |

Each arm runs under reader dispatch P0 (round-robin, today) and P2 (idle reader first, decided as
EP-18 part A), and on two slow tails, all paired by configuration with common random numbers.

- **Bands.** Realistic: `fuzz.real_config(0..499)`, the 500 configurations of REPORT § 7 (K 1–8, slow
  reads at p 0.1–1 % log-uniform, GPU, closed loop). Stress: the main-fuzz configurations
  `fuzz.sample_config(0..1999)` with K ≥ 16 and slow p ≥ 2 %, the filter REPORT § 7 uses (580
  configurations).
- **Slow tails.** `base`: each configuration's own slow tail (median 0.2–8 s, σ 0.25–1.5).
  `long`: the same slow-read probability with read time lognormal, median log-uniform 10–30 s per
  configuration, σ 0.25, to stress the run limit.
- **Clients** as REPORT § 2: agents do not honour `retry_after_ms`; after a turn resolves they think
  (lognormal) and issue the next turn. A failure is counted as a retry the agent must make on its next
  turn; retries are not fed back as extra load (at realistic failure rates of ~0.1 % this is negligible;
  stated as a limit).

20 arms × 2 dispatch × 2 tails × 1,080 configurations = 86,400 simulations of 20,000 calls each.

## Metrics (per configuration, after a 2 % warm-up)

- Outcomes per call: `ok_rate`, `refuse_rate` (busy, by reason `rf_queue_full`, `rf_deadline`),
  `timeout_rate` (abandoned after start: at the deadline under `old`, at the run limit under `rtc`),
  `fail_rate` = busy + abandoned.
- Latency of successful calls: `ok_p50`, `ok_p95`, `ok_p99`, `ok_p999`.
- Late answers: `ok_late_rate` (per call) and `ok_late_share` (per success) answered after the 10 s wait
  deadline; `ok_late30_rate` after 30 s; `late_start_ok_rate` (late successes that waited for their
  permit); the histogram `late_h_*` over (10, 12], (12, 15], (15, 20], (20, 30], (30, 60], (60, 120],
  (120, ∞) s.
- `goodput_per_s`: successes per second over the span to the last resolution.
- Abandoned work: `ab_permit_s_per_kcall` (permit-seconds held by abandoned work per 1,000 calls),
  `wasted_permit_s_per_kcall` (whole permit holds of abandoned calls), `abandoned_avg`.
- Retries caused: `fail_rate` (each failure is one re-issued call) and `turn_failed` (turns with any
  failure; each costs the agent one more LLM turn).
- Time to data: `ttd_mean`, `ttd_p99`; a failed call costs its own time, plus one think-time median, plus
  one retry at the run's mean ok latency.
- Queue and head-of-line: `max_queue`, `rf_queue_full`, `to_hol`, `false_refusal_rate` (under `rtc`: the
  call would have got a permit before its wait deadline).

**Statistics.** Pooled mean over configurations of a band; paired differences per configuration with a
95 % bootstrap interval (2,000 resamples, seed 1). Latency percentiles are reported as the median over
configurations of the per-configuration value.

## Decision rules

**R1. Start check (question 2).** Deciding cell: realistic band, P2, base tail, run limit 60 s.
ΔF = `fail_rate(rtc_p90_rl60) − fail_rate(rtc_wait_rl60)`, paired.

- **Drop the p90 check** (any call that gets a permit before its wait deadline starts) if all hold:
  1. the 95 % interval of ΔF is not entirely below 0 (p90 does not fail measurably fewer calls);
  2. wait-only's `ttd_p99` is at most 10 % above p90's (median over configurations of the paired ratio);
  3. the late-answer guard: under wait-only, mean `ok_late_share` ≤ 1 % and mean `ok_late30_rate`
     ≤ 0.1 % in the realistic band, base and long tails.
- **Keep the p90 check** if the interval of ΔF is entirely below 0, or condition 2 fails.
- **Report to the owner without deciding** if condition 3 fails.
- P0 and the stress band do not change the decision. If either points the other way (the sign of the
  mean ΔF differs and its interval excludes 0), the report says so as a condition on the decision.
- p95 and p99 are descriptive unless p90 is kept; then the percentile with the lowest mean `fail_rate`
  among those meeting condition 2 replaces p90.

**R2. `run_limit_ms` (question 3).** With the R1 start check, P2, realistic band: the smallest run limit
in {30, 60, 120} s whose mean `timeout_rate` (run-limit hits) is ≤ 0.01 % of calls on the base tail and
≤ 0.05 % on the long tail. If none qualifies: 120 s, reported to the owner. Hit rates for every band,
tail and dispatch are reported.

**R3. Queue depth and head-of-line (question 4).** Run to completion changes the depth result if, for the
R1/R2 arm against `old_p90`: mean `rf_queue_full` differs by more than 0.1 points, or the share of
configurations whose `max_queue` reaches 32 rises by more than 1 point. It changes the head-of-line
result if the paired P0 − P2 excess in `fail_rate` differs from the old excess by more than 0.05 points,
or the P0 − P2 excess in `ok_p99` (median over configurations) differs by more than 25 %. Reported per
band and tail. A change triggers the depth rule of queue-admission.md § 7.3 only through the live sweep.

**R4. EP-09 (question 5).** With EP-09 and the R1 check, P2, realistic band: the smallest run limit in
{10, 30, 60} s whose mean `fail_rate` is within 0.01 points of the EP-09 60 s arm on both tails. Also
reported: abandoned permit-seconds (zero by construction), `fail_rate` and `ok_late_share` against the
same arm without EP-09.

**Question 1 (how many former late timeouts become successes, how late)** is descriptive: `old_p90`
timeouts and late-admit timeouts against `rtc_*_rl60` late successes, run-limit hits and the lateness
histogram, per band and tail.

## Gate

The simulator changes and these conclusions get a Fable-medium review; each fix lands RED first.

## Addendum, 2026-10-18 (after review, before run 2)

**What was seen.** Run 1 was stopped at 5,429 of 86,400 simulations (realistic band, roughly
configurations 0–67) after the Fable-medium review of the simulator. The reviewer probed 2,102 of those
rows; a smoke test of `rtc_analysis.py` read 1,360. Seen: the start-check lock-up below, and an early R1
direction ("drop"). The reviewer's report also gave run-1 arm means for about 42 realistic
configurations: R2's 30 s limit as marginal on the long tail (hits 0.049 % against the 0.05 % limit),
the collapse of p99, and the ordering of the EP-09 arms. No R1–R4 threshold was changed after that. Run 1 is discarded (`results/rtc-aborted-run1.*`, not committed); run 2 reruns
everything.

**Lock-up (review finding B1).** Under run to completion a late completion is a service-time sample (as
queue-admission.md § 6.1 specifies). One read slower than 10 s can lift the p90 of its tool kind above
the 10 s that any new call has. Every later call of that kind is then refused with reason `deadline`,
even with a permit free, so no new sample arrives and the window never recovers. With 8 samples, p90 is
the largest of them, so one slow read among the first eight is enough. This is a property of the design
as written (step 4 re-checks `deadline − now ≥ need` after any permit), not of the simulator.

**Changes for run 2.**

- New metric `rf_with_permit`: refused for time while a permit was free (at arrival or at grant).
- `ab_permit_s_per_kcall` now counts only the measured calls (after warm-up), from abandonment to the
  thread's return. `turn_failed` is undefined (NaN) for open-loop configurations, which have no turns.
- **Exploratory arms**, not part of the preregistered rules: `rtc_p90cap_rl{30,60,120}` and
  `rtc_p90cap_ep09_rl{10,30,60}`: p90 with `need` capped at 9.95 s, so a call that finds a free permit
  within 50 ms of arrival always starts and a late sample cannot lock its kind out. 26 arms × 2 × 2 ×
  1,080 = 112,320 simulations.
- **Reporting added** (exploratory): per cell, the number of configurations with `rf_with_permit` > 1 %
  for each percentile check (lock-ups); R1 recomputed without those configurations; R1's three conditions
  with `rtc_p90cap_rl60` in place of `rtc_p90_rl60` (R1x). R1 stays the decision. If R1 and R1x disagree,
  both go to the owner.
- **Rule clarifications** (review B2, N1): when R1 keeps a percentile check, the candidates are p90, p95
  and p99 in the deciding cell; one qualifies if its `ttd_p99` is at most 10 % above wait-only's (median
  paired ratio), and the qualifying one with the lowest mean `fail_rate` is chosen. R2 then uses that check's run-limit arms. EP-09 arms exist only for p90,
  wait-only and capped p90; if R1 picks p95 or p99, R4 uses p90's EP-09 arms and says so. R4's "within
  0.01 points" is one-sided: a shorter limit with a lower fail rate qualifies.

**After run 2 (second review, 2026-10-18).** The lock-up count used `rf_with_permit`, which also counts
grant-time sheds. A new metric, `rf_arrival_free`, records refusals at arrival with a permit free, and
the four percentile arms at 60 s were rerun to record it. The rerun used the same configurations, and
every `fail_rate` reproduced. A per-call pairing of the old rule's timeouts (`rtc_pairing.py`) replaces
the aggregate "how many become answers". Both are exploratory; R1–R4 are unchanged.
