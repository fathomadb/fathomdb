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
to the quality run. `fresh_store` and `fresh_store_warm_query` are separate treatments.
`fresh_store` means a newly created database in the same process; it makes no
process-cold or OS-cache-cold claim. `fresh_store_warm_query` performs one
unmeasured query only after the fresh database is opened and written; it does
not warm ingestion. The runner verifies the saved manifest and its input
digests before execution, and does not accept duplicate corpus, query, or knob
flags.
