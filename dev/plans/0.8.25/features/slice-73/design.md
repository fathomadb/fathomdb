---
title: 0.8.25 Slice 73 — draft design
status: DRAFT
design_version: 1
target_release: 0.8.25
depends_on: 72
---

# Slice 73 draft design

The implementation design must inventory the current Windows native-artifact
path, name a fixed retained-module allowlist, define isolated packed-package
resolution proofs, and bind workflow structural tests to every required step.
Its receipt records exact source SHA, workflow run/job, runner image/toolchain,
artifact hashes, commands, module results, and package-resolution paths. This
is a focused feature-coverage job; the final unmerged exact-head release CI
gate remains Slice 75 work.
