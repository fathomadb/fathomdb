---
title: 0.8.30 Slice 40 study record
status: ACTIVE
slice: 40
target_release: 0.8.30
---

# Slice 40 study record

## Current CPU SDK baseline

The [prospective design](design.md) passed Astra-medium after FIX-1 and was
committed at `7e16195d1579cc27be14828d31c13351fdfe51f6`. The native baseline
source was `8972d4daab8cf1e27756ce8c19f80cb39bb2c91c`. The later coverage
amendment `9538272e638d6bca4547a61519a0ce9c98c3872d` changed release
documentation and state before the matched comparison and did not change
the baseline artifacts.

Linux x86-64 with an AMD Threadripper PRO 5945WX, all 24 logical CPUs in the
observed affinity, normal scheduling and ordinary host load. Exact compiler
and feature closures are in the evidence. The source locked rusqlite 0.41.1
with SQLite 3.54.1, sqlite-vec 0.1.9 and Candle trio `2ee39305`. The
current SDK ran the pinned F32 BGE model on CPU. The original archive
produced the initial dense vectors on its qualified .29 CUDA path. The
current opens, query inference and new natural-write projection used CPU.

The archived closed format-37 main file was 715,223,040 bytes, SHA-256
`e60fc08720a21748f4cf023a65d5fa55c3fe2df3da045d5e0cba1ee24d6d8f55`.
Its WAL had zero bytes. Copies opened with 100,000 active `doc` rows
from `test:slice30`, 100,000 logical, physical and span rows, zero
canonical edges, dependencies or authored node validity windows, and a
ready projection at cursor 100,000 with zero pending or failed work.
Open reports resolved the exact 384-dimensional BGE revision and all
three cached asset hashes. Hybrid diagnostic requests executed embedding,
both retrieval arms and returned vector-bearing hits. Direct text used
the same store and initialized provider, with no request embedding or
vector retrieval. Both used public SDK default options and a limit of 10.

| Ordinary route | Warm returns | p50 / p95 / p99 (ms) | Useful per loop second | Loop wall (s) |
| --- | ---: | ---: | ---: | ---: |
| Hybrid | 480/480 | 204.622 / 342.787 / 382.181 | 4.718 | 101.735 |
| Direct text | 480/480 | 20.255 / 59.105 / 75.254 | 41.432 | 11.585 |
| Hybrid with b1/s10 writes | 80/80 | 219.622 / 514.931 / 618.453 | 3.911 | 20.453 |

Each route had zero refusal, timeout, other error, degraded success or
missing completion. The warmups were a separate 80-call population per
read route. Their p50/p95/p99 were 193.239/273.780/290.043 ms hybrid and
17.623/37.476/52.154 ms text. The four first calls after SDK open and
untimed readiness were 117.878/126.268/126.268 ms hybrid and
5.126/5.291/5.291 ms text. These first-call tails are descriptive, and the
fixture copies share OS cache effects. G-W had 20 warmups, no separate
first-call population, and eight governed writes with eight current marker
probes.

Process user plus system CPU seconds per ordinary read run ranged from
66.99 to 68.12 hybrid and 7.36 to 7.61 text. Peak RSS was 518,552–521,460
KiB hybrid and 288,504–289,820 KiB text. G-W consumed 64.60 process CPU
seconds with 1,130,732 KiB peak RSS. GNU time reported zero input
operations and 14,448–14,456 output operations per hybrid read run,
55,064 per direct-text run and 5,295,448 in G-W. Sampled WAL was zero
during read-only runs and peaked at 1,829,312 bytes during G-W.
Copy preparation was outside those counters. The measured populations
returned 5,505,988 and 6,017,152 UTF-8 body bytes for hybrid and text,
respectively. The harness separately encoded 6,832,665 and 7,467,053 JSONL
bytes over all phases in those runs. Recorder wall was 30.455 and 29.979 ms
including 0.674 and 0.193 ms in buffer checkpoints. These metrics describe
harness persistence. The product routes return in-process values.

