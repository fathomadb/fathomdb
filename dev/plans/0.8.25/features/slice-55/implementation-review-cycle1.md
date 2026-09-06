---
title: 0.8.25 Slice 55 implementation review cycle 1
status: FAIL
candidate: f5676855522badb24c8e569828da3c8315bbdec8
---

# Slice 55 implementation review cycle 1

## Verdict

**FAIL.** The independent review found six blocking P1 findings, two blocking P2
findings, and one P3 follow-up. A second review is required after FIX-1; the
review cycle remains within the plan's maximum of seven.

## Findings

1. **P1 — integrity implementation is a stub.** Implement all four
   authoritative matrices, indexed cap-plus-one accounting, guarded minimal
   receipt parsing, findings cap/order, shared Slice 40 classification, exact
   CLI errors, and real corruption/property tests. The current implementation
   in `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs` counts rows
   but always returns an empty findings vector.
2. **P1 — trace authority and nondisclosure are incomplete.** Validate the full
   normalized provenance/dependency chain and closure fences in the
   authenticated transaction, including counterpart roles. Unverifiable or
   hidden corruption must collapse to absence. Add production query-plan hooks
   and the complete real-database matrix in
   `src/rust/crates/fathomdb-engine/src/dependency_trace.rs` and its tests.
3. **P1 — the performance witness is not real.** Build the required 50,000
   hidden-dependent fixture outside the measured window; prove byte-equality,
   absence, and no hidden-row-induced bound error. The progress handler must
   interrupt above 10,000,000 VM steps and the test must capture exact RSS/VM
   evidence.
4. **P1 — codecs and SDK request validation are incomplete.** Implement strict
   recursive encoding/decoding, including nonempty dependency edges, exact
   paths/ranges, canonical integers, closed unions, and declaration order. Add
   malformed matrices and schema-first request validation in Python and
   TypeScript.
5. **P1 — Python exports the wrong exception class.** Export or alias the actual
   native `DependencyTraceError` with stable attributes. Installed tests must
   catch actual native malformed, unavailable, bound, and corruption refusals,
   rather than a separate pure-Python class.
6. **P1 — structural explanation is not authoritative.** Classify real
   lifecycle, projection readiness/degradation, graph-bound state, and complete
   dependency state from the same search snapshot. Exercise every state through
   live behavior, genuine race/concurrency tests, and cross-SDK candidate-native
   tests.
7. **P2 — operator boundary leaks into default builds.** Gate
   `DataPlaneIntegrityRequestV1` and its related surface under `operator`; add a
   default-build absence oracle through the established compile-surface
   mechanism.
8. **P2 — durable package evidence is missing.** Check in the plan-required
   installed smoke and package-smoke extensions. They must exercise real
   candidate-native responses and refusals, not `SimpleNamespace` or fake-object
   claims.
9. **P3 — dependency scope should be tightened.** Prefer gating `rusqlite`
   hooks and `libc` to test-only measurement paths while fixing the performance
   witness. Record the final disposition if doing so is impractical.

## Required FIX-1 disposition

Every P1/P2 correction requires a failing test committed before its production
change. Existing tests remain read-only except for additive strengthening; no
oracle may be weakened or regenerated. Final evidence must identify the exact
RED and GREEN commits, focused commands, diagnostics, package provenance, and
any P3 disposition before review cycle 2.
