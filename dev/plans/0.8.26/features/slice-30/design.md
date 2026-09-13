---
title: FathomDB 0.8.26 Slice 30 — operator integrity distribution design
status: DRAFT
---

# Slice 30 design — operator integrity distribution

## Recommended approach

First qualify the already published crates.io Rust CLI operation
`fathomdb doctor data-plane-integrity --json <db-path>` as the operator
boundary. Require exact CLI artifact identity and database-schema
compatibility. If Slice 30 proves that Memex cannot deploy the existing route,
it stops and returns the evidence to HITL. HITL selected this sequence at
`seq-280`: qualification failure must warn loudly before any prebuilt scope is
designed or implemented. Do not mirror the doctor surface into Python
or TypeScript unless distribution is proven insufficient and a new HITL
decision authorizes the larger authority surface.

## Contract

- The tool and database format must be compatible with the owning FathomDB
  release; mismatch refuses before inspection.
- Slice 30 specifies and validates the invocation's quiescent/locking
  precondition. The tool must not claim a coherent cutover proof while writers
  can invalidate its boundary.
- JSON carries a stable schema version, check set, boundary, bounded findings,
  truncation state, status, and privacy-safe locators.
- Exit codes distinguish clean, findings present, invalid invocation/version,
  and inspection failure.
- The operation opens the real database read-only in behavior and produces no
  repair, rebuild, lifecycle, projection, or mutation side effect.

## Distribution decision

1. Use the existing exact-version crates.io CLI if it fits deployment.
2. If it does not, stop and present a loud, evidence-backed warning to HITL.
   A later explicit ruling may authorize the smallest named prebuilt artifact
   for the selected target rather than a full new matrix.
3. Limit 0.8.26 evidence to Memex's named cutover platforms or require the full
   Linux x86_64/aarch64, macOS x86_64/arm64, and Windows x86_64 matrix.

The authorized path is the existing separately invocable CLI. Concrete
deployment evidence may trigger a prebuilt proposal, but cannot itself
authorize that fallback.
