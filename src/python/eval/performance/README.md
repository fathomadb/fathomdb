# EARP-linked performance runner

This developer-side runner creates repeated performance evidence from an
existing EARP quality run. It reads the recorded resolved configuration and
candidate provenance; operators do not retype query knobs.

For an EARP diagnostic run:

```bash
python -m eval.performance.cli diagnostic \
  --experiments-root experiments \
  --quality-run <earp-quality-run-id> \
  --repetitions 5
```

For a corpus-scale EARP characterization run, use the same quality-run ID:

```bash
python -m eval.performance.cli characterization \
  --experiments-root experiments \
  --quality-run <earp-quality-run-id> \
  --repetitions 5
```

The runner records an independent `performance.earp.v1.json` artifact linked
to the quality run. `fresh_store` and `warm` are separate treatments.
`fresh_store` means a newly created database in the same process; it makes no
process-cold or OS-cache-cold claim. It does not reinterpret a one-run EARP
observation as percentile or support evidence. It consumes the saved resolved
configuration; it does not accept duplicate corpus, query, or knob flags.
