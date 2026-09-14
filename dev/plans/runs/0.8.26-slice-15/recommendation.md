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

The complete post-selection treatment is materially more expensive than graph
traversal alone, especially at the maximum 50-result carrier:

| Arm | Median campaign p50 | Treatment/control ratio, median (bootstrap 95%) |
| --- | ---: | ---: |
| one result | 0.479 ms control; 0.841 ms treatment | 1.797 (1.726–1.820) |
| 50 results | 3.024 ms control; 13.948 ms treatment | 4.640 (4.503–4.672) |
| 8 callers, one result | see derived JSON | 1.898 (1.868–1.955) |
| 8 callers, 50 results | 3.508 ms control; 22.829 ms treatment | 6.511 (6.349–6.592) |
| 10,000 work units, 50 results | 7.303 ms control; 19.610 ms treatment | 2.696 (2.678–2.711) |

At 50 results, A's expected p50 under evidence-request fractions
`0, .01, .1, .5, 1` is respectively `3.024, 3.133, 4.116, 8.486, 13.948 ms`.
B is `13.948 ms` at every fraction. At `p=1`, both execute the same treatment.
This supports paying the complete provenance/security cost only when requested.

The treatment-on response is large under either carrier: one-result control/A/
B is `476/2,222/2,197` bytes; 50-result control/A/B is
`15,119/101,243/101,120` bytes. The sidecar framing premium is only 25 bytes at
one result and 123 bytes at 50, so carrier bytes do not justify making evidence
mandatory.

Point resolution remains small in absolute terms. Across five campaigns, node
and edge 1 KiB p50 values were 0.262–0.265 ms; eight-caller point p50 was
0.265–0.271 ms. With a 102,400-byte canonical source, node p50 was
0.383–0.392 ms and edge p50 0.382–0.389 ms. A Memex-shaped batch of 25 node and
25 edge resolutions was 13.357–13.816 ms p50. The 100 KiB and 30-sample batch
p99 values are descriptive because they are undersampled.

Writer triggers did not fire. With hydrated graph load, median writer
throughput ratio was 1.007 (bootstrap 0.960–1.046) and p99 ratio 0.995
(0.789–1.003). With point-resolution load, throughput ratio was 1.030
(0.990–1.044) and p99 ratio 1.001 (0.807–1.041). Neither the greater-than-10%
throughput-loss nor greater-than-25% p99-growth threshold crossed, and four of
five campaigns did not agree on a harmful direction.

Process-isolated peak-RSS growth was 248–360 KiB for control and 440–488 KiB
for treatment, with a median paired ratio of 1.774 (1.311–1.968). This is a
repeatable absolute increment of roughly 112–240 KiB at 50 results, but not a
hard-limit failure.

Twenty paired observations per erasure verb preserved ordering. Erase p50 was
3.978 ms idle and 4.102 ms with a resolver parked after materialization;
excise was 3.939 and 4.100 ms. All 40 held outcomes completed only after the
resolver released. A separately held WAL reader produced the existing typed
`ErasureIncomplete(stage="wal_checkpoint")` outcome.

The 50-result preflight executed exactly two class-specific statements. Each
received and returned 50 rows, all access paths were indexed, and the shared
22-byte canonical source was hashed once across both classes. Corrupt hash,
invalid locator, incomplete provenance, current context, foreign/mismatched
context, tamper, restart, post-erasure unavailability, class-aware edge
eligibility, and all-or-nothing refusal passed.

The existing read sentinels also remained green. AC-081a/b observed
176.939 ms sequential and 44.154 ms concurrent for 1,600 searches in each arm,
with neither warning nor failure flag set. AC-081c proved a second reader made
progress while another held a snapshot. The AC-081 warning band therefore did
not require its larger attribution campaign.

## Why not B

B imposes the 4.64× single-caller and 6.51× concurrent 50-result work on every
graph call, grows every response, and either rejects existing current-context
calls or creates mixed required-field semantics. A keeps one V1 family and one
treatment implementation without those ordinary-path consequences. The 39-
file maintained graph surface still needs coordinated Slice 20 review, but A
preserves the established target encoding when evidence is not requested.

The detailed samples and reproducible derivation are in `raw-samples.json`,
`derived-analysis.json`, and `derive.jq` in this directory.
