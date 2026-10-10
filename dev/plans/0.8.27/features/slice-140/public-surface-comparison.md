---
title: FathomDB 0.8.27 Slice 140 — public surface comparison
status: REVIEWED_INTENDED_DELTA
target_release: 0.8.27
source_before: 1476d83e06b45feefbadabd995eb265f2c9217ea
source_after: 18ef03da9a5ee5769f1ac49083bd1ab20902c375
---

# Public surface comparison

The unchanged Slice 30 tool captured all 13 rows from clean pre-Slice 140
`1476d83e0` and integrated `18ef03da9` checkouts with pinned Node `v25.9.0`.
Metadata compares equal. The [exact immediate diff](public-surface-immediate-diff.json)
exits 1: `rust-engine-default` and `rust-engine-operator` each remove 31
entries belonging to the `ReaderSearchPauseForTest` fixture type,
its explicit methods, and derived/blanket impl entries. Every other row is
equal; there are no additions or changed entries. This is an intended fixture
removal, not an equality pass. The Removed changelog and Rust interface name
the withdrawn default route and retained unit/explicit-feature route.
The external default dev/release controls reject the guard at its own source
line, while explicit-feature controls compile it in both profiles.

The independent `gpt-6-sol` high reviewer accepted the exact 31-entry
removal in each engine row and required the explicit Rust interface route
wording; that wording is updated. The guard/root re-export lacked
`#[doc(hidden)]`, so the changelog uses the broader engine-test-fixtures
description. The immediate
comparison gives the complete derived path/signature removal list for review.
The later `01f4063f7` source-comment cleanup changes no API or gate.

## Capture identity

| Evidence | SHA-256 |
| --- | --- |
| Before capture | `5e57ce07a3f9303361f6f59f126182eda435a47cb0680f9ef6b48c395bf07059` |
| After capture | `2ff0d0573c3ba202060083fc9b9829a6c2b4667544c10c7869c37d266bd60d70` |
| Immediate diff | `0046c7d2944c3f196fde674b0e19f0cef687c240275fe535fd36fc42a2866661` |

The full before/after captures are local verification receipts under `/tmp`,
not public download links. The tracked diff and this summary preserve the
complete Slice 140 public delta.

## Immutable baseline comparison

The original `slice-30/baseline.json` is retained unchanged. Its comparison
with the after capture also exits 1 with equal metadata. It includes inherited
changes from earlier slices: NAPI/TypeScript subscriber typing and removal of
`AttachSubscriberOptions`, Python open/search/subscriber signatures and
`DependencyTraceError`, and Rust configuration/diagnostic additions. These are
not Slice 140 additions: the exact immediate comparison above isolates this
slice. Changed row counts against the immutable baseline are:

| Row | Added | Removed | Changed |
| --- | ---: | ---: | ---: |
| `napi-production` | 1 | 2 | 1 |
| `python-native-stub` | 3 | 3 | 3 |
| `python-wrapper-declarations` | 5 | 4 | 4 |
| `rust-engine-default` | 107 | 0 | 0 |
| `rust-engine-operator` | 203 | 0 | 0 |
| `rust-engine-operator-test-hooks` | 279 | 0 | 0 |
| `rust-engine-test-hooks` | 183 | 0 | 0 |
| `rust-facade-default` | 2 | 0 | 0 |
| `rust-facade-operator` | 5 | 0 | 0 |
| `typescript-declarations` | 3 | 4 | 2 |
