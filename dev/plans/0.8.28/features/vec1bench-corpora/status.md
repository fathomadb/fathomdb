---
title: FathomDB 0.8.28 vec1bench corpora status
status: HARNESS COMPLETE; FULL-SCALE RUN DEFERRED
target_release: 0.8.28
observed_on: 2026-10-11
---

# Vec1bench corpora status

Plan context: [0.8.28 draft scope](../../../0.8.28-draft-scope.md)
(D28-12, vector latency attribution). The
[vec1bench corpus and test plan](../../../../vector-bench-corpora/vec1bench/README.md)
defines the sources, preparation, measurement boundary, and run commands.
This owner-requested harness supplies a future measurement input; it does not
close D28-12 or the 0.8.28 release qualification.

## Completed

- Four revision- and SHA256-pinned HDF5 sources are registered: SIFT1M,
  ImageNet CLIP, Landmark DINO, and AG News MXBAI. Acquisition requires an
  explicit network flag; offline preparation validates HDF5 layout and exact
  neighbors, then seals the converted files with SHA256.
- Four ignored, full-scale FathomDB engine tests ingest the complete source
  populations and report vector-stage recall@1, recall@10, and separate
  top-1/top-10 latency distributions. A small real-engine fixture covers
  retrieval and two database reopens. Commits: `0d2cf4f4f`, `01f7272af`.
- An independent read-only review found and then verified the fix for a
  reopen-time dense-retrieval failure; its final verdict had no open finding.

## Verification

- Focused checks passed: six Python acquisition/preparation contract tests,
  Ruff, Rust formatting, targeted Clippy, the real-engine fixture, scoped
  Markdown lint, and a synthetic HDF5 conversion with real `h5py` and checked
  output seals. Invoking an opt-in full-scale test without its corpus fails
  explicitly.
- The full `./scripts/agent-verify.sh` run passed lint, typecheck, all strict
  security checks in a ptrace-capable executor, and the serial Rust workspace
  suite. The test stage reported 194 suites passed, one failed, and two
  skipped. The sole failure is the existing Slice 135 harness's reference to
  a missing external wheel under `/tmp/slice135-current-wheel-224e44/`;
  neither that test nor its protocol was changed by this work. The full gate
  is therefore **not green**.

## Deferred by owner direction

The four multi-gigabyte source files were not downloaded. No full-scale
FathomDB or vec1 performance run was started, and no dataset recall or latency
result is claimed. A later authorized run can follow the linked corpus plan on
external scratch storage and record results on a matched host.
