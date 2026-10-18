# How often does a call time out behind a slow call, and how sophisticated must FathomDB's queueing be?

Measurement, code trace and discrete-event simulation. 2026-10-18.

Files (all under this directory): `sim.py` (simulator), `test_sim.py` (46 tests with `test_fuzz.py`, `test_runner.py`),
`measure.py` (sequential service times against the staged server), `probe.py` (burst probe), `fuzz.py`, `grids.py`,
`step.py`, `incident.py` (studies), `runner.py` (crash-safe resumable driver), `analysis.py` (tables),
`results/` (`tables.md` with every table, summaries, the small per-study CSVs, `incident-transcripts/` extracted
from the six subagent transcripts; the large fuzz CSV/JSONL runs and the superseded `pre-review/` round are not
committed, see `README.md`). The in-repo code trace is
`dev/design/agent-access/queue-lock-trace.md` on branch `exp/queue-lock-trace` (`24c726f4c`).

## 1. What was measured on the real server

Staged binary `~/.local/share/fathomdb-mcp-eval/fathomdb-mcp`, RTX 3090 #0, store `conv4` (3.3 MB; LoCoMo
conversation 4, the largest), one call at a time after a 60-call warm-up, 400 calls per type, LoCoMo
questions as queries. Times are client-side at the stdio boundary (include rmcp and JSON).

| call | n | p50 ms | p90 | p99 | max |
| --- | --- | --- | --- | --- | --- |
| search text, limit 10 | 400 | 1.00 | 1.39 | 1.79 | 1.92 |
| search text, limit 100 | 400 | 1.53 | 2.02 | 2.60 | 4.20 |
| search hybrid, limit 10 | 400 | 6.92 | 8.12 | 10.48 | 11.26 |
| search hybrid, limit 100 | 400 | 7.66 | 8.82 | 11.23 | 12.38 |
| fetch, 10 refs | 400 | 0.50 | 0.63 | 0.75 | 3.60 |
| expand, max_depth 3 | 400 | 0.36 | 0.45 | 0.56 | 1.06 |

The stores have no edges, so `expand` returns nothing and measures only the round trip. No slow tail exists on
these stores; the simulation fuzzes one (section 4).

**Burst probe** (`probe.py`, `results/burst_probe.csv`): k calls written in one stdin write, each response
timestamped. Busy responses return in under 1 ms. A warm 10-hybrid burst at cap 4 clears in 45 ms (cap 7:
68 ms); 10 text in 3.5 ms. The first burst on a fresh process takes 490 ms (model load). Concurrency sweep at
cap 7: hybrid 6.2 / 19 / 44 / 67 ms for 1 / 2 / 4 / 7 concurrent calls, so **query embeddings serialize on the
GPU at about 9.5 ms each**; text 0.95 / 1.2 / 1.9 / 5.5 ms with a knee at 5 concurrent calls (cause not
identified; candidates are the five WAL read-mark slots and the global page-cache mutex).

**The incident's "330 ms"** (research § 9 input 2): in the transcripts, busy results reached the agents
294–338 ms after the turn's last tool call was emitted, ok results 536–709 ms after. The server returns busy
in under 1 ms and cleared each 10-call burst before the next agent's burst arrived 70–89 ms later. The 330 ms
is Claude Code's dispatch and result handling, not server time. Service time under concurrency (GPU
serialization) accounts for the 45 ms server part.

## 2. Grounding of the simulation inputs

From the six subagent transcripts (`results/incident-transcripts/calls.csv`): 97 MCP calls in 31 turns; turn
sizes 1×14, 2×3, 3×9, 10×5; every one of the 29 busy results was in a 10-call fan-out turn (4 ok + 6 busy at
cap 4, one agent 5 busy); inter-turn gaps median 3.3 s (1.5–14.5 s); at most 34 calls in flight client-side.
Claude Code subagents share the parent's one MCP session.

