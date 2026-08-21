# Experiment-harness agent instructions

These instructions apply to all work under `experiments/`.

- For a run or harness change that serves PROGRAM, first run
  `./scripts/track-runner.sh check` and read
  `./scripts/track-runner.sh brief <TRACK-ID>` plus the named charter.
- New PROGRAM configurations carry `program_track: <TRACK-ID>` in their typed
  resolved configuration. Do not rewrite historical configs or receipts solely
  to add it.
- Keep `experiments.record.v1` and `experiments.index-row.v1` intact. Each run,
  including a blocked one, writes its safe receipt and appends one index row;
  raw corpus payloads and model output remain external.
- A harness preflight is not authorization for corpus acquisition, model/GPU,
  paid, or external execution. Stop at the declared approval boundary.
