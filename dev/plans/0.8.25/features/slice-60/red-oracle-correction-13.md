---
title: Slice 60 verification FIX-6 RED oracle correction 13
---

# Slice 60 verification FIX-6 RED oracle correction 13

AC-050a bans the public prefix `legacy_`. The Cycle 3 projection-state test
hook is public under `test-hooks`, so its caller must move to the audited exact
name `projection_legacy_unverified_degraded`. This correction changes only the
frozen caller before the production definition exists; it retains the exact
legacy-unverified/degraded tuple and every assertion unchanged. No alias is
permitted, because it would retain the forbidden public symbol.

| Path | Old SHA-256 | New SHA-256 |
| --- | --- | --- |
| `src/rust/crates/fathomdb-engine/tests/slice60_fix3_projection_states.rs` | `6e57f78e7da8591e3b01d3c37c638fa5e9dcdb1f12bd13e89cd86666352f3ffd` | `861d90d89fb3d1debd2d1ca864f6412fb80e1115b0eb0608e9b4c2b82883aee3` |

The intended RED compile fails with E0599 until the production definition is
renamed to the exact audited symbol.

Implementation-review cycle 7 found that the initially recorded old hash was
truncated to 61 hexadecimal characters. The value above is the corrected exact
SHA-256 obtained directly from the pre-correction Git object at `49b048c3`.