From the literature (46 cited sources, `research-scratch/REPORT.md`): agent think time log-normal, p50 ~5 s,
p90 ~20 s (TraceLab, 8k Claude Code/Codex sessions); tool calls per turn mean 1.2, p99 4 in the general
population, with prompted-parallel agents far above that and no documented cap; Claude Code default 20
concurrent subagents; Claude Code's MCP tool timeout is effectively none (~28 h), so the server's 10 s deadline
binds; agents do not sleep on busy, they re-issue on the next LLM turn; SQLite WAL read p99 is 3–5× median
with a writer active, worst 10–30×; a 100–1000× cold/checkpoint mode at p 1e-3–1e-2 is a design assumption;
for shed cutoffs, practitioners use high percentiles (AWS p99.9, hedging at p95), and low percentiles or
minimums only as a queue-presence signal (CoDel, Vegas), with Netflix documenting over-shedding from a
minimum-biased estimate.

Simulator inputs therefore: think time lognormal median 3–5 s, σ 0.55–1.1; two client shapes, prompted-parallel
(one F-call fan-out per agent session, F 4–12, then 1–3-call turns) and TraceLab-like (1–4 calls, mean 1.2);
K 1–32 agents; agents never honour `retry_after_ms` in the main fuzz (they re-issue after a think time); slow
reads at p 0–5 % (realistic band 0.1–1 %), median 0.2–8 s lognormal; writer mutex holds 1–500 ms at duty
1–80 %; 0–2 background projection embed workers with 50 ms–2 s batches; CPU-like times ×5–20 in 30 % of
configs. Hybrid embeds serialize on one GPU at 1.8× solo time under contention (calibrated to the probe within
15 % for hybrid; text under-predicted above 4 concurrent, the knee above).

## 3. Simulator validation

- 46 deterministic tests: refuse/FIFO/shed/predict mechanics, late-permit refusal, abandoned work holding its
  permit, retry_after formulas with effective capacity, hand-traced head-of-line example under P0 (read 9 waits
  4.2 s behind a 5 s read while 7 workers idle; P1 and P2 do not), P2 fallback, embed saturation and expiry,
  writer mutex, EP-09 interrupt paths, projection contention, GPU serialization, common random numbers,
  calibration against the live probe, and the review findings below.
- M/M/c against Erlang C (P1, exponential service): c = 2 ρ = 0.7 Wq 0.965 vs 0.961, Pw 0.580 vs 0.577;
  c = 8 ρ = 0.8 Wq 0.288 vs 0.286; c = 8 ρ = 0.95 Wq 2.119 vs 2.110. All within 1.5 %.
- An adversarial code review (Fable, medium) found 5 bugs and 5 suspicions; all were fixed test-first
  (false-refusal counterfactual for same-instant refusals, a duplicated realistic-band sampler, common random
  numbers under retry_after, EP-09 not dropping a queued GPU forward, embed-queue expiry evaluated late, HOL
  accrual past the deadline, injected probes in metrics) and re-verified by the reviewer. Every study in this
  report was rerun after the fixes; the pre-review round was kept locally and is not committed.
- Known gaps: text service above 4 concurrent calls is under-predicted (the live knee); within a burst the
  simulated hybrid completions form a staircase while live ones finish nearly together (total clearing time
  matches); service-time estimators see only on-time completions, as the design specifies, which biases the
  shed threshold low under overload.

## 4. Code trace: the stack of queues and locks

Full trace with file:line citations in `dev/design/agent-access/queue-lock-trace.md`. 42 primitives on one
call's path. The findings that matter for the queue design:

- **A. Abandoned work keeps every resource.** The adapter's timeout abandons only the JoinHandle; the closure
  owns the permit and the blocking thread, the embed wait runs to its own 30 s, and a queued reader request
  still runs in full. Nothing below the adapter knows the deadline.
- **B. Round-robin head-of-line.** `fetch_add % 8` then a blocking `send` to a per-worker `sync_channel(4)`;
  the reply `recv()` has no timeout.
- **C. One embed queue for queries and projection.** 64-text projection batches share the five embed slots
  and the GPU with query embeddings; at 20 waiting, queries silently run text-only; projection polls every
  25 ms when saturated.
