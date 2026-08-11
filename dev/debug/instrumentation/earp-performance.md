# EARP and performance instrumentation runbook

## Purpose

Use this runbook when an EARP quality result needs its own observed cost, or
when a developer needs linked performance evidence for the exact same
workload. It covers local developer instrumentation only. It does not set a
performance promise, modify release gates, or replace the scale protocol.

EARP answers whether a pinned workload retrieves the intended evidence. Its
cost sidecar answers what that single execution observed. The independent
performance runner repeats the workload only after it verifies the quality
artifact graph and its workload manifest.

## Evidence model

```text
quality configuration
       |
       v
EARP quality run ----> earp.result.v1.json
       |              earp.per-query.v1.jsonl
       |              config.resolved.yaml
       |              earp.workload-manifest.v1.json
       |              earp.observed-cost.v2.json
       v
record.json advertises every artifact with SHA-256
       |
       v
performance runner re-verifies graph and immutable workload
       |
       v
performance.earp.v1.json (cells, raw samples, invalidity, summary)
```

The manifest, not `record.json` configuration or CLI options, is the source of
the complete resolved workload and predeclared performance plan. A stale,
missing, altered, or legacy manifest is refused before execution.

## Instrumentation catalogue

### One-run observed cost

`earp.observed-cost.v2.json` is emitted beside a quality result. The flat form
contains:

- `phases_ms`: observed open, ingest/write, and query intervals when available.
- `counts`: accepted documents, query count, and result count when available.
- `storage`: SQLite main-database, WAL, and SHM byte sizes at the collection
  point.
- `query_samples`: one result per executed query with `wall_ms`, result count,
  and a complete/failed outcome.
- `unavailable`: a typed `{code, message}` reason for every canonical
  checkpoint that was not observed. Zero is not used to mean unavailable.
- `provenance`: captured candidate, cleanliness, command, lockfile, toolchain,
  device, and fixtures, or a typed reason that each unavailable item could not
  be observed.

Comparison campaigns preserve one such document for each named arm under
`arms`. Never pool timings from different arms into a single distribution.

This sidecar is a one-run observation. It can identify which phase warrants
investigation, but it cannot establish p95/p99, QPS, process-cold behaviour,
or a portable cost envelope.

### Linked repeated performance

`performance.earp.v1.json` records a performance execution linked to the
quality workload. It contains:

- quality and execution workload digests plus either
  `same_candidate_reexecution` or `cross_candidate_reexecution`;
- the exact `fresh_store` or `fresh_store_warm_query` treatment matrix and
  repetition count declared by the parent manifest;
- one complete or invalid cell for every planned treatment/repetition;
- raw phase/count samples, treatment witnesses, and execution provenance;
- typed invalidity retained with the failed cell rather than silently dropped;
- summaries only for treatments whose entire declared matrix is complete.

`fresh_store_warm_query` means a fresh database followed by an unmeasured query
warm-up. It is **not** process-cold. Process-cold needs a separate subprocess
and OS-cache witness before it can be offered or compared.

## Before running

1. Work from a clean checkout with the native Python binding built by the
   supported repository workflow. Do not copy an extension between worktrees.
2. Pin the corpus snapshot, corpus-data root, gold file/hash, qrels version,
   retrieval call, projections, embedder/device, and query knobs in the
   quality configuration.
3. Put output in a new experiments root. Never write over a prior run and
   never retrofit a workload manifest onto a historical artifact.
4. Validate a candidate EARP configuration before execution:

   ```bash
   PYTHONPATH=src/python .venv/bin/python -m eval.earp.cli validate path/to/config.yaml
   ```

5. Run real-corpus jobs in a persistent terminal (`tmux`, a normal terminal,
   or an equivalent session manager). In the Codex execution environment, a
   non-TTY command wrapper has a short lifetime and will kill a long query
   loop; use a TTY-backed session and poll it instead. This is a launcher
   constraint, not a native-engine failure.

## Produce a quality result and observed-cost sidecar

There is currently no generic EARP quality-run CLI. A campaign driver invokes
`run_diagnostic` or `run_characterization` with the pinned inputs. Those
functions write the quality record, workload manifest, and observed-cost
sidecar together. The fixture-style call sites are in:

- `src/python/tests/earp/test_diagnostic_runner.py`
- `src/python/tests/earp/test_characterization_observed_cost.py`

For a real corpus-scale characterization, the caller must provide the same
complete pinned inputs as `run_characterization`: corpus root, snapshot path,
gold path and SHA-256, corpus hash, qrels version, output root, experiment
name, UTC timestamp, and the K ladder. Do not reconstruct them from an older
`record.json`; start a current quality run.

After a successful run, inspect the durable result rather than relying on
console output:

