# queue-sim — admission and reader-dispatch simulator

A custom discrete-event simulator (a `heapq` event loop, no framework) of the
`fathomdb-mcp` admission line and the engine's reader dispatch. It models
per-store admission slots, a FIFO waiting line, per-call deadlines, early shed,
reader-pool dispatch policies (P0 round-robin, P1/P2 idle-first, a shared FIFO),
one GPU for hybrid embeds, and the writer/projection contention. Service times
are calibrated from the live server (`measure.py`, `probe.py`).

`REPORT.md` holds the findings. They feed
`dev/design/agent-access/queue-admission.md`.

## Files

- `sim.py` — the simulator. `step.py`, `fuzz.py`, `grids.py`, `incident.py` —
  the studies. `runner.py` — resumable multi-process driver. `analysis.py` —
  builds `results/tables.md` from the study CSVs.
- `measure.py`, `probe.py`, `probe_summary.py` — live measurements against a
  staged `fathomdb-mcp` binary. `mcp-config.json` and `probe-config-cap*.json`
  are their server configs. Point `roots[0].path` at your store directory
  before running them.
- `rtc.py`, `rtc_analysis.py` — the run-to-completion study (REPORT § 12) and its tables and decision
  rules; `rtc_pairing.py` follows the old rule's timeouts call by call;
  `PREREG-run-to-completion.md` fixes the study's arms, metrics and rules before the run.
- `test_sim.py`, `test_fuzz.py`, `test_runner.py`, `test_run_limit.py`, `test_rtc.py`,
  `test_rtc_analysis.py`, `test_rtc_pairing.py` — the tests.
- `research-scratch/` — the cited literature inputs (`REPORT.md`, `SOURCES.md`).
- `results/` — the committed summaries: `tables.md`, `service_times.csv`,
  `burst_probe*.csv`, `burst_probe_summary.md`, `incident_summary.txt`, the small
  grid/step/incident CSVs, and `incident-transcripts/`.

## Run the tests

```bash
python -m pytest dev/experiments/queue-sim
```

## Regenerate results

Run from this directory. Each study writes `results/<name>.csv` and
`results/<name>.jsonl` and resumes from the JSONL if interrupted.

```bash
python fuzz.py main 2000     # 28,000 runs
python fuzz.py real 500      # 7,000 runs
python fuzz.py thresh 810    # 32,400 runs
python grids.py
python step.py
python incident.py
python analysis.py           # rewrites results/tables.md
python rtc.py 500 2000       # 112,320 runs, ~4 h on 22 processes
python rtc.py lockup         # 17,280 runs: rf_arrival_free for the lock-up table
python rtc_pairing.py        # 3,000 runs, ~5 min: results/rtc_pairing.md
python rtc_analysis.py results/rtc.csv   # rewrites results/rtc_tables.md
```

Live measurements need a staged binary and the LoCoMo `locomo10.json` file:

```bash
python -I measure.py <binary> mcp-config.json <locomo10.json> results/service_times.csv
python -I probe.py <binary> <locomo10.json> results/burst_probe.csv
python probe_summary.py results/burst_probe.csv
```

The raw fuzz runs (`fuzz_*.csv`, `rtc.csv`, all `*.jsonl`, about 170 MB before the run-to-completion study) and the
superseded pre-review round are not committed. `analysis.py` skips any table
whose input CSV is missing, so regenerate the fuzz runs before rebuilding
`tables.md`.