- **D. Writer mutex shared with reads.** Hybrid (mean vector) and orient (three acquisitions) take the writer
  connection's unfair `std::sync::Mutex`; in embedded mode writers hold it for whole transactions.
- **E. One global page-cache mutex.** `SQLITE_ENABLE_MEMORY_MANAGEMENT` in the bundled build forces a single
  page-cache group with one mutex on every page fetch for all 12 connections. Design row H13 is wrong.
- No lock-order inversion was found on the read path. Unbounded queues exist only above admission (tokio
  tasks) and are bounded in practice by the cap.

The 2026-10-18 incident used none of A–E: it was fan-out above a zero-length queue (section 5).

## 5. Incident reproduction (research § 9 inputs 1, 8)

Model: 6 agents in one session, each session one 10-call hybrid fan-out turn then 1–3-call turns, think
median 3.3 s σ 0.55, starts spread over 3.5 s, 97 calls, 300 seeds (`incident.py`, `results/incident.csv`).

| admission | queue limit | cap | refused / calls | refusals inside fan-out turns | fan-out turns with exactly 10−cap refused | ok p99 |
| --- | --- | --- | --- | --- | --- | --- |
| refuse (today) | — | 4 | 40 % [p10 37, p90 43] | 100 % | 97 % | 40 ms |
| refuse | — | 7 | 21 % | 100 % | 95 % | 64 ms |
| fifo_shed | 8 (2×cap) | 4 | 0.7–1.2 % | 100 % | 0.4 % | 90 ms |
| fifo_shed | 8 | 7 | 0.2–0.3 % | 100 % | 0.2 % | 91 ms |
| fifo_shed | unbounded | 4 or 7 | 0 % | — | — | 90 ms |

The mechanism matches the transcripts: busy appears only in fan-out turns, and a 10-call burst at cap 4 loses
exactly 6 calls. The aggregate differs (40 % vs the observed 30 %) only because one of the six real agents never
fanned out: 5 fan-out turns × 6 = 30 predicted, 29 observed. Cap 7 would still have refused 3 per burst (21 %).
**The zero-length queue, not cap 4, was the constraint.** The research's claim holds. Client ceiling (input 8):
the transcripts show up to 34 calls in flight from the one session, so no per-session concurrency cap below
that was in force; a per-agent cap of 10 is consistent with the 10-call turns.

## 6. Queue depth, overload and head-of-line (research § 9 inputs 4, 5, 7)

**Depth (T3 in `results/tables.md`):** grounded agents, 10-call fan-outs, cap 4, hybrid, P0:

| depth | K=2 | K=6 | K=12 | K=20 (refused / turns hit / burst p99 / max queue / worst lone wait) |
| --- | --- | --- | --- | --- |
| 0 (today) | 34.7 % / 20 % | 34.9 % / 20 % | 35.2 % / 21 % | 35.6 % / 21 % / 36 ms / 0 / 0 |
| 8 (= 2×cap, design default) | 0.1 % / 0.1 % | 0.3 % / 0.3 % | 0.5 % / 0.6 % | 1.0 % / 1.1 % / 102 ms / 8 / 74 ms |
| 16 | 0 | 0 | 0 | 0.1 % / 0.0 % / 119 ms / 16 / 100 ms |
| 32 | 0 | 0 | 0 | 0 / 0 / 120 ms / 24 / 156 ms |
| unbounded | 0 | 0 | 0 | same as 32 (max queue 24) |

Two fan-outs overlapping (12 arrivals at cap 4) already exceed a depth of 8. Depth 32 is never reached by 20
closed-loop agents (max 24). A lone call behind another agent's burst waits at most 156 ms. The arrival test
(`fifo_predict`) changes nothing here: predicted waits are tens of milliseconds against a 10 s deadline. Cap 7
halves burst completion times (hybrid burst p99 120 → 123 ms is unchanged because the GPU, not the cap, bounds
hybrid throughput; text bursts clear 2× faster).

