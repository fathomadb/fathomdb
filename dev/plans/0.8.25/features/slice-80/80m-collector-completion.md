# Slice 80.m — collector completion

This is a named completion work package inside Slice 80, not a new slice.
It is authorized by the repository owner on 2026-09-11; see
[the authorization record](additional-campaign-authorization.md).

| Stage | Required work | Exit condition |
| --- | --- | --- |
| Collector repair | Focused RED/GREEN for AC-081's missing census-subshell exclusion. | Both shell runners exclude only their own census subshell while detecting a separate runner. |
| Readiness proof | Run both exact production collection paths in collector-only mode. | Valid identity/environment records; a positive competitor control is rejected; missing records fail closed. |
| Campaign execution | Qualify each raw log immediately. | The first invalid or failed timed cell stops all remaining timing. |
| Acceptance completion | Run seven fresh AC-081 processes and three AC-072 repetitions. | Every required numerical and environment check passes. |
| Closeout | Independent evidence review, write-receipt applicability check, state regeneration. | Slice 80 complete and `next_slice` is 85. |

## Active allowance

Earlier authorizations and observations remain historical records. This table is
the only active timing budget.

| Gate | Earlier history | Slice 80.m remaining | Rule |
| --- | --- | ---: | --- |
| AC-081 | Seven prior numerical passes, all qualification-invalid from the collector's own census shell. | 0; 7/7 Slice 80.m cells passed and qualified. | Complete. |
| AC-072 | Six prior numerical passes without three fully valid cells; the prior final series was not started. | 0; R1 was invalid from swap-in delta 2, and R2–R3 remain unstarted. | Stop rule exhausted the campaign. |

Collector-only tests and readiness proof are not timing observations. They are
bounded to one RED→GREEN correction and one review-driven amendment. No
environmental polling, product tuning, threshold change, broad regression, or
historical write rerun is part of this work package.

## Execution result

The collector-only records qualified for both runners. Their focused review
passed after the parser's exact prefix/identity binding correction. The final
AC-081 campaign passed seven of seven fresh qualified processes: median
172.883034 ms sequential and 52.527637 ms concurrent, with no warnings.

AC-072 R1 measured p50 69 ms and p99 75 ms but its start/end `pswpin` delta was
two. This is an environment-invalid acceptance observation under the unchanged
policy. The stop rule prevents R2, R3, replacement observations, and Slice 80
closure. No product change, broad regression, or historical write rerun ran.
