---
title: FathomDB 0.8.27 hidden-surface successor f327c28c — diff record
status: REVIEW_PENDING
target_release: 0.8.27
previous_baseline: baseline-01f4063f.json
capture_source: f327c28caa5f20c3342127fbc5fde3c6ea189c7b
---

# Successor baseline `f327c28c`

## Identity and scope

The [successor manifest](baseline-f327c28c.json) binds final reviewed source
`f327c28caa5f20c3342127fbc5fde3c6ea189c7b`, including the completed Slice 117
handoff, default pinned-proof corrections, publication-facing 0.8.27 docs and
generated library/contributor references. Later receipt commits change
internal documentation only. The capture used the unchanged hidden-surface
tool, pinned `nightly-2026-04-24` (`rustc 1.97.0-nightly (36ba2c771 2026-04-23)`),
`x86_64-unknown-linux-gnu`, and required CUDA inventory access. All 37 rows
completed; source-modified is false. No historical baseline was overwritten.

| Evidence | SHA-256 |
| --- | --- |
| [Previous approved successor](baseline-01f4063f.json) | `0e041e0cb3951b70d831acaebd3d626fae9555fabb606ef88effb2f58d0362b1` |
| Successor capture | `e42c87f0684ee45e3cc5e51c30f82592999654ed62087cc213260595cd5b7109` |
| [Immediate diff](baseline-f327c28c-immediate-diff.json) | `20ddda61597369dee935be1f5008bae806d678319746b91da61197ec8fd1b405` |

## Exact immediate inventory delta

The unchanged comparator exits 1 for the intended test-inventory changes.
Metadata, toolchain, target, schema and all row identities are equal. Every
engine/facade Rust API row and the release-probe row is exactly equal to the
previous approved successor. There are no additional hidden API additions,
removals, shape changes or cfg changes.

All **295** test targets remain inventoried. Exactly two target predicates
change: `slice20_dense_readiness` and `slice25_registration_identity_inert`
return from `required-features = ["test-hooks"]` to the default route. Their
pinned proofs now exercise production caller/no-embedder open and governed
configuration. Six fixture-only Slice 20 sibling cases retain individual
explicit-hook gates. Default coverage is restored without exposing fixtures.

The aggregate `(crate, path, run/ignored)` test union remains **1,924**.
Exactly one qualified proof moves, with the same name, raw-database oracle
and `run` status:

- from `slice25_registration_identity_inert::the_internal_structural_row_writer_mints_no_logical_id`;
- to `lib::write::the_internal_structural_row_writer_mints_no_logical_id`.

The structural writer proof is a top-level engine unit test. C1 changes only
its evidence locator; obligation, category and contract hash are unchanged.
Focused anti-evaporation mutants cover its source/default execution route.
There are zero newly ignored tests and zero lost proof identities after that
explicit relocation. The full matrix has no `test-build` failure entries.

Seven non-hook engine test rows gain ten default proofs: the relocated unit,
two Slice 20 pinned proofs and seven Slice 25 tests. Each of eight hook-enabled
rows retains its count and changes only the qualified structural-proof path.
The engine default inventory is 444 (previously 434), explicit hooks 1,293,
and operator hooks 1,449. The exact JSON lists every row and path.

## Prior reviewed surface decisions

The [previous approved diff](baseline-01f4063f-diff.md) remains the record of
75 primary fixture gates, 101 derived non-hook removals, retained explicit
hook shapes and separate inherited changes. The immediate public comparison
still records only the documented 31 reader-guard-derived removals per engine
default/operator row. The visible graph-statistics exception is preserved.
This successor adds no new public/hidden Rust API delta.

The earlier `b3416577f` capture is diagnostic only. Its rows and comparable
metadata match this final capture exactly, but it was not promoted because
it preceded source-link/package-root review corrections. Source provenance
here names the final reviewed commit.

## Review and verification boundary

Independent `gpt-6-sol` high final-source code review passed on `f327c28ca`,
with no P0/P1/P2. Independent review of this exact capture/diff remains
required before promotion; source review alone is not artifact approval.
Focused default/explicit-hook proofs, ownership mutations, generated-doc
freshness, Pages pipeline/currency fixtures and scoped docs checks passed as
recorded in [Slice 140 status](../slice-140/status.md). The final-source
large-file register still covers 39 paths with no human approval gate.
Terra, three independent final reviews, package examples and the integrated
full gate remain parent-owned closeout requirements. This inventory is not a
publication receipt or Slice 150 final release qualification.
