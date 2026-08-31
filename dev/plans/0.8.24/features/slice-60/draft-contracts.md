---
title: 0.8.24 Slice 60 — approved contract refinements
status: APPROVED
target_release: 0.8.24
---

# Slice 60 contract refinements

## Disposition of allocated drafts

The Slice 3 drafts for NEED-016, REQ-050/AC-054, and REQ-052/AC-056 are
approved with these release-specific refinements.

- **N60-1 — target-native installed proof.** A supported target is not proven
  by a source checkout, a local editable install, or an uploaded artifact. It
  needs a fresh-environment lifecycle from the public source selected for that
  target.
- **R60-1 — retry-safe completion.** Immutable existing bytes at an exact
  version are success/no-op only after identity and digest validation; an
  absence permits publish; query uncertainty fails closed. A release is not
  complete until every required target has its installed proof.
- **R60-2 — Tegra selection.** Classic Jetson only installs
  `fathomdb==0.8.24+tegra` from the sole authorized Pages index. No floating
  requirement and no `--extra-index-url` can select the smoke artifact.
- **AC60-1 — public-index smoke.** The smoke rejects malformed version, index,
  or SHA-256 input before contacting the network; downloads only the exact
  wheel from the declared index with cache disabled; verifies its SHA-256;
  installs it into a new venv without dependencies or source fallback; then
  passes open/write/search/close/process exit with a forced-CUDA in-process
  witness.
- **AC60-2 — CPU preservation.** Existing idempotent publisher and smoke
  fixtures continue to cover CPU publication paths, including inherited Windows
  CPU artifacts. This is preservation, not new Windows capability work.

These refinements change neither the accepted package identity nor the Windows
deferral. They correct release evidence semantics so `REQ-050` and `REQ-052`
can be satisfied by observable, retry-safe behavior.