**Sustained overload (T4):** clients with no think time, 10-call bursts, cap 4, P0. One client: depth ≥ 8
absorbs everything (0 refused, 1128 text or 134 hybrid ok/s; refuse-at-once gives 60 % refusals and half the
goodput). Four clients, no slow reads: depth 8 refuses 61–72 %, depth 32 refuses 2–7 % at p99 9 ms text /
315 ms hybrid, unbounded 0 % at the same p99. Four clients with 1 % two-second reads: depth 32 keeps goodput
(39 ok/s) with 2–7 % refused but p99 5.7 s and 0.11 % timeouts; the arrival test (EWMA or p90) refuses 94 %
and halves goodput (16–18 ok/s) for 0.02 % timeouts, with 13 % of its refusals false; the p99 variant refuses
78–96 % with 36–51 % false. **Under real overload the arrival test protects latency by shedding most work; a
depth-only queue keeps goodput and lets latency grow toward the deadline.** Neither is wrong; they are
different contracts. For closed-loop agents (every realistic case) the question never arises.

**Head-of-line (T5):** one slow read injected every 2 s, fifo_shed, depth 32; other calls' p95 / timeouts:

| slow read | workload | P0 | P0 + EP-09 | P1 | P1 + EP-09 |
| --- | --- | --- | --- | --- | --- |
| 1 s | 6 agents, cap 4 | 47 ms / 0 | 47 ms / 0 | 32 ms / 0 | 32 ms / 0 |
| 3 s | 6 agents, cap 4 | 1.9 s / 0 | 1.9 s / 0 | 35 ms / 0 | 35 ms / 0 |
| 10 s | 6 agents, cap 4 | 9.6 s / 6.2 % | 8.4 s / 0 | 9.9 s / 0 | 3.9 s / 0 |
| 10 s | 6 agents, cap 7 | 9.3 s / 13.9 % | 8.4 s / 0.01 % | 37 ms / 0 | 37 ms / 0 |
| 3 s | cap clients, no think, cap 4 or 7 | 3.0 s / 0 | 3.0 s / 0 | 2 ms / 0 | 2 ms / 0 |
| 10 s | cap clients, cap 7 | 49 ms / 22.9 % | 8.0 s / 0 | 3 ms / 0 | 3 ms / 0 |

By the design's own rule (§ 7.3: EP-18 A proceeds if the probe raises other calls' p95 by more than 2×), a
3 s slow read is enough: P0 raises p95 from 35 ms to 1.9 s; P1 is unaffected. A 1 s slow read is not: p95
stays under 50 ms either way (p99 678 ms vs 52 ms). At 10 s slow reads with cap 4 the cap itself is the
bottleneck (five abandoned slow reads hold all permits; hazard A), and only EP-09 helps; at cap 7 the shared
queue alone removes all timeouts. The probe is valid only if a ≥ 3 s read exists on real stores; on the
3 MB test stores the slowest call ever measured was 12 ms.

## 7. Main fuzz: timeout rate by admission × dispatch

2000 configurations over the full ranges (section 2) plus a dedicated realistic band of 500 (K 1–8,
slow p log-uniform 0.1–1 %, GPU), each run for 20 000 calls in 14 paired arms with common random numbers.
Failure = timeout + busy refusal, per call. Full tables T7–T11 in `results/tables.md`.

