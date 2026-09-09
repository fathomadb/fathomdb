---
title: 0.8.25 Slice 72 — installed CE profile and generic preflight design
status: APPROVED
design_version: 2
target_release: 0.8.25
depends_on: 71
---

# Slice 72 design

## Boundaries

This slice changes release tooling and adds a measurement harness. It is not a
product-feature slice. Existing release-state authority, CE scoring/ranking,
device policy, pinned model, Python API, and shipping feature defaults remain
unchanged unless a focused RED proves a defect.

## Generic preflight

### State lifecycle

The selected live state adds:

```json
"active_ref": "refs/heads/release/0.8.25"
```

For any release `R`, the only valid value is
`refs/heads/release/R`. This is a local active-development baseline; it makes no
remote reachability or release-completion claim.

The existing optional completion contract remains separate:

- no `completion`: use the validated `active_ref`;
- `main_integration=PENDING`: use `origin/release/R` after validating the exact
  ref and resolving it locally;
- `main_integration=COMPLETE`: require the completion ref to be reachable from
  `origin/main`, then use `origin/main`.

Preflight calls the existing tracked-only `scripts/release-current.py` selector.
It does not scan untracked files, choose the highest version, or infer a release
from the current branch name.

### Validation and worktree algorithm

For state-dependent options (`--worktree` or `--expect-closed`):

1. require exactly one live tuple from `release-current.py`;
2. parse its state and require exact filename/release, tracked board, state
   `plan`, and active/completion ref shapes;
3. resolve the lifecycle baseline to a commit;
4. require the target worktree HEAD to equal or descend from that baseline; and
5. when a dependency is requested, find exactly one numerically equal ladder
   entry, require a closed status and resolving SHA, then require that commit to
   be an ancestor of the lifecycle baseline and target HEAD. Target HEAD means
   the `--worktree` HEAD when supplied, otherwise the invoking checkout's HEAD.

`--plan` remains accepted for callers. When supplied with `--expect-closed`, its
normalized repository path must equal `state.plan`; its prose is never searched
for closure. A missing `--plan` is acceptable because state is authoritative.

Plain health checks that request neither worktree freshness nor dependency
closure need no active-release facts. Landing mode retains its existing linked-
worktree and subsequent repository gates.

The final one-line summary adds the selected release, state file, baseline ref,
and dependency SHA when applicable, making the decision auditable without
changing its pass/fail shape.

### Preflight files

- `dev/plans/release-state-0.8.25.json`: explicit `active_ref`.
- `scripts/preflight.sh`: selector integration, validation, baseline, and
  dependency checks.
- `scripts/tests/test_preflight_landing.sh`: RED/GREEN behavior fixtures.
- `scripts/tests/test_release_current.sh`: selector regression fixture only if
  integration exposes an uncovered selector case.

No generalized release-state schema rewrite is needed.

## Installed CE profile

### Artifact and process isolation

Profile Python wheels because one installed binding exercises both the public
standalone and end-to-end Engine paths without duplicating Slice 75's SDK matrix.
Build two wheels per commit:

- CPU: `pyo3/extension-module,default-reranker`;
- CUDA: `pyo3/extension-module,rerank-cuda`.

Each wheel is installed into a fresh external virtual environment. The worker
fails unless `fathomdb.__file__` and the native extension resolve beneath that
environment and outside every source checkout. Network is disabled during
measurement. The pre-existing cache is staged read-only and its three pinned
file hashes must match the manifest.

Candidate and baseline use separate source worktrees and build directories, but
the same compiler, Python, host, CPU affinity, environment, cache bytes, and
selected GPU. The candidate SHA is recorded only after profile implementation
and its manifest are committed.

### Fixed workload

The manifest owns one six-passage Berlin fixture derived from the existing CE
equivalence test:

- query: `How many people live in Berlin?`;
- passage IDs and input order fixed in the manifest;
- `rerank_depth=6`, `pool_n=6`, `alpha=1.0`;
- one off-topic passage begins above the population passage so CE activity must
  visibly reorder it; and
- 20 timed calls form each steady repetition.

The standalone path passes those exact passages to `fathomdb.rerank`.

The Engine path writes the same bodies as fixed `doc` records, drains, and runs
`Engine.search` with the same query and rerank parameters. It requires at least
two CE-scored hits, finite non-degenerate scores, stable IDs/order within the
cell, and no expected result loss. The standalone canary owns the exact known
Berlin reorder; the end-to-end path proves installed search invokes CE without
pretending its FTS candidate order is the standalone input order.

### Repetitions and measurements

For every `(commit, device, path)` cell:

- three fresh processes record import, Engine open where applicable, and first
  CE call/model load;
- five fresh steady processes each warm once, then time 20 calls;
- each steady repetition records per-call durations, p50/p95/p99, throughput,
  peak RSS, output IDs, and CE scores; and
- CUDA repetitions additionally record selected UUID, worker PID, and observed
  matching-PID VRAM while the timed process is alive.

The runner pins common thread variables to `1` and records CPU affinity. It uses
one selected RTX 3090 with no unrelated compute process when feasible; any
contention is disclosed and the paired baseline/candidate run is repeated, not
adjusted statistically.

The worker measures only the public operation with `time.perf_counter_ns` and
stores integer nanoseconds. Import, database creation/open, fixture write,
projection drain, model warmup, JSON serialization, and CUDA sampling remain
outside steady-operation timings and are reported separately where applicable.
For sorted durations `d` of length `n`, percentile `q` uses nearest rank
`d[ceil(q*n)-1]` for q in `{0.50, 0.95, 0.99}`. Milliseconds are nanoseconds
divided by `1_000_000`; throughput is `n * 1_000_000_000 / sum(d)`. The
validator recomputes these values and its unit fixtures include hand-calculated
samples.

### Correctness and comparison rules

A cell fails on any exception, skip, identity fallback, non-finite or constant
CE scores, missing ID, changing order, or source import. CPU must resolve CPU;
CUDA must be forced and resolve CUDA with a matching allocation witness.

For the candidate, CPU and CUDA standalone rank order must be identical.
Per-ID normalized CE scores must satisfy absolute difference `<= 1e-2`, matching
the existing backend-equivalence tolerance. The Engine result order must be
stable per device; differing CPU/CUDA order fails unless every affected score
is within the tolerance and the manifest explicitly identifies a score tie.
The initial manifest declares no ties.

For each device/path, calculate the median of the five repetition p95 values.
Candidate passes when it is `<= baseline * 1.10`. The 10% boundary is a
regression guard, not a claim about absolute latency. First-call time, import,
open, throughput, RSS, VRAM, wheel size, and model size are reported but do not
gain new thresholds.

### Evidence layout and schemas

Retain under `dev/plans/runs/0.8.25-slice-72/`:

- immutable manifest and its SHA;
- raw JSON for every process;
- artifact/build identity and command log;
- validated aggregate receipt; and
- concise comparison table.

The validator accepts only the manifest's complete cell set, exact repetition
counts, known keys/schema version, and matching hashes. It independently
recomputes percentiles, throughput, tolerances, and regression ratios from raw
durations. Diagnostic or incomplete results may be retained but cannot produce
a PASS receipt.

## TDD, review, and verification

Commit failing preflight and receipt-validator tests before their
implementations. Test files remain unchanged during each GREEN step. Existing
CE tests are read-only oracles unless a separately demonstrated defect requires
a new product RED.

Independent reviewers check design first, then the implementation diff. A
separate verifier audits commands, hashes, raw results, recomputed verdicts,
and focused test outputs without rerunning the campaign. Full regression and
release closure remain Slice 75 work.
