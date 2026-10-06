# FathomDB 0.8.27 Slice 90 evidence

This is the retained host-local evidence for the completed Slice 90 runtime
checkpoint and final candidate. It was originally under
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/`.
Historical receipts and logs retain that execution path; the checkpoint gate
maps it to this durable directory when revalidating artifacts.

| Item | Location |
| --- | --- |
| D27 historical entry | [`d27-entry-post-reboot/`](d27-entry-post-reboot/) |
| D27 strict candidate pass | [`d27-v2-candidate-2e94aaf4f-retry/`](d27-v2-candidate-2e94aaf4f-retry/) |
| AC-073 sealed execution | [`ac073-2e94aaf4f/`](ac073-2e94aaf4f/) |
| Stage-2 evidence manifest | [`stage2-gates-2e94aaf4f/stage2-evidence-manifest.json`](stage2-gates-2e94aaf4f/stage2-evidence-manifest.json) |
| Final candidate manifest | [`final-1398c821d/final-evidence-manifest.json`](final-1398c821d/final-evidence-manifest.json) |

The bound checkpoint and receipt hashes are in
`dev/plans/release-state-0.8.27.json` and
`dev/plans/0.8.27/features/slice-90/runtime-performance-qualification.md`.
From a current release checkout, run `python3 scripts/check-runtime-checkpoints.py`
to validate them and the retained external bundles. Do not regenerate or edit
the historical evidence to make a failed check pass.
