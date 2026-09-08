---
title: 0.8.25 Slice 72 — draft design
status: DRAFT
design_version: 1
target_release: 0.8.25
depends_on: 71
---

# Slice 72 draft design

The implementation design must define a release-neutral state schema and Git-
verified dependency algorithm before changing `scripts/preflight.sh`. It must
also define strict CE manifest/receipt schemas, feature-present artifact
construction, source-independence checks, semantic canary, fixed comparison
fixture, numerical-equivalence tolerance, cold/steady timing boundaries, and
CUDA allocation proof. Diagnostic or incomplete cells may be retained but
cannot pass. Verification is focused to these changed surfaces; Slice 75 owns
the full release matrix.
