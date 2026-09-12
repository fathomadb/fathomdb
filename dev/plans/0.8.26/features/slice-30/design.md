---
title: FathomDB 0.8.26 Slice 30 — operator integrity distribution design
status: DRAFT
---

# Slice 30 design — operator integrity distribution

## Recommended approach

First qualify the already published crates.io Rust CLI operation
`fathomdb doctor data-plane-integrity --json <db-path>` as the operator
boundary. Require exact CLI artifact identity and database-schema
compatibility. Add a new/prebuilt artifact only if Slice 8 confirms that Memex
cannot deploy the existing route. Do not mirror the doctor surface into Python
or TypeScript unless distribution is proven insufficient and a new HITL
decision authorizes the larger authority surface.

## Contract

- The tool and database format must be compatible with the owning FathomDB
  release; mismatch refuses before inspection.
- Invocation occurs under a documented quiescent/locking precondition selected
  in Slice 8. The tool must not claim a coherent cutover proof while writers
  can invalidate its boundary.
- JSON carries a stable schema version, check set, boundary, bounded findings,
  truncation state, status, and privacy-safe locators.
- Exit codes distinguish clean, findings present, invalid invocation/version,
  and inspection failure.
- The operation opens the real database read-only in behavior and produces no
  repair, rebuild, lifecycle, projection, or mutation side effect.

## Distribution choices for Slice 8

1. Use the existing exact-version crates.io CLI if it fits deployment.
2. If it does not, add the smallest named prebuilt artifact for the selected
   target rather than assuming a full new matrix.
3. Limit 0.8.26 evidence to Memex's named cutover platforms or require the full
   Linux x86_64/aarch64, macOS x86_64/arm64, and Windows x86_64 matrix.

The recommendation is the existing separately invocable CLI unless concrete
deployment evidence requires a smaller prebuilt form.
