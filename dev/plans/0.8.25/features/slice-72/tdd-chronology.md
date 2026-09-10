# Slice 72 TDD chronology

- RED was committed at `bc85ac2d`: the release-state and CE-profile contract
  tests failed because their helper and validator did not exist.
- GREEN began at `e7ecda58`: generic release-state preflight, installed-artifact
  construction, the bounded CPU/CUDA runner, and the fail-closed receipt
  validator satisfied the focused tests and independent code review.
- The first installed build exposed venv launcher symlink resolution and missing
  explicit CUDA build inputs. Commits `9cc2cb92`, `3b44b1ef`, and `616803dd`
  preserve the venv launcher and bind the local CUDA toolkit, library path, and
  non-repaired local wheel mode in each artifact receipt.
- The first exact campaign failed only CUDA standalone p95 at ratio `1.176432`.
  Its retained call timings showed periodic observer spikes: the worker ran
  `nvidia-smi` concurrently with timed inference despite the design excluding
  CUDA sampling from timing. Commit `2e14f5ba` moved the allocation sample after
  the timed calls. The paired CUDA rerun passed without changing the 10% limit.

No product CE behavior or release-wheel feature default changed.
