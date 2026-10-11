---
title: FathomDB 0.8.27 Slice 115 — invalid and superseded attempts
status: RETAINED
---

# Invalid and superseded attempts

| Attempt | Disposition | Retained evidence |
| --- | --- | --- |
| 1, external release build | No measurement. An unpinned external lockfile attempted the network, then offline Cargo could not unpack cached crates inside the sandbox. Using the repository lockfile and an unconfined offline build resolved the setup failure. | [Build diagnostics](attempt1-build.log); no raw sample existed. |
| 2, first real-engine smoke | **Invalid**: the graph null control used an absent seed and returned `GraphSeedUnavailable`; workload exited 101 before raw output. | [attempt.json](attempt.json), [stdout](run.stdout.log), [stderr](run.stderr.log). |
| 3, corrected graph smoke | Semantically successful, superseded when protocol binding, corpus digest and profile controls were tightened. No final characterization uses its samples. | [attempt3 raw](attempt3-raw.json). |
| 4, first full raw candidate | Semantically successful, superseded when control-vs-profile observations and model-projection acceptance were completed. Its p90 order selected erasure; it does not determine the final deep-profile set. | [attempt4 raw](attempt4-raw.json). |
| 5, first profile-backed candidate | Incomplete against the final real-default-model projection rule. Three deep profiles were initially collected concurrently, violating the quiet-host control; those files were preserved and redone sequentially. | [superseded attempt 5](../superseded-attempt5), [concurrent profile artifacts](invalid-profiles). |
| 6, pre-review candidate | Semantically valid under its earlier protocol, then superseded after code review found the projection timer started after write, raw flags could disable model/prestate checks, and `Cargo.lock` was not frozen. Its latency and profiles do not enter final characterization. | [superseded attempt 6](../superseded-attempt6). |
| 7, corrected final candidate | Valid under the corrected write-to-ready timing, unconditional semantic controls and frozen dependency lock. All eight profile-backed paths have operation stacks; fresh open, populated reopen and graph evidence have selected matched controls. | [final receipt](../final). |

An initial GDB/MI graph-evidence collection timed out before the reader fix;
short canonical-write and erasure collections had fewer than three operation
stacks. They were excluded and repeated with an operation-focused loop or
more samples. No failed or superseded timing was pooled into the final summary.
The invalid and superseded GDB transcripts are retained as deterministic
`*.txt.gz` files to keep this record compact. Each profile directory has a
`compression-manifest.json` with SHA-256 for both gzip bytes and the original
text; the companion profile JSON retains the original text digest. The final
candidate's profiles remain plain text for direct receipt validation.
