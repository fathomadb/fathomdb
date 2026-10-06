---
title: Slice 110 independent design review
status: APPROVED_FOR_IMPLEMENTATION
target_release: 0.8.27
---

# Slice 110 design review

The first independent review used `gpt-6.1-sol` high. It rejected the initial
direct bounded ThreadsafeFunction design: pinned napi-rs 2.16.17 leaks a boxed
record on failed enqueue or null-environment shutdown, and its stock callback
path turns a listener throw into a fatal exception. It also required a
generation gate, bounded JS draining, an exact payload, and correct engine
close outcomes.

Subsequent `gpt-6-sol` high reviews required protection of the raw N-API
handle against final-release races, a linearizable pending-wakeup transition,
synchronous JS-facing close entry, and a null-environment callback guard. The
accepted successor ADR now specifies an owned 4096-record queue, raw null-data
wakeups, a short handle mutex, 64-record JS turns, explicit failure/teardown
behavior, and those tests. The final read-only review returned **PASS for
implementation readiness** with no remaining design blocker. This is not a
runtime or artifact qualification claim.

The review also confirmed the `spawn_blocking` authority, the one-Engine
identity, the separate subscriber correction, and the clean thin-main plus
platform-pair packaging scope. The required cross-platform rows remain exit
gates; their availability is not inferred from this design verdict.

## As-built architecture documentation review

After implementation, a separate `gpt-6-sol` high read-only design review
compared the new TypeScript addendum in the active
[architecture](../../../../design/fathomdb-data-plane-architecture-v2.md),
the [binding](../../../../design/bindings.md) and
[lifecycle](../../../../design/lifecycle.md) designs, and this slice design
against the code and accepted subscriber ADR. It found one wording issue:
the queue caps record count, not diagnostic payload bytes. That wording was
corrected. The reviewer found no other architecture or design mismatch in
the additions. This review does not qualify the remaining platform rows.

## Merged Tegra design documentation review

After the Tegra allocator and early-`cuInit` fix merged, a `gpt-6-sol` high
read-only design review checked the TypeScript interface, public embedder
guide and Tegra platform reference against the as-built probe record. The
design keeps the early Node registration call because a later fragmented V8
heap can leave no address-space range for `cuInit`. The aarch64 Linux allocator
fallback addresses the separate default-pool range failure; it does not
guarantee stream-ordered allocation. Those causes and limits remain distinct
in the platform reference.

The review found that the out-of-memory message hint reads a process-wide
slot per probe kind. A concurrent probe of the same kind can replace the
record before a refusal is formatted, so the hint is best effort and can be
absent or suggest an unrelated `cuInit` failure. The documentation now
describes that limitation, makes late-import failure conditional, and names the
reranker error code. The stable error code, kind and no-CPU-fallback rule
remain the contract. The reviewer found no further material mismatch after
those corrections.
