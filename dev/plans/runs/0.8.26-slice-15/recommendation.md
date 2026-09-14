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
carrier is independent of ranked-search payloads and commits locator, source
and artifact identities, lifecycle and dependency data, graph disclosure, and
edge endpoints/kind/direction.

| Arm | Median campaign p50 | Treatment/control ratio, median (bootstrap 95%) |
| --- | ---: | ---: |
| one result | 0.480 ms control; 0.786 ms treatment | 1.633 (1.625–1.644) |
| 50 results | 3.015 ms control; 4.632 ms treatment | 1.529 (1.517–1.536) |
| 8 callers, one result | 0.589 ms control; 0.956 ms treatment | 1.648 (1.578–1.760) |
| 8 callers, 50 results | 3.711 ms control; 5.596 ms treatment | 1.543 (1.242–1.581) |
| 10,000 work units, 50 results | 7.404 ms control; 8.916 ms treatment | 1.223 (1.200–1.235) |

At 50 results, A's expected mean latency under evidence-request fractions
`0, .01, .1, .5, 1` is respectively `3.052, 3.068, 3.213, 3.858, 4.665 ms`.
B is `4.665 ms` at every fraction. At `p=1`, both execute the same treatment.
This supports paying the complete provenance/security cost only when requested.

The treatment-on response is large under either carrier. One-result control/A/
B is `476/1,368/1,343` bytes; 50-result control/A/B is
`15,119/58,543/58,420` bytes. The sidecar framing premium is 25 bytes at one
result and 123 bytes at 50, so carrier bytes do not justify mandatory evidence.

Point resolution remains small in absolute terms. Across five campaigns, node
and edge 1 KiB p50 values were 0.306–0.333 ms; eight-caller point p50 was
0.307–0.318 ms. With a 102,400-byte canonical source, node p50 was
0.485–0.534 ms and edge p50 0.486–0.533 ms. A Memex-shaped batch of 25 node and
25 edge resolutions was 15.419–15.917 ms p50. The 100 KiB and 30-sample batch
p99 values are descriptive because they are undersampled.

Writer triggers did not fire. With hydrated graph load, median writer
throughput ratio was 0.999 (bootstrap 0.978–1.012) and p99 ratio 0.999
(0.937–1.069). With point-resolution load, throughput ratio was 0.983
(0.970–1.003) and p99 ratio 1.001 (0.937–1.026). Every loaded arm began after a
successful background commit; graph arms observed 3–4 and point arms one
successful operation during their short samples. Neither the greater-than-10%
throughput-loss nor greater-than-25% p99-growth threshold crossed.

Process-isolated peak-RSS growth was 248–312 KiB for control and 460–500 KiB
for treatment, with a median paired ratio of 1.855 (1.487–1.935). This is a
repeatable absolute increment of roughly 152–232 KiB at 50 results, but not a
hard-limit failure.

Twenty paired observations per erasure verb alternated idle/held and operation
order behind a before-primary-lock rendezvous. Erase p50 was 5.997 ms idle and
6.179 ms held; excise was 5.985 and 6.142 ms. All 40 held outcomes completed
only after resolver release. A separate held WAL reader produced the existing
typed `ErasureIncomplete(stage="wal_checkpoint")` outcome, and the test proved
the committed erasure remained observable afterward.

The 50-result preflight returned 50 node and 50 edge rows through indexed
plans, and hashed the shared 22-byte source once. Corrupt hash/locator,
incomplete provenance, current context, foreign/mismatched context, tamper,
restart, post-erasure unavailability, class-aware edge eligibility, and
all-or-nothing refusal passed.

The existing read sentinels remained green. AC-081a/b observed 174.388 ms
sequential and 38.754 ms concurrent for 1,600 searches in each arm, with no
warning or failure flag. AC-081c proved a second reader progressed while
another held a snapshot. Its larger attribution campaign was not required.

## Why not B

B imposes the 1.53× single-caller and 1.54× concurrent 50-result work on every
graph call, grows every response, and either rejects existing current-context
calls or creates mixed required-field semantics. A keeps one V1 family and one
treatment implementation without those ordinary-path consequences. The
43-file maintained graph surface still needs coordinated Slice 20 review, but
A preserves the established target encoding when evidence is not requested.

The real samples and reproducible derivation are in `raw-samples.json`,
`derived-analysis.json`, and `derive.jq` in this directory.
