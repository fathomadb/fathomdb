# Slice 80.n AC-072 continuation campaign

Status: **STOPPED — ACCEPTANCE STREAK IMPOSSIBLE**

The owner-authorized continuation requires three consecutive, fresh-process
AC-072 observations that are both environment-valid and numerically passing.
The fixture, binary, runner and limits are unchanged: 10,000 rows, 384
dimensions, 1,000 warm-treatment queries, p50 <=80 ms, p99 <=300 ms, and zero
swap-I/O delta.

The required short smoke passed. C1 through C5 executed the full test and all
passed numerically. C1 and C4 are fully qualified. The two allowed
post-environment-invalidity collector-only readiness checks (after C2 and C3)
were qualified; they launched no benchmark. An earlier collector-only check
after qualified C1 is retained for provenance but does not consume either
post-invalidity cycle. The checks found no competitor and zero swap delta; they
do not convert later measurement-time swap activity into valid evidence.

| Record | p50 / p99 ms | Environment | Consequence |
| --- | ---: | --- | --- |
| Smoke | functional only | qualified | permitted acceptance dispatch |
| C1 | 70 / 79 | valid | first valid numeric pass; streak later broken by C2 |
| C2 | 70 / 77 | invalid: swap-in +2 | first recovery/readiness check |
| C3 | 70 / 77 | invalid: swap activity | second recovery/readiness check |
| C4 | 70 / 76 | valid | first valid numeric pass in a new streak |
| C5 | 70 / 76 | invalid: swap activity | breaks the streak; stop |
| C6 | not run | not observed | cannot yield three consecutive passes |

The raw logs and structured verdicts are retained in
`raw/ac072-slice80n-continuation-campaign/`; the smoke is retained in
`raw/ac072-slice80n-continuation-smoke/`. C2, C3 and C5 are numerically
passing but remain environment-invalid solely under the zero-swap rule. C1 and
C4 are the fully qualified continuation acceptance observations. No host setting
was changed, swap was not disabled, and no unrelated process was terminated.

This campaign consumed five fresh runner processes and five full measurements
(C1–C5), with two qualified numeric passes (C1 and C4), a maximum qualified
streak of one, and both permitted post-invalidity readiness cycles. The sixth
process is not dispatched because it cannot change the required
three-consecutive-pass outcome. Slice 80 therefore remains blocked on a new
owner disposition for valid AC-072 evidence; Slice 85 remains blocked.
