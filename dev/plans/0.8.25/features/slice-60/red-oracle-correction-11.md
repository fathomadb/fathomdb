# Slice 60 RED-oracle correction 11 — contract-valid closure witness

Cycle 5 found that the prior FIX-4 closure witness depended on a test hook
that wrote a cross-owner derived provenance chain. That mechanics defect made
the witness depend on the persisted source-ID equality checks that the same
review requires to be restored.

The corrected fixture retains its real `Proving` barrier, surviving-row and
graph-exclusion assertions, `NotRegistered` to `Registered` transition, and
separate `erase_source`/`excise_source` controls. Its test-hooks real-SQL setup
now creates a same-owner derived row, derived revision, and source link copied
from the canonical source, so the setup is valid when the production
fail-closed checks are restored. The correction also removes only imports and
the digest helper made unused by the earlier audited fixture migration.

| Path | Old SHA-256 | New SHA-256 |
| --- | --- | --- |
| `src/rust/crates/fathomdb-engine/tests/slice60_fix4_dependency.rs` | `87f45913bbd05aec5c0667e96c9e9f4c95d982e6979d948579308c5839f563a9` | `ce783ae67a843e4af33308d9787523280b2b690f8697799f8bb737806c5e22af` |
