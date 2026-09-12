---
title: FathomDB 0.8.26 Slice 30 — operator integrity distribution design
status: DRAFT
---

# Slice 30 design — operator integrity distribution

## Recommended approach

Use the existing Rust CLI operation
`fathomdb doctor data-plane-integrity --json <db-path>` as the operator
boundary. Package that executable, or a narrowly scoped artifact containing
it, alongside version metadata. Do not mirror the doctor surface into Python
or TypeScript unless artifact distribution is proven insufficient and a new
HITL decision authorizes the larger authority surface.

## Contract

- The tool and database format must be compatible with the owning FathomDB
  release; mismatch refuses before inspection.
- Invocation occurs under a documented quiescent/locking precondition selected
  in Slice 6. The tool must not claim a coherent cutover proof while writers
  can invalidate its boundary.
- JSON carries a stable schema version, check set, boundary, bounded findings,
  truncation state, status, and privacy-safe locators.
- Exit codes distinguish clean, findings present, invalid invocation/version,
  and inspection failure.
- The operation opens the real database read-only in behavior and produces no
  repair, rebuild, lifecycle, projection, or mutation side effect.

## Distribution choices for Slice 6

1. Add the CLI binary to existing native release artifacts.
2. Publish a separate version-locked operator artifact.
3. Limit 0.8.26 evidence to Memex's named cutover platforms or require the full
   Linux x86_64/aarch64, macOS x86_64/arm64, and Windows x86_64 matrix.

The recommendation is the smallest separately invocable version-locked
artifact that reuses the current CLI and engine implementation.
