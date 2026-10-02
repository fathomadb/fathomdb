---
title: ADR-0.8.27-embed-dispatch-default-capacity
date: 2026-10-02
target_release: 0.8.27
desc: Select the smallest qualified default embed-dispatch worker count while retaining bounded admission and the synchronous engine topology.
blast_radius: engine default configuration; Rust, Python, and TypeScript open behavior; runtime and performance qualification
status: proposed (HITL selection rule seq-299; remaining named selectors pending)
---

# ADR-0.8.27 — Embed-dispatch default capacity

**Status:** proposed. The repository owner authorized a measured selection rule
at `seq-299`: test higher default embed worker counts and choose the lowest
count that passes the unchanged frozen D27 comparison, AC-081, and named
release performance gates. The five-worker candidate has passed D27 and AC-081;
the remaining named selectors are pending. This ADR becomes accepted only when
those selectors pass and the exact selected source is recorded below.

## Context

The accepted [engine-owned runtime topology](ADR-0.8.27-engine-owned-runtime-topology.md)
sets `embedder_pool_size` to one by default. That setting shares one provider
slot between projection and foreground embedding. The strict six-repetition
D27 comparison found provider service time near its historical 2.06 ms p50,
but queueing raised direct-embed p95 to about 10.31 ms against the historical
2.38 ms ceiling. The unchanged AC-081 eight-reader vector-only search gate
also returned empty results when the one-worker/four-waiting-slot admission
bound saturated.

The remedy must preserve the accepted `1..=64` configuration range, the
`4 * embedder_pool_size` waiting-queue formula, nonblocking admission,
operation-specific overload outcomes, absolute deadlines, fixed-worker
shutdown, and the existing synchronous SQLite ownership topology. The frozen
D27 protocol, corpus, workload, and comparison rule are unchanged.

## Proposed decision

Set the omitted `embedder_pool_size` default to **five** after all named release
selectors pass on the exact candidate. Explicit values remain `1..=64`; callers
can still select one worker. The default starts five embed-dispatch workers
when a provider is attached, permits up to five simultaneous calls to that
provider, and allows 20 waiting requests. Without a provider it starts no
embed worker or request queue. `scheduler_runtime_threads` remains two by
default. No queue, executor, deadline, or provider-failure rule changes.

The choice is based on the first full six-repetition candidate comparison to
pass the frozen D27 rule after lower-count failures. The two-worker run at
`781b2f5e0`, three-worker proof at `2a62b14c7`, and four-worker run at
`c03a398f8` each completed all six environment-valid repetitions and failed
direct-embed latency. The three-worker direct-embed p95 medians were 4.139 ms
under projection-heavy load and 4.141 ms under foreground-heavy load. An
earlier three-worker attempt was invalidated by host swap and is not counted.
Five workers at `ad31c3a61` passed six
valid D27 repetitions. Its direct-embed p95 medians were 2.116 ms under
projection-heavy load and 2.120 ms under foreground-heavy load. The same
exact source passed AC-081a/b in seven fresh official runs and AC-081c.

## Supersession and consequences

On acceptance, this ADR supersedes only the default-one and default-one
hung-provider posture clauses in the engine-owned runtime topology ADR. The
five-worker default allows other slots to progress when one provider call is
permanently hung, until all five slots are occupied. Such a call still retains
its slot and does not create a replacement worker. Explicit one-worker
configuration retains the prior one-slot posture. All other clauses of the
topology ADR remain in force.

The resource inventory at the default `S=2, E=5` is 16 engine-owned threads
and 12 SQLite connections with an attached provider, at most 128 admitted
projection rows, and 20 waiting embed requests. This is a bounded capacity
change, not a provider throughput guarantee. Explicit `N>1` already permits
concurrent calls to the shared provider; the new default makes that permission
the ordinary open behavior.

## Acceptance evidence

The retained historical D27 entry is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-entry-post-reboot/receipt.json`
(SHA-256 `74c5ee0b43188dd2f2af138eb84dd67aa281034017693e095c82327c6dadc129`).
The five-worker comparison is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-default5-retry-ad31c3a61/receipt.json`
(SHA-256 `29e5206e02751ac83d2a28fa5cf9b85bf98e164cc3270c0cdf88cef5a5b4d185`).
The retained failed three-worker proof is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-default3-proof-2a62b14c7/raw-output.jsonl`;
all six attempts in that bundle are environment-valid and the unchanged
comparison reports `projection_heavy direct_embed p95 above latency ceiling`.
The exact AC-081 campaign summary is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/ac081-default5-ad31c3a61/summary.txt`
(SHA-256 `00a4313dc52ae5cb9c1939e6c220901581d530adad691467c63ce09c8d1084ab`).

On exact five-worker source `ad31c3a61`, AC-011a/b, AC-017, AC-018, AC-029,
AC-072, and AC-076 passed unchanged. The CUDA EU7 run measured AC-073
mixed-tail stress p99 418 ms within its same-run 491 ms bound. Its combined
selector exited 101 on the unchanged AC-075 recall assertion (0.772,
95% CI high 0.798 below 0.90), the same result already retained by Slice 85
as `superseded-by-tc5`. The exact named-selector report is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/named-selectors-default5-ad31c3a61/report.txt`
(SHA-256 `1c83338b90dcbf6c870f108f665bf704cb462970a68ec84da18a9a253f612230`).
The combined selector is not reported as PASS. A reviewed checkpoint receipt
must bind the AC-073 stress evidence and the retained AC-075 outcome.

Installed-binding parity, independent code review, and Terra verification
remain before acceptance and the runtime checkpoint. No structural Phase 3
source move may begin before that checkpoint passes.