| arm | realistic: timeout / refused | realistic, writer off and no projection: fail / ok p99 | stress (K ≥ 16, slow ≥ 2 %): timeout / refused |
| --- | --- | --- | --- |
| refuse · P0 (today) | 0.06 % / 8.25 % | 9.03 % / 125 ms | 0.47 % / 31.8 % |
| fifo (wait to D) · P0 | 0.08 % / 0.17 % | — | 1.54 % / 16.8 % |
| fifo_shed · P0 | 0.08 % / 0.18 % | 0.17 % / 260 ms | 0.99 % / 16.4 % |
| fifo_shed · P1 (shared queue) | 0.05 % / 0.06 % | 0.03 % / 35 ms | 0.55 % / 8.1 % |
| fifo_shed · P2 (try idle, else RR) | identical to P1 in every configuration | | |
| fifo_predict (arrival test) · P1 | 0.05 % / 0.11 % | 0.03 % / 35 ms | 0.38 % / 8.4 % |
| fifo_shed · P0 · EP-09 interrupt | 0.05 % / 0.06 % | 0.03 % / 214 ms | 1.41 % / 7.6 % |
| fifo_shed · P1 · EP-09 interrupt | 0.05 % / 0.06 % | — | 1.35 % / 4.3 % |

By client shape in the realistic band (failure = timeout + refusal): prompted-parallel agents fail 15.95 % today
(77 % of configurations above 1 %), 0.38 % with fifo_shed on P0 and 0.15 % on P1; TraceLab-like agents fail
0.24 % today, 0.12 % and 0.07 %. The realistic band's ok p95 of 0.5–1 s comes entirely from the writer mutex and projection
contention (writer off, no projection: p95 20 ms); it does not depend on the admission or dispatch policy.

- **P2 ≡ P1.** With the cap below 8 readers there is always an idle reader, so `try_send` to an idle worker
  always succeeds. P2 is the cheap, lock-free way to get P1's behaviour while the adapter cap is ≤ 7 and no
  host SDK reads bypass admission.
- **Timeout causes** (fifo_shed, all configs, as a share of all calls; the largest component inside the call's
  own deadline window): P0 0.23 % head-of-line, 0.22 % admission wait, 0.13 % own slow read, 0.40 % other
  (embed queue, GPU and writer-mutex waits); P1 0.00 % / 0.17 % / 0.15 % / 0.39 %. In the realistic band
  the totals are 0.075 % (P0) and 0.051 % (P1), with HOL 0.024 % of calls under P0 and none under P1.
- **EP-09** frees the permit at the deadline: abandoned permits drop to zero and refusals halve in the stress
  band, while timeouts rise (calls that would have been refused are now admitted and some time out). Total
  failure falls (stress: 17.4 % → 9.0 % on P0, 8.7 % → 5.6 % on P1).

**HOL map** (paired failure excess P0 − P1 under fifo_shed, all configurations):

| slow p \ slow median | 0.2–0.5 s | 0.5–1 s | 1–2 s | 2–4 s | 4–8 s |
| --- | --- | --- | --- | --- | --- |
| 0–0.5 % | 0.04 % | 0.05 % | 0.40 % | 0.95 % | 2.33 % |
| 0.5–1 % | 0.04 % | 0.58 % | 0.32 % | 0.97 % | 6.19 % |
| 1–2 % | 0.06 % | 0.81 % | 2.29 % | 4.13 % | 10.30 % |
| 2–5 % | 0.39 % | 0.83 % | 4.20 % | 8.47 % | 16.23 % |

By agents and cap: K 1–4 0.48 % (cap 4) / 0.24 % (cap 7); K 5–8 1.50 % / 0.64 %; K 9–16 3.3 % / 3.6 %;
K 17–32 7.0 % / 6.4 %.

**Decision tree** (regression tree on the same excess, 2000 configurations):

```text
slow-read median < 2.6 s
├─ slow-read spread σ < 1.16
│   ├─ median < 1.6 s  → P0 costs 0.3 % extra failures (840 configs)       ← realistic agent workloads
│   └─ median ≥ 1.6 s  → 1.8 %
└─ σ ≥ 1.16 (heavy tail)
    ├─ median < 1.1 s  → 1.4 %
    └─ median ≥ 1.1 s  → 10.4 %
slow-read median ≥ 2.6 s
├─ K < 20 agents
│   ├─ σ < 1.08  → 2.5 %
│   └─ σ ≥ 1.08  → 11.8 %
└─ K ≥ 20 agents
    ├─ no fan-out  → 12.8 %
    └─ fan-out ≥ 4 → 25.2 %
```

