---
title: Scheduler Subsystem Design
date: 2026-04-30
target_release: 0.6.0
desc: Projection worker dispatch, bounded admission, retry behavior, and shutdown ordering
blast_radius: projection scheduler; REQ-015, REQ-016, REQ-027, REQ-029, REQ-030, REQ-055
status: locked
---

# Scheduler Design

This file owns projection worker admission, capacity waits, retry policy, and
shutdown. The accepted runtime topology is
[`ADR-0.8.27-engine-owned-runtime-topology`](../adr/ADR-0.8.27-engine-owned-runtime-topology.md).

## Current connection and generation boundary

`scheduler_runtime_threads` starts exactly that many projection workers and
worker-owned SQLite connections (default two, accepted `1..=64`). The
dispatcher admits at most `scheduler_runtime_threads * 64` active plus queued
rows; each scan has the same bound. Workers compute and publish through their
own SQLite connections, not the primary caller-write connection. The shared
`commit_gate` serializes publication transactions with the primary writer.
Every job carries its serving generation and revalidates membership and
generation before publication. Stale work is discarded; pending current work
is rediscovered from durable state.

Canonical node and edge writes, including derived-edge actuation, use this
same bounded machinery. Receipt pending cursors create no separate scheduler.
Projection workers submit embedding to the separate engine-owned embed
dispatcher before taking `commit_gate`. Its `4 * embedder_pool_size` waiting
queue is a distinct bound. Full embed admission or queued expiry waits for
capacity and leaves durable work pending; it does not spend or reset a
provider-failure retry. Close wakes these capacity waits.

## Fixed retry policy

Started provider failure or timeout and invalid output use the existing
bounded projection retry policy:

- 3 retries maximum
- backoff schedule `1s`, `4s`, `16s`

These delays are engine constants, not `Engine.open` knobs. Exhaustion records
a durable failed terminal; successful current work can advance readiness.
Operator workflow is inspection plus the explicit CLI regenerate path
(`recover --rebuild-projections`). `drain` can return after terminal work is
idle; capacity starvation leaves work pending and can make bounded drain fail.
