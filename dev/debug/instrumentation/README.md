# Developer instrumentation catalogue

This directory is the developer-facing catalogue for instrumentation that
explains FathomDB behaviour without changing its public API or turning one
developer-machine observation into a support claim.

| Area | Entry point | What it records | Primary output |
| --- | --- | --- | --- |
| EARP one-run cost | `eval.earp.characterize.execute_arm` and `eval.earp.runner` | open, ingest/write, query time, counts, SQLite sizes, per-query timings, and typed unavailable fields | `earp.observed-cost.v2.json` |
| EARP arm comparison | `eval.earp.comparison` | one independent observed-cost document per arm; it does not pool arm timings | `earp.observed-cost.v2.json` with `arms` |
| EARP provenance and artifact graph | `eval.earp.writer` | quality result, resolved config, workload manifest, SHA-256 artifact references, and collision protection | `record.json` plus quality artifacts |
| Linked repeated performance | `eval.performance.earp_adapter` and `eval.performance.cli` | declared treatments, cells, raw samples, invalidity, execution provenance, and eligible summaries | `performance.earp.v1.json` |

The [EARP and performance runbook](earp-performance.md) describes how to
produce, inspect, and interpret those artifacts.

## Capability boundary

This is a catalogue of what the current instrumentation actually observes. A
blank cell is deliberately not inferred from a nearby measurement.

| Question | Current capability | What it provides | What it does not provide |
| --- | --- | --- | --- |
| Which retrieval requests are slow? | **Partial** | One quality run records a `wall_ms`, result count, and outcome for every query ID. A developer can sort those samples to identify slow query IDs and compare them within that exact run. | No automatic outlier report, query text, query-plan/explain capture, CPU time, or attribution below the whole query call. |
| Which phase is slow? | **Yes, one run** | Open, ingest/write, and total query-wall intervals; accepted/query/result counts; SQLite main/WAL/SHM byte sizes. | No engine-internal span breakdown, queue depth, projection/embedding separation in every campaign, or cross-host comparison by default. |
| Is memory growing or leaking? | **No** | SQLite file/WAL/SHM sizes are storage observations. | No process RSS, heap/allocation profile, Python/Rust allocator telemetry, GPU memory, cgroup memory events, or leak trend. |
| Is there semaphore, lock, or SQLite contention? | **No, except surfaced failures** | A query failure can be retained as an error/invalid cell. | No lock-wait duration, semaphore permits, queue depth, SQLite busy/retry count, blocking graph, or contention timeline. |
| Is there a race condition? | **No** | The repeated runner keeps an invalid execution cell rather than hiding it. | No concurrent workload scheduler, interleaving trace, race detector, deterministic replay of schedules, or thread/task timeline. |
| Can it characterize general performance? | **Partial** | Per-query wall-time samples plus phase/count/storage observations; linked repeated runs can retain raw samples and eligible descriptive summaries. | No CPU profiling, sampling profiler, flamegraph, hardware counters, `perf` integration, tracing spans, or process-cold witness. |
| Does it capture stack traces? | **No** | Typed error/invalidity code and message where an exception is surfaced. | No Python traceback, Rust backtrace, native crash dump, symbolization, or debugger attachment. |

For the current collectors, “not observed” is represented as a typed
unavailable reason in the artifact. It must not be reported as zero, healthy,
or absent.

## Scope boundary

- EARP remains the retrieval-quality evaluator. Its sidecar records the cost
  observed during that one quality execution.
- The independent performance runner repeats an immutable EARP workload; it
  does not accept a second corpus/query/knob configuration from the operator.
- A one-run sidecar is descriptive evidence, not a latency SLO, QPS claim, or
  statistically defensible tail-latency result.
- Performance and quality are linked by a manifest and artifact digests, not
  by matching names or an operator's recollection of command-line options.

## Sources of truth

- `src/python/eval/earp/schema/earp.observed-cost.v2.schema.json`
- `src/python/eval/earp/schema/earp.workload-manifest.v1.schema.json`
- `src/python/eval/performance/schema/performance.earp.v1.schema.json`
- [`../../plans/performance-and-hardening-initiative.md`](../../plans/performance-and-hardening-initiative.md)
- [`../../plans/0.8.24-retrieval-performance-evidence-spec.md`](../../plans/0.8.24-retrieval-performance-evidence-spec.md)

When this catalogue conflicts with a versioned schema, the schema and its
writer behaviour are authoritative; update this catalogue in the same change.
