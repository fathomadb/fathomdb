---
title: Scheduler Subsystem Design
date: 2026-04-30
target_release: 0.6.0
desc: Projection job dispatch, backpressure, retry behavior, and shutdown ordering
blast_radius: projection scheduler; REQ-015, REQ-016, REQ-027, REQ-029, REQ-030, REQ-055
status: locked
---

# Scheduler Design

This file owns projection job spawn policy, queue/backpressure behavior, retry
policy, and the ordered shutdown path that cooperates with the writer thread.

## Current connection and generation boundary

Workers compute and publish through projection-owned SQLite connections rather
than the primary caller-write connection. The shared `commit_gate` totally
orders worker publication transactions with the one-write-transaction
invariant. Every job carries the serving generation captured at dispatch and
revalidates membership/generation before publication; a stale job is discarded
and current pending work is rediscovered.

Canonical node and edge writes, including derived-edge actuation, use the same
bounded queue/backpressure and retry machinery. Receipt pending cursors do not
create a separate scheduler or change the retry owner.

## Fixed retry policy

0.6.0 uses one bounded retry policy for projection jobs:

- 3 retries maximum
- backoff schedule `1s`, `4s`, `16s`

These values are engine constants in 0.6.0, not `Engine.open` knobs. Operator
workflow is inspection plus the explicit regenerate path
(`recover --rebuild-projections`), not per-deployment retry tuning.