Fresh SDK opens of each first closed subject reproduced the ordinary
Q0–Q5 outputs and projection cursor. Two diagnostic reopens in each route
also preserved exact hit order, cursor, score bits, body, source and
default response fields. The first descriptive block had two hybrid
lifecycle processes between its route timings. The fixed treatment
comparison kept the two timed arms adjacent within every block.

## Request attribution

Twelve complete G-O samples per route, outside normal caller timing.
The table shows the median span duration in milliseconds. The hybrid
node FTS span also contains eligibility work. Spans overlap and are
not additive.

| Observed stage | Hybrid | Direct text |
| --- | ---: | ---: |
| Query compile | 0.010 | 0.005 |
| Query embedding dispatch wall | 19.632 | Inapplicable |
| Embedding mean bytes preparation | 0.119 | Inapplicable |
| Reader transaction | 170.058 | 17.485 |
| Reader response wait | 170.122 | 17.516 |
| Dependency presence | 0.019 | 0.016 |
| Guarded all-node eligibility | 50.046 | Guard skipped |
| Vector retrieval, including hydration | 27.348 | Inapplicable |
| Vector SQL / hydration | 27.141 / 0.196 | Inapplicable |
| Node FTS candidates | 135.101 | 17.364 |
| Edge FTS section | 0.190 | 0.125 |
| Fusion and final ranking | 7.095 | 0.022 |
| Deferred identity hydration | 0.115 | Inapplicable |
| Final API response shaping | 0.0004 | 0.0002 |

The reader wait includes its response. Queue-only latency and service outside
the observed reader transaction are unobserved because this observer profile
does not supply distinct matched admission and worker-start clocks. No
cross-encoder reranker ran at depth zero. The SDK returned in-process
values. The JSONL wrapper encoding and flushing are reported above.

The direct eligibility scan and winner-hydration route each ran on 12 of 12
hybrid requests. The median of the 12 directional scan times was 50.240 ms,
26.3% of the same requests' 191.237 ms median caller wall. The table uses
nearest-rank p50, while the prospective selection uses the middle-pair
median at even n. The bundled SQLite callback reported a canonical table
scan with 99,999 full-scan steps and 1,200,019 VM steps, while the dependency
check took zero full-scan steps and 13 VM steps over its covering index.
An early ineligible row stopped the former at zero full-scan steps and 27 VM
steps. A mixed window at about half the table took 49,999/600,020.
The equivalent shapes on an independently SDK-seeded 12-row store took
11/163, 0/27 and 5/92. The earlier raw counter file samples the statement
before its final SQLite step. The complete counter file includes that step.
Current evidence met the frozen treatment selection thresholds.

## Treatment and focused correctness

The selected node-only winner-hydration route applies the existing
ownerless existence and half-open validity predicates on the matched
canonical join before FTS ranking. The reference route still handles
its prior eager, mixed-edge, nondefault inclusion and other guarded paths.
The treatment adds no write, projection, format or public SDK change.

- RED `e0f4de0d7` observed one whole-store query in the actual all-eligible
  owner route. GREEN `fc878c90f` passed identical-state default-view reads
  in all-eligible, early-ineligible and mixed real stores. Controls checked
  pre-cap ranks, stored identity, source provenance, snippets, projection and
  visibility authority, and score bits. Hand-expected membership includes
  validity start at the captured second and excludes expiry at that second,
  future, pending and superseded rows. The existing .30 committed-state rank
  test also passed.
- RED `d97211205` preserved the original writer-reopen refusal. G-W correctly
  retained 100,002 physical, logical and span rows after two replacements
  while keeping 100,000 current active rows and ready projection. The old
  preparation predicate expected a virgin cardinality and native reopen
  exited 101 before reads. GREEN `9194f612a` added the opt-in written G-O
  census and rerunnable real SDK regression. It proved both current marker
  cursors and bodies on the same subject after close/open.
