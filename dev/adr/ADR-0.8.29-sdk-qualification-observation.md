---
title: ADR-0.8.29-sdk-qualification-observation
date: 2026-11-13
target_release: 0.8.29
desc: Classify one optional passive SDK qualification observation without expanding governed operations
blast_radius: Rust SDK diagnostic feature contract and parity checker
status: accepted
---

# Passive SDK qualification observation

This narrow addendum follows the approved Slice30 G observer/SDK-forwarding
scope and its independent Astra policy-fit PASS. No new owner ruling is needed.
[ADR-0.8.27-rust-sdk-parity](ADR-0.8.27-rust-sdk-parity.md) decision4 remains
verbatim: custom providers, operator/recovery, raw SQL, test hooks and Rust-only
search overloads stay excluded from the application SDK.

Passive qualification observation is distinct from an excluded test hook: it
cannot inject business state/providers/SQL, pause/fault operations, expose the
core Engine or change ordinary results/defaults. It records the already reviewed
bounded diagnostic intervals and numeric facts. Collection adds diagnostic cost;
its inclusive host intervals are not exclusive GPU execution times.

The optional surface is closed to exactly SDK
`Engine::begin_slice30_observation(max_records: usize)`, returning
`Result<fathomdb_engine::Slice30ObservationGuard>` through exactly
`Ok(self.core.begin_slice30_observation(max_records)?)`. It requires direct
`#[cfg(feature = "slice30-observer")]` and `#[doc(hidden)]`. The coordinated
engine/SDK name replaces the misleading `_for_test` spelling without an alias.
The existing guard/session/ownership/error contract in the
[Rust interface](../interfaces/rust.md#private-slice30-qualification-observations)
is unchanged.

The nondefault SDK feature forwards only `fathomdb-engine/slice30-observer`.
No other SDK feature may activate this diagnostic capability; SDK test-hooks or
operator forwarding remains forbidden. Shipped binding/facade/CLI/MCP feature
lists do not enable it. No runtime configuration knob is introduced.

A closed diagnostic catalog, separate from signed operations and ordinary
non-command members, validates the exact declaration, forwarding and feature
graph before classification. Missing/wrong/broadened gates, missing hidden
attributes, incorrect forwarding, default exposure and unknown members (even
similarly gated ones) fail closed. Existing core-leak checks and all53 governed
operations/signed mappings remain unchanged. Feature-positive/default-negative
compiled consumers remain independent checks. This addendum does not authorize
model/GPU execution, measurements, publication or whole-slice clearance.

## Slice130 D addendum

Accepted after independent Astra review of the material seam at906667b72.
This narrow extension adds one catalog entry; the original Slice30 contract
above remains unchanged. The reviewed [D seam design](../plans/0.8.29/features/slice-130/design.md#d-observational-seam-proposal)
and [exact binding-support contract](../interfaces/rust.md#private-cuda-pool-observation)
define its passive scope.
The additional catalog entry is static SDK
`Engine::cuda_pool_observation_for_qualification() -> Result<String>`, with the
exact forwarding/gates specified there, and dedicated `cuda-pool-observer`
forwarding only `fathomdb-engine/cuda-pool-observer`. Existing Slice30 entry,
no-other-feature reachability, no core-engine leak and no test-hooks/operator
forwarding remain unchanged. Native-only Python/Node transport exposes the same
passive JSON observation without changing governed operations or application
wrappers. No native generation, allocator authority, default feature or shipped
artifact support is introduced. Checked current counters require110's actual
shared tracked token; absent adoption returns named unavailable/untracked data.
