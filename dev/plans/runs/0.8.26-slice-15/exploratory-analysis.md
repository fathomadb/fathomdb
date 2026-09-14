# Slice 15 exploratory result — not D26-01 evidence

## Observations

| Arm | n | p50 µs | p95 µs | p99 µs | mean µs |
| --- | ---: | ---: | ---: | ---: | ---: |
| ordinary graph, one target | 1,000 | 498 | 600 | 668 | 500.30 |
| shortcut hydrated graph, one target | 1,000 | 952 | 1,110 | 1,207 | 963.28 |
| point node resolution | 1,000 | 255 | 274 | 304 | 259.91 |
| point edge resolution | 1,000 | 255 | 272 | 303 | 259.28 |

The shortcut treatment added 92.5% at p50. This is not an estimate of the
approved treatment: it rediscovers the terminal edge after selection and uses
the ranked-evidence per-artifact loader rather than two bounded joined
preflight statements with source deduplication. The result is therefore useful
only as an upper-direction exploratory signal.

The one-target ordinary response was 476 bytes. Stable identities plus two
opaque references added 1,347 bytes in the prototype carrier. For this fixture,
the option-A byte model `476 + p × 1,347` yields 476, 489.47, 610.70, 1,149.50,
and 1,823 bytes at `p = 0, .01, .1, .5, 1`. Option B is 1,823 bytes at every
request fraction. A 50-target linear upper illustration adds 67,350 bytes, but
must not be mistaken for a measured 50-target response.

The actual long-run AC-081a/b observation passed outside its warning band:
171,984,086 ns sequential, 46,238,078 ns concurrent, ratio 3.719534, 1,600
searches per arm and eight concurrent threads. AC-081c's real-database
independent-reader sentinel also passed.

## Stop disposition

The approved measurement matrix was not run. Before decision measurement, the
prototype still needs retained winning-edge cursor flow, exactly two bounded
class-specific joined preflights, full provenance validation with source
deduplication, query-plan proof, graph-operation commitment, restart and
nondisclosure coverage, corrupt/incomplete atomic refusal, deterministic
erasure ordering, held-WAL behavior, 1/50/10,000-work fixtures, concurrent
calls, writer campaigns, isolated RSS, and paired erase/excise campaigns.

No recommendation to rule D26-01 is supported by this exploratory run. The raw
sample SHA-256 is
`7d5ce2fa7c9311b4359521181ddfb1d371f633a72d512d623377933264a6127d`.