- The first fixed A/B comparison retained all 8 G-R and 4 G-W pairs. The
  preliminary build reduced paired warm p50 by 24.47% (48.80 ms) in G-R and
  35.93% (75.15 ms) in G-W. The Sol-high review rejected promotion of those
  bytes because a mixed-store ownerless raw signed-cursor tie changed the
  old reference order. RED `4ca1c54d4` reproduced the order, score and
  snippet change. GREEN `5db468a8c` preserved the old reference behavior
  on that anomaly path. Both malformed losing identity controls preserved
  the old typed outcome. The same ordinary proof was repeated once for
  the corrected artifact because the initial bytes failed correctness.
- The follow-up natural diagnostic query uses the first 60 words of each
  replaced stored body, plus the new revision marker query. Before writing,
  both original-body probes return the applicable old cursor. After
  reopening, both routes return only the current cursor for that ID.
  The analyzer also checks the matched request snapshot for each measured
  search, marker and reopening query. Successful degraded returns and
  incomplete native outputs have individual accounting. RED
  `00df0ddd1` reproduced complete-row loss on an interrupted summary.
  GREEN `5850f59e1` retained the completed row and missing outcome.
- Scoped controls passed for Slice 71 statement trace (2), Slice 35 pre-cap
  eligibility (3), lightweight ownership and malformed output (10), rank
  and projection (2), the native measurement example (12), and analysis
  (5). The Sol-high correction re-review passed the engine subject for
  native artifact dispatch. Terra independently checked the raw baseline.

## Corrected CPU comparison

The ordinary and diagnostic builds used the same source and lock inputs
within each pair apart from the engine treatment. The reference source was
`9194f612a`. The corrected native source snapshot was
`342d859591f43e5ecb22cb63d3b1c32e6bbcc680` with product correction
`5db468a8c`. Controller `5850f59e1` retained the two artifacts per arm
and their SHA-256 identities. All 24 timed arms completed with no retry.
Direct text retains its established scope and required no treatment rerun.

| Schedule | Arm | Successes | p50 / p95 / p99 (ms) | Useful per loop second |
| --- | --- | ---: | ---: | ---: |
| Hybrid G-R | Reference | 960/960 | 196.743 / 325.546 / 374.219 | 4.902 |
| Hybrid G-R | Corrected | 960/960 | 148.674 / 282.404 / 331.257 | 6.355 |
| Hybrid G-W | Reference | 320/320 | 216.584 / 505.166 / 610.194 | 4.066 |
| Hybrid G-W | Corrected | 320/320 | 134.245 / 263.157 / 311.228 | 6.764 |

The median paired G-R warm p50 reduction was **24.76% / 48.48 ms**.
All eight blocks improved. The observed paired range was 18.44%–27.15%.
The min/max interval has 99.22% sign coverage under independent blocks.
This nearby single-host design may have serial dependence. G-W improved
paired p50 by **37.27% / 80.27 ms** across four blocks, with observed
range 34.62%–40.02%. Median paired useful-rate ratios were 1.307× and
1.661×, CPU per successful measured call 0.894× and 0.858×, and peak
RSS 1.003× and 0.568× for G-R and G-W, respectively. Both pooled tails
improved. The eight G-R first calls separately had p50 of 116.709 ms
and 66.547 ms. The paired median ratio was 0.572×.

All measured calls returned without degradation, refusal, timeout, error or
missing response. Sampled WAL was zero on both ordinary read arms and
1,829,312 bytes in each G-W arm. Each arm returned 11,011,976 body bytes
in G-R and 3,450,008 in G-W. The process file-system counters and harness
JSONL sizes are retained in the corrected summary. G-W output operations
varied substantially across arms on the shared host. The work was a reader
SQL change, with identical stored-state operations and no observed increase
in WAL extent or body representation.