Rule of thumb: P0's round-robin costs under ~0.5 % extra failures until single reads routinely take more
than about 1.5 s; it costs several percent once multi-second reads occur with a heavy tail or with 10+ agents,
and tens of percent at 20+ fanning-out agents. With the cap ≤ 7, P2 removes the penalty entirely.

**Which knob matters most** (η², share of variance in failure rate, all configurations):

| knob | today (refuse · P0) | fifo_shed · P0 | fifo_shed · P1 | P0 − P1 excess |
| --- | --- | --- | --- | --- |
| client fan-out size (session prefix) | 0.43 | 0.16 | 0.09 | 0.12 |
| writer-mutex duty | 0.39 | 0.25 | 0.12 | 0.23 |
| CPU-like service (no GPU) | 0.45 | 0.30 | 0.15 | 0.43 |
| projection workers sharing the embed queue | 0.07 | 0.07 | 0.14 | 0.01 |
| agents K | 0.11 | 0.07 | 0.02 | 0.08 |
| slow-read median | 0.06 | 0.07 | 0.01 | 0.20 |
| admission cap (4 vs 7) | 0.03 | 0.01 | 0.01 | 0.00 |
| hybrid fraction, think time | ≤ 0.03 | | | |

Today, failures are driven by fan-out size (the queue length is zero). Once the queue exists, what remains is
service time (CPU vs GPU), writer-mutex contention and slow reads. The cap is the least important knob.

## 8. The admission cutoff: fixed or adaptive (owner's second question)

600 configurations × 2 dispatch policies × 27 cutoff policies, queue depth 2×cap (the design default), clients
that honour `retry_after_ms` (so its accuracy can be measured). Columns as fractions of all calls: false refusal
counts any refusal whose call would have finished if queued (exact FIFO counterfactual), which is dominated by
`queue_full` refusals at depth 2×cap and therefore nearly identical across cutoffs. The cutoff-specific columns
are the deadline sheds and the late admissions.

| cutoff (P1, all configs) | deadline sheds | of which the call's own service fit the time left | late admits that timed out | fail total |
| --- | --- | --- | --- | --- |
| T0: wait until D = 10 s | 0.39 % | 0 | 0.287 % | 12.09 % |
| T1: fixed 50 ms / 250 ms / 1 s | 0.40 % | 0.39 % (250 ms) | 0.288 % / 0.285 % / 0.282 % | 12.04–12.10 % |
| T2: p5 of last 128 | 0.41 % | 0.10 % | 0.230 % | 11.98 % |
| T2: p50 of last 128 | — | — | 0.184 % | 11.86 % |
| T2: p90 of last 128 (32 and 512 the same) | 0.47 % | 0.33 % | 0.162 % | 11.82 % |
| T2: p95 of last 128 | — | — | 0.160 % | 11.83 % |
| T3: EWMA α 0.1 × 1 / × 2 | — | — | 0.188 % / 0.143 % | 11.92 % / 12.01 % |
| T3: EWMA α 0.3 × 2 | 3.30 % | 3.18 % | 0.132 % | 12.82 % |
| D = max(1 s, 3 × p99) with p90 cutoff | — | — | 0.282 % | 13.70 % |
| D = max(1 s, 10 × p99) with p90 cutoff | — | — | 0.204 % | 13.44 % |

P0 shows the same ordering (late admits: T0 0.337 %, p5 0.280 %, p90 0.209 %, p95 0.200 %; fail total 24.2 % vs
23.6–24.0 %). In the overloaded subset (T0 fails > 0.5 %) the ranking is unchanged.

- **The cutoff is a second-order knob.** All cutoffs land within 0.3 points of each other. It matters only for
  calls that wait seconds behind slow or abandoned work, and those are rare.
- **Fixed floors do nothing.** 50 ms, 250 ms and 1 s cut almost no late admissions (0.285 % vs 0.287 %), and the
  250 ms floor refuses calls that would have finished (0.39 % false sheds). Against a 10 s deadline with ms-scale
  service, a fixed number is either irrelevant or wrong, and it cannot follow a shift in service time.
