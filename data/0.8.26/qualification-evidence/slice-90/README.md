# FathomDB 0.8.27 Slice 90 evidence

This is the retained host-local evidence for the completed Slice 90 runtime
checkpoint and final candidate. It was originally under
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/`.
Historical receipts and logs retain that execution path; the checkpoint gate
maps it to this durable directory when revalidating artifacts.

| Item | Location |
| --- | --- |
| D27 historical entry | `d27-entry-post-reboot/` |
| D27 strict candidate pass | `d27-v2-candidate-2e94aaf4f-retry/` |
| AC-073 sealed execution | `ac073-2e94aaf4f/` |
| Stage-2 evidence manifest | `stage2-gates-2e94aaf4f/stage2-evidence-manifest.json` |
| Final candidate manifest | `final-1398c821d/final-evidence-manifest.json` |

## Minimum host-local set for Slice 90 checkpoint revalidation

The current `scripts/check-runtime-checkpoints.py` reads these 12 files when
the retained bundle is present. Together they contain **81,701,383 bytes
(77.92 MiB)**. Keep their directory layout and byte contents; the checkpoint
compares receipts and outputs and hashes the three test executables.

| Bundle | Files to keep |
| --- | --- |
| `d27-entry-post-reboot/` | `receipt.json`, `runner.bundle`, `corpus.jsonl`, `raw-output.jsonl`, `target/release/deps/d27_runtime_workload-864291735fc5bf93` |
| `d27-v2-candidate-2e94aaf4f-retry/` | `receipt.json`, `runner.bundle`, `corpus.jsonl`, `raw-output.jsonl`, `target/release/deps/d27_runtime_workload-477d6d10431be3c9` |
| `ac073-2e94aaf4f/` | `eu7.json`, `target/release/deps/eu7_real_corpus_ac-7330e27ac1709bed` |

The D27 executables must be the unique files matching the SHA-256 values in
their respective `receipt.json` files. The AC-073 executable must match the
SHA-256 in the repository-held `ac073-execution.json`. The `.d` files and other
Rust `target/` outputs are not read by this checkpoint. This is the minimum
for **checkpoint revalidation only**; other Slice 90 records, including the
stage-2 and final-candidate manifests above, may be needed for release audit
or a different qualification check. Do not discard those records based on this
list alone.

The bound checkpoint and receipt hashes are in
`dev/plans/release-state-0.8.27.json` and
`dev/plans/0.8.27/features/slice-90/runtime-performance-qualification.md`.
The repository also retains the D27 protocols and candidate receipt, and the
AC-073 stress receipt, execution manifest, EU7 output, and raw run log. Those
committed files are required alongside the host-local set above.
From a current release checkout, run `python3 scripts/check-runtime-checkpoints.py`
to validate them and the retained external bundles. Do not regenerate or edit
the historical evidence to make a failed check pass.
