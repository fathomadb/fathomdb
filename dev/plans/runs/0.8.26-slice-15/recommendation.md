# Slice 15 result and D26-01 recommendation

## Recommendation

Select **D26-01 option A: an opt-in first-generation V1 graph-evidence
sidecar**. Keep ordinary graph expansion on the literal existing byte path;
require a frozen context when the sidecar is requested; return stable target
and winning terminal-edge revision identities only alongside authenticated,
graph-disclosure-bound references; resolve intrinsic evidence without a raw-ID
route.

This is a recommendation for HITL, not a ruling. Slice 15 does not unblock
Slice 20 until the HITL records D26-01.

## Decision evidence

The corrected treatment performs two class-specific joined data preflights
after selection. It validates and deduplicates complete provenance material,
hashes the shared source once, and mints from that material with no additional
SQL or hashing. Plan inspection is outside the timed path. The intrinsic
carrier is independent of ranked-search payloads and commits the canonical
source locator and span, complete source and artifact identities, the direct
dependency including generation, class-specific lifecycle, graph disclosure,
and edge endpoints/kind/direction.

| Arm | Median campaign p50 | Treatment/control ratio, median (bootstrap 95%) |
| --- | ---: | ---: |
| one result | 0.481 ms control; 0.781 ms treatment | 1.627 (1.596–1.656) |
| 50 results | 3.010 ms control; 4.612 ms treatment | 1.525 (1.516–1.543) |
| 8 callers, one result | 0.609 ms control; 0.953 ms treatment | 1.596 (1.524–1.744) |
| 8 callers, 50 results | 3.560 ms control; 5.478 ms treatment | 1.549 (1.512–1.577) |
| 10,000 work units, 50 results | 7.278 ms control; 8.835 ms treatment | 1.228 (1.195–1.242) |

At 50 results, A's expected mean latency under evidence-request fractions
`0, .01, .1, .5, 1` is respectively `3.040, 3.056, 3.203, 3.855, 4.670 ms`.
B is `4.670 ms` at every fraction. At `p=1`, both execute the same treatment.
This supports paying the complete provenance/security cost only when requested.

The treatment-on response is large under either carrier. One-result control/A/
B is `476/1,368/1,343` bytes; 50-result control/A/B is
`15,119/58,543/58,420` bytes. The sidecar framing premium is 25 bytes at one
result and 123 bytes at 50, so carrier bytes do not justify mandatory evidence.

Point resolution remains small in absolute terms. Across five campaigns, node
and edge 1 KiB p50 values were 0.306–0.341 ms; eight-caller point p50 was
0.308–0.319 ms. With a 102,400-byte canonical source, node p50 was
0.484–0.537 ms and edge p50 0.484–0.535 ms. A Memex-shaped batch of 25 node and
25 edge resolutions was 15.305–17.921 ms p50. The 100 KiB and 30-sample batch
p99 values are descriptive because they are undersampled.

The writer-throughput reconsideration trigger fired after FIX-2 corrected the
harness to require repeated overlap. With hydrated graph load, the median
writer throughput ratio was 0.781 (bootstrap 0.618–0.963), a 21.9 percent loss;
with point-resolution load it was 0.661 (0.598–0.767), a 33.9 percent loss.
Graph arms recorded 110–187 successful timed-interval writes and point arms 20;
setup and shutdown successes are excluded. Writer p99 did not grow: median
ratios were 0.542 (0.438–1.046) and 0.781 (0.693–1.013), respectively. The
throughput result is repeatable enough to require Slice 20 design
reconsideration, not to reject A: opt-in treatment limits the observed
interference to calls that actually request evidence, whereas B imposes it on
every graph expansion.

Process-isolated peak-RSS growth was 280–300 KiB for control and 464–560 KiB
for treatment, with a median paired ratio of 1.676 (1.547–2.000). This is a
repeatable absolute increment of roughly 164–280 KiB at 50 results, but not a
hard-limit failure.

Twenty paired observations per erasure verb alternated idle/held and operation
order behind a before-primary-lock rendezvous. Erase p50 was 5.563 ms idle and
5.895 ms held; excise was 5.549 and 5.850 ms. All 40 held outcomes completed
only after resolver release. A separate held WAL reader produced the existing
typed `ErasureIncomplete(stage="wal_checkpoint")` outcome, and the test proved
the committed erasure remained observable afterward.

The 50-result preflight returned 50 node and 50 edge rows through indexed
plans, and hashed the shared 22-byte source once. Corrupt hash/locator,
incomplete provenance, current context, foreign/mismatched context, tamper,
restart, post-erasure unavailability, class-aware edge eligibility, and
all-or-nothing refusal passed.

The existing read sentinels remained green. AC-081a/b observed 167.389 ms
sequential and 36.891 ms concurrent for 1,600 searches in each arm, with no
warning or failure flag. AC-081c proved a second reader progressed while
another held a snapshot. Its larger attribution campaign was not required.

## Why not B

B imposes the 1.53× single-caller and 1.55× concurrent 50-result work on every
graph call, grows every response, and either rejects existing current-context
calls or creates mixed required-field semantics. A keeps one V1 family and one
treatment implementation without those ordinary-path consequences. The
writer-throughput trigger means Slice 20 must reduce or explicitly bound
interference before shipping either shape; it is additional evidence against
making treatment mandatory. The 43-file maintained graph surface still needs
coordinated Slice 20 review, but A preserves the established target encoding
when evidence is not requested.

The real samples and reproducible derivation are in `raw-samples.json`,
`derived-analysis.json`, and `derive.jq` in this directory.