- **p5 is too optimistic.** Admitting when remaining ≥ p5 means admitting whenever at least one call in twenty
  of this kind would have finished in the time left. It removes only a fifth of the late admissions (0.287 → 0.230 %).
- **p90 to p95 is the right percentile.** p90 admits a call only if nine in ten recent calls of this kind would
  have finished in the time left. It removes 44 % of late admissions for 0.07 points more deadline sheds. p95 is
  equal within noise. EWMA α 0.3 × 2 removes slightly more but sheds about ten times as many calls whose own service
  would have fit, and it oscillates after a shift.
- **Window:** 32, 128 and 512 give the same steady-state result. After a 10× service step (T14 in
  `results/tables.md`), p90 with a window of 128 re-converges in about 76 s with low jitter (coefficient of
  variation 0.12). A window of 32 converges in 12 s but oscillates (CV 1.4). A window of 512 takes 5 minutes.
  p5 with 512 takes 10 minutes. EWMA α 0.3 × 2 oscillates (CV 1.5) and swings the threshold to 2.3 s.
- **Keep D fixed.** A derived deadline (k × p99) fails more calls with both k = 3 and k = 10. It shortens the
  wait for calls that would have succeeded and lengthens it after slow periods.
- **The arrival test of queue-manager-research.md § 7** (A_s rows in T12) has the lowest totals only because
  it runs with depth 32. Its own refusals are 2.4 % `predicted`. In the overload grid (section 6) it halves
  goodput for no gain in failures. With a deep queue, the time-based shed already does its job.
- **`retry_after_ms` fails the design's ship rule.** Observed wait is a median 4× the advised value; 23 of
  479 cells pass. Per § 7.3, ship busy without `retry_after_ms` and revise the formula. Agents also do not
  sleep on it; they re-issue a turn later.

**Recommended default, in operator words:** "A waiting call is started only if, at the current pace, nine out
of ten recent calls of the same kind would finish in the time it has left; otherwise it gets `busy` at once."
Configuration: `shed_percentile = 90`, `shed_window = 128` completed calls per tool, with a 50 ms floor while
fewer than 8 calls have completed. The deadline stays a fixed 10 s.

## 9. Answers

**How often does a call time out behind a slow call?** On today's stores, essentially never: the slowest of
2 400 measured calls took 12 ms against a 10 s deadline. With a fuzzed slow tail in the realistic band
(K ≤ 8, slow reads 0.1–1 %), 0.024 % of calls time out from head-of-line waiting under round-robin (P0) and
none under a shared queue. Today's real failure is not timeouts at all: it is 8 % busy refusals (16 % for
fan-out agents) from a zero-length queue.

**When does HOL cause timeouts under P0, compared with P1 and P2?** See the decision tree in section 7. Under
0.5 % extra failures while slow reads stay below about 1.5 s; several percent once reads routinely take
multiple seconds with a heavy tail or with 10+ agents; up to 25 % at 20+ fanning-out agents with 2.6 s+
reads. The HOL probe shows the threshold sharply: a 3 s read every 2 s pushes other calls' p95 from 35 ms to
1.9 s on P0, while P1 is unaffected; a 1 s read does not. P1 and P2 behave identically while the cap is
below the number of readers.

**Is FIFO wait plus early shed enough, without changing engine dispatch, for realistic agent workloads?**
Yes. It takes failures from 8.3 % to 0.25 % (0.075 % timeouts + 0.175 % refusals), and from 16 % to 0.38 %
for fan-out agents. The remaining gap to P1 (0.25 % vs 0.11 %) is mostly refusals (0.12 points) from permits held
by HOL-blocked or abandoned calls, plus 0.02 points of HOL timeouts; EP-09 alone closes it (0.11 %). Two conditions: the queue depth must cover overlapping fan-outs
(depth 8 = 2×cap leaks 0.3–1 % at K 6–20; depth 16–32 leaks nothing), and the shed must use a high
percentile (section 8).

