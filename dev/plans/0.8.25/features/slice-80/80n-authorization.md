# Slice 80.n execution authorization

Recorded: 2026-09-11

The repository owner commissioned the focused Slice 80.n implementation and
acceptance-completion path. This authorization supersedes the stopped 80.m
AC-072 allowance only for the following bounded work:

| Work | Allowance | Stop rule |
| --- | ---: | --- |
| AC-081 | 0 | Reuse the accepted 80.m receipt when protected inputs match. |
| AC-072 smoke | One non-acceptance 10-row/384d warm run | A functional or qualification failure blocks longer timing until an identified harness repair is proved. |
| AC-072 acceptance | One three-cell, fresh-process 10k/384d/1,000-query warm campaign | Validate each cell before the next; the first numeric failure, invalid environment, timeout, or malformed receipt stops remaining timing. |
| Protected writes | 0 | Reuse Slice 79 receipts unless a product/build input changes. |
| Broad verification | 0 | Reserved for Slice 85. |

The authorization includes ordinary focused runner, parser, collector, record,
and review corrections needed to execute this route. It does not authorize
product optimization, threshold changes, additional timing campaigns, host
configuration changes, disabling swap, terminating unrelated processes,
publication, push, or Slice 85 verification.

Existing observations remain immutable historical evidence. The smoke is
explicitly non-acceptance and cannot satisfy any AC-072 campaign cell.

## Continuation authorization

Recorded: 2026-09-11. The owner directed: "Fix it." For the unresolved
environment-only AC-072 blocker, this authorizes one fresh three-cell campaign
after the documented dispatcher repair and read-only quiet-environment check.
It keeps the exact 10k/384d/1,000-query warm fixture, thresholds, collector,
and per-cell stop rule: any invalid, failed, timed-out, or malformed cell stops
the new campaign immediately. It does not authorize host setting changes,
stopping the Windows VM, disabling swap, terminating unrelated processes,
threshold changes, further replacement campaigns, broad verification, or
publication.

## Superseding continuation authorization

Recorded: 2026-09-11. This supersedes the preceding continuation allowance.
The owner authorizes up to six fresh-process AC-072 observations in one
continuation campaign. Completion requires three consecutive observations that
are both environment-valid and numerically passing. Stop when that streak is
obtained or the six-observation cap is consumed; every observation remains
retained and no historical result may substitute for one.

Before acceptance, run one short non-acceptance smoke through the sealed path.
For an environment-invalid observation, pause dispatch, inspect its controls,
and use at most two bounded read-only diagnosis/readiness cycles across this
campaign before deciding whether to continue. A valid-environment numerical
failure stops for investigation. Existing thresholds, fixture controls, zero
swap-I/O rule, accepted AC-081/write receipts, and focused-only scope remain
unchanged. No host-policy change, swap disabling, unrelated-process termination,
or unlimited retrying is authorized.