```bash
RUN_DIR="$EXPERIMENTS_ROOT/runs/$QUALITY_RUN_ID"
jq '{verdict, metrics, artifacts}' "$RUN_DIR/record.json"
jq '{phases_ms, counts, storage, unavailable, provenance}' \
  "$RUN_DIR/earp.observed-cost.v2.json"
```

The 2026-08-11 local full-corpus witness, written outside the repository, had
10,506 documents and 4,597 queries. It recorded approximately 2.13 seconds of
write time, 219.8 seconds of query time, 4,597 query samples, and a 120 MB
SQLite database. Those values are a machine-specific diagnostic baseline, not
a supported performance claim.

## Run linked performance evidence

The performance CLI accepts a quality run that already has a valid workload
manifest. Supply **exactly** the plan stored in that manifest:

```bash
PYTHONPATH=src/python .venv/bin/python -m eval.performance.cli characterization \
  --experiments-root "$EXPERIMENTS_ROOT" \
  --quality-run "$QUALITY_RUN_ID" \
  --repetitions 1 \
  --treatments fresh_store,fresh_store_warm_query
```

Use `diagnostic` instead of `characterization` only for a diagnostic quality
run. The command rejects:

- a quality run from the wrong campaign;
- a missing, malformed, legacy, or digest-mismatched manifest;
- any corpus, gold, projection, embedder, device, query call, or knob drift;
- CLI treatments or repetitions that differ from the manifest's predeclared
  plan; and
- incomplete or fabricated execution provenance.

The standard quality writer currently predeclares one repetition and marks it
`descriptive_nonclaim`. Consequently the command above is useful for linked
diagnosis, not percentile or throughput claims. A 20+ repetition performance
campaign requires a quality writer/driver that predeclares that exact plan; do
not edit the manifest JSON to bypass this rule. Exposing that campaign-plan
selection is follow-up work, not an operator workaround.

## Inspect and compare evidence

For a single quality run, inspect the observed sidecar's phase timings and its
per-query samples. For linked performance, inspect cells first:

```bash
PERFORMANCE_RUN_DIR="$EXPERIMENTS_ROOT/runs/$PERFORMANCE_RUN_ID"
jq '{relation, plan, cells, summary}' \
  "$PERFORMANCE_RUN_DIR/performance.earp.v1.json"
```

Only compare measurements when corpus/gold digests, workload, candidate,
device, build, and treatment are equivalent. A `cross_candidate_reexecution`
artifact is useful diagnostic evidence but is not a same-candidate cost or
quality comparison. A treatment with an invalid cell has no eligible summary.

Treat p50/p95/p99 as descriptive order statistics only when the predeclared
sampling protocol authorizes them. One repetition must never be reported as a
tail-latency estimate.

## Failure triage

| Symptom | Meaning | Action |
| --- | --- | --- |
| Process stops near 30 seconds without `finally` cleanup | Short-lived non-TTY launcher killed the job. | Re-run in a persistent TTY/session; do not change FathomDB code. |
| Import cannot find `_fathomdb` | The current Python environment lacks the built binding. | Use the supported repository build/install workflow; do not copy an extension into a worktree. |
| Quality run is refused by performance CLI | Manifest/artifact graph, campaign, or predeclared plan is invalid. | Read the error, verify digests, and create a new current quality run if needed. |
| Performance cell is `invalid` | Execution did not provide comparable evidence. | Preserve the artifact and diagnose its typed invalidity/provenance; do not discard the cell or summarize the treatment. |
| Field is under `unavailable` | The collector did not observe it. | Keep the typed reason; do not substitute zero or invent a value. |
| One arm appears faster in a comparison campaign | Arms retain separate observations. | Compare arm-local samples only; do not pool them. |

## Developer checklist

- [ ] Configuration validates and pins all corpus/gold/query inputs.
- [ ] Run is launched in a persistent session if it can exceed the launcher
      timeout.
- [ ] Quality record advertises manifest, result, config, per-query, and
      observed-cost artifacts with SHA-256 digests.
- [ ] Missing observation/provenance fields have typed reasons.
- [ ] Performance CLI plan exactly matches the parent manifest.
- [ ] Invalid cells remain durable and excluded from summaries.
- [ ] Any stated comparison names its candidate, environment, corpus/gold,
      treatment, sample count, and claim class.

## Related documents

- [Catalogue index](README.md)
- [`../../plans/performance-and-hardening-initiative.md`](../../plans/performance-and-hardening-initiative.md)
- [`../../plans/0.8.24-retrieval-performance-evidence-spec.md`](../../plans/0.8.24-retrieval-performance-evidence-spec.md)
- [`../../../experiments/README.md`](../../../experiments/README.md)
- [`../../../scripts/perf-experiments/README.md`](../../../scripts/perf-experiments/README.md)