The fixed-time diagnostic pair had exact ordered result fields and matching
visibility, dependency and projection authority. Winner hydration ran in
12 of 12 requests in both arms. The global eligibility span ran in 12 of
12 reference requests and zero corrected requests. The corrected node FTS
span had median 84.043 ms versus 129.033 ms in the separate diagnostic
profile. Each diagnostic G-W arm joined 20 measured request snapshots,
two current marker snapshots and ten reopened snapshots, with ready
projection at 100,002. Original-body probes in both routes distinguished
old cursors 1/2 from current cursors 100,001/100,002. The ordinary G-W
cells also completed every governed cohort and marker probe.

**Disposition:** The corrected treatment meets the current CPU comparison
and correctness rules. Source adoption is supported on these inputs. Slice
60 must reconcile the exact final consumed native pins before promotion.
Slice 70 owns integrated source qualification and Slice 75 owns installed
artifact and platform proof under the owner-approved seq-407 split. The
separate inherited hints qualification remains open below.

## Workspace verification and scoped recovery

The single workspace gate on clean source `b757d0922b7c4461c826815f202e5d7135d587f8`
ran with owned Python 3.12, Node 26.13.1/npm 11.20.0, and a ptrace-capable
executor. Lint, typecheck and strict security passed with zero violations,
blockers or downgrades. The Tegra lint skipped on x86-64. The gate wrapper
exited **1** with **241 registered, 238 ran, 237 passed, 1 failed, 3 skipped,
0 excluded**. The three private opt-in controls skipped. Workspace Rust,
Python and its native receipt, TypeScript, and the hermetic N-API build
passed. The source-bound Python wheel and model/artifact hashes are retained.

The sole failing suite was `test-slice30-g-observer` (rc 101, 9 passed,
1 failed). Its original node-only fixture expected an event for an executed
global SQL proof. That fixture received zero global eligibility events in the
new path. RED `22735c2ff` retained the full original SQL assertions on a
real mixed-edge store and added the node-only skipped-scan requirement.
That mixed-edge case passed, and the new event assertion genuinely failed.

GREEN `8f71f4cb8` emits one explicit `match_local_eligibility` skip for a
request that bypassed the scan. The 11 observer cases and targeted Clippy
passed. Sol independently reviewed the correction. The rebuilt CPU
diagnostic artifact preserved all 12 full fixed-time results on fresh 100k
copies and emitted exactly one untimed, no-SQL skip per request. The rebuilt
ordinary feature closure excludes the event and matched all four selected
same-state SDK outputs. Terra independently checked the recovery outputs
and artifact hashes. This scoped check adds no latency sample. The
ordinary timing evidence remains attached to the measured binaries.

The original gate result is retained alongside the passing affected
recovery. The final branch incorporates metadata `1e4459376` after the gate.
There is no uninterrupted full-gate pass claim.

## Inherited MCP hints qualification

Keep the 200 µs bound and four failing results from
[Slice 10](../slice-10/status.md): 289.9, 366.8, 345.3 and 374.6 µs. The
ordered baseline and candidate matched their runtime feature profile, with
the recorded host-only proc-macro compile-profile difference. Those
observations did not establish noise as the cause.

The single current unchanged devloop execution passed. Its quiet
libtest run suppressed numeric stdout for a passing case. The separately
specified balanced 31×15 diagnosis kept one iteration and all hits.
At limit 100, planning added a median 2.685 µs, preplanned emission and
shaping 186.162 µs, and the combined seam 182.124 µs with p99 of
218.964 µs. The limit-10 combined addition was 57.209 µs median and
69.712 µs p99. The seam includes an additional structured string
conversion. At limit 100 the structured output grew from 40,675 to
41,386 bytes. This attributes the current synthetic delta predominantly
to the post-planning seam without determining the cause of the original
failures. **The inherited hints qualification remains open.**

Raw current artifacts, attempts, per-query diagnostics, counter work and
comparison outputs are under `/tmp/fathomdb-0830-slice40-evidence`. Original
hints evidence remains in `/tmp/fathomdb-0830-slice10-evidence` and
`/tmp/fathomdb-0830-terra-node/hints-pair-result.json`. The
[study commands](../../../../experiments/0.8.30-slice40/README.md) regenerate
the selected comparisons from the declared inputs.