**At what point does P1/P2 matter?** When single reads take more than about 1.5–3 s: stores large enough
that a deep `expand` or a large-limit hybrid search takes seconds, a cold cache, a CPU-only host (service
×5–20; the single largest driver of the P0 penalty, η² 0.43), or 10+ concurrent agents. In the stress band
P1 halves failures (17.4 % → 8.7 %). P2 gets this without a lock while the cap stays at most 7.

**Which knob matters most?** Today, queue depth: fan-out size explains 43 % of the variance in failures, the
cap 3 %. With a queue: service time (GPU vs CPU) and writer-mutex contention, then slow-read magnitude.
Raising the cap from 4 to 7 is the least effective knob; it halved refusals in the incident (40 % → 21 %) but
a depth-16 queue removes them.

**Why the 330 ms, and what it means for sizing:** client-side (section 1). Server burst clearing is 45 ms
(hybrid, GPU-serialized) or 3.5 ms (text), so any queue that absorbs a burst costs agents tens of
milliseconds, against hundreds already spent in the client.

## 10. Verdict: how tricky do we need to get?

Not very, and in this order:

1. **A bounded FIFO wait in the adapter (queue-admission.md § 6.1) with a depth of 32, not 2×cap.** This
   alone removes the incident (refusals 40 % → 0 % in the reproduction) and 97 % of realistic failures. The
   depth is a backstop that 20 closed-loop agents never reach (max observed 24); closed-loop agents cannot
   overload it.
2. **Shed by time with p90 of the last 128 completions per tool** (section 8), deadline fixed at 10 s. Do not
   add the arrival-prediction test: it changes nothing for agents and halves goodput under overload. Ship busy
   without `retry_after_ms`: its estimate is 4× off and agents do not sleep on it.
3. **Make reader dispatch work-conserving cheaply: P2** (`try_send` to an idle rendezvous worker, else
   round-robin). Under the adapter's cap of at most 7 it is exactly as good as a shared queue, with no lock. Do
   this before stores grow to where reads take seconds; a full shared queue (EP-18 A) matters only if host SDK
   reads bypass admission and fill all eight readers.
4. **EP-09 (deadline into the engine: reply timeout + `sqlite3_interrupt`)** is the only thing that helps when
   reads exceed the deadline. It ends abandoned work, so permits are never held by dead calls (hazard A).
5. **Separately from queueing: the writer mutex and the shared embed queue** (hazards C, D) set the latency
   floor of hybrid search (0.5–1 s p95 whenever a writer or projection is active) and are independent of every
   admission choice. Move the mean-vector and orient reads off the writer connection, and give query embedding
   priority over projection.

Adaptive concurrency limits, LIFO, per-session partitioning and CoDel-style controllers are not needed at
these loads: the cap is not the bottleneck, and the stack's real risks are the engine-side hazards above.

## 11. Limits of the model

- Service times come from 3 MB stores where nothing is slow. The slow tail is fuzzed, not measured; its
  realistic band (100–1000× median at p 1e-3–1e-2) is a design assumption grounded only loosely in the
  literature (no published SQLite p999 was found).
- Not modelled: SQLite-internal contention (the global page-cache mutex, WAL read-mark slots, checkpoints,
  WAL growth), page-cache effects and cold caches, beyond the fuzzed writer-mutex holder. The live text knee
  at 5 concurrent calls is one sign of this and is under-predicted.
- The GPU is modelled as one FIFO server with a contention factor fitted to one probe on one RTX 3090; burst
  totals match within 15 %, but per-call ordering inside a burst does not.
- Client behaviour is grounded in six transcripts from one evaluation plus published population statistics;
  real agent fleets may fan out wider or retry differently. Counterfactual false refusals assume the refused
  call alone would have been queued in FIFO order behind the actual later releases.
- The estimators see only on-time completions, as the design specifies, which biases shed thresholds low
  under overload.
- Single host, single store, no embedded-host SDK traffic except as fuzzed writer holds.
