---
title: Slice 135 exact-candidate installed TypeScript qualification
status: AUDITED_FUNCTIONAL_NOT_LATENCY
target_release: 0.8.27
---

# Exact-candidate installed TypeScript qualification — 2026-10-08

The Linux x64 GNU N-API addon was built from clean source
`224e44c593c13d86ece648adabe445723db04070` with Node v25.9.0 and the
repository's release build. The [build identity](build-identity.json) binds the
source tree, Cargo lock, native binary, two packed npm archives, installed
JavaScript module and raw exercise. The native SHA-256 is
`5073e7ecc7dfef733da97cc5ff92be0f128f61741e1bc6ad05093d9d913b5aa2`.
The package version string remains 0.8.26 and is not candidate identity.

The installed consumer ran 43 real-database cases and partitioned all 44
governed operations: **41 selected positive cases executed, zero failed,
zero supported gaps, three provider/model cases unavailable**. The
[live audit](live-audit.json) checked installed bytes, source routes and
reopened state. The [retained-archive audit](retained-audit.json) independently
checked the same partition against copied archive bytes and rejected four
tampering controls. Selected positive calls do not exercise every filter,
error, cancellation, concurrency or platform condition.

The same installed artifact passed separate 32-row and 256-row S01 smokes
with 100 warm samples per query cell, and a three-sequence S02 smoke with
reopened-state checks. Their raw hashes are in the build identity. These
smokes establish executable fixtures, not comparative latency. The two
[exact-candidate S01](../../s01-ts-224e-comparison-protocol.json) and
[S02](../../s02-ts-224e-comparison-protocol.json) protocols freeze unchanged
workloads and baseline pilots before paired candidate timing.

The 8 MiB npm archives, adapted tests and raw case record are local and
untracked pending end-of-phase retention. The tracked [SHA-256 manifest](SHA256SUMS)
names their copied files here; tracked summaries alone do not publish the raw
evidence.
